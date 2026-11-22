// Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
//! Bounded SCF1 persistence for the native SoundCraft session and MIDI subset.

use core::fmt;

use crate::midi::{CtrlEvent, CtrlKind, MAX_NATIVE_EVENTS, MidiError, Note, Sequence};
use crate::session::{MAX_TRACKS, Session, SessionError, TrackId, TrackKind};

const MAGIC: &[u8; 4] = b"SCF1";
const VERSION: u16 = 1;
const HEADER_BYTES: usize = 32;
const MAX_NAME_BYTES: usize = 1_024;
pub const MAX_FILE_BYTES: usize = 192 * 1_024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectError {
    BufferTooSmall,
    FileTooLarge,
    InvalidMagic,
    UnsupportedVersion,
    InvalidHeader,
    InvalidUtf8,
    InvalidTrack,
    InvalidMidiTrack,
    TrackLimit,
    EventLimit,
    InvalidEvent,
    TrailingData,
    OutOfMemory,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::BufferTooSmall => "SCF1 output buffer is too small",
            Self::FileTooLarge => "SCF1 file exceeds the native size limit",
            Self::InvalidMagic => "SCF1 magic is invalid",
            Self::UnsupportedVersion => "SCF1 version is unsupported",
            Self::InvalidHeader => "SCF1 header is invalid or truncated",
            Self::InvalidUtf8 => "SCF1 name is not valid UTF-8",
            Self::InvalidTrack => "SCF1 track is invalid",
            Self::InvalidMidiTrack => "SCF1 selected MIDI track is invalid",
            Self::TrackLimit => "SCF1 track count exceeds the native limit",
            Self::EventLimit => "SCF1 event count exceeds the native limit",
            Self::InvalidEvent => "SCF1 MIDI event is invalid",
            Self::TrailingData => "SCF1 contains trailing data",
            Self::OutOfMemory => "not enough memory for the SCF1 project",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub session: Session,
    pub sequence: Sequence,
    pub midi_track: TrackId,
}

impl Project {
    pub fn encode(&self, output: &mut [u8]) -> Result<usize, ProjectError> {
        encode_project(&self.session, &self.sequence, self.midi_track, output)
    }

    pub fn decode(input: &[u8]) -> Result<Self, ProjectError> {
        if input.len() > MAX_FILE_BYTES {
            return Err(ProjectError::FileTooLarge);
        }
        if input.len() < MAGIC.len() {
            return Err(ProjectError::InvalidHeader);
        }
        if input.get(..MAGIC.len()) != Some(MAGIC.as_slice()) {
            return Err(ProjectError::InvalidMagic);
        }
        if input.len() < HEADER_BYTES {
            return Err(ProjectError::InvalidHeader);
        }
        let mut reader = Reader::new(input);
        let _ = reader.bytes(MAGIC.len())?;
        if reader.u16()? != VERSION {
            return Err(ProjectError::UnsupportedVersion);
        }
        if reader.u16()? != 0 {
            return Err(ProjectError::InvalidHeader);
        }
        let sample_rate = reader.u32()?;
        let session_name_len = usize::from(reader.u16()?);
        let track_count = usize::from(reader.u16()?);
        let note_count = usize::try_from(reader.u32()?).map_err(|_| ProjectError::EventLimit)?;
        let controller_count = usize::try_from(reader.u32()?).map_err(|_| ProjectError::EventLimit)?;
        let midi_track = TrackId(reader.u64()?);
        if session_name_len > MAX_NAME_BYTES {
            return Err(ProjectError::InvalidHeader);
        }
        if track_count > MAX_TRACKS {
            return Err(ProjectError::TrackLimit);
        }
        let event_count = note_count.checked_add(controller_count).ok_or(ProjectError::EventLimit)?;
        if event_count > MAX_NATIVE_EVENTS {
            return Err(ProjectError::EventLimit);
        }
        let session_name = decode_name(reader.bytes(session_name_len)?)?;
        let mut session = Session::new(session_name, sample_rate).map_err(map_session_error)?;
        for _ in 0..track_count {
            let id = TrackId(reader.u64()?);
            let kind = track_kind_from_u8(reader.u8()?)?;
            let flags = reader.u8()?;
            if flags & !0x07 != 0 {
                return Err(ProjectError::InvalidTrack);
            }
            let name_len = usize::from(reader.u16()?);
            if name_len > MAX_NAME_BYTES {
                return Err(ProjectError::InvalidTrack);
            }
            let name = decode_name(reader.bytes(name_len)?)?;
            session.restore_track(id, name, kind, flags & 1 != 0, flags & 2 != 0, flags & 4 != 0).map_err(map_session_error)?;
        }
        let selected = session.track(midi_track).ok_or(ProjectError::InvalidMidiTrack)?;
        if !selected.kind().is_midi() {
            return Err(ProjectError::InvalidMidiTrack);
        }

        let mut sequence = Sequence::with_event_limit(MAX_NATIVE_EVENTS);
        for _ in 0..note_count {
            sequence
                .add_note(Note {
                    pitch: reader.u8()?,
                    velocity: reader.u8()?,
                    release_velocity: reader.u8()?,
                    channel: reader.u8()?,
                    start: reader.i64()?,
                    length: reader.i64()?,
                })
                .map_err(map_midi_error)?;
        }
        for _ in 0..controller_count {
            let tick = reader.i64()?;
            let channel = reader.u8()?;
            let kind_tag = reader.u8()?;
            let kind_data = reader.u8()?;
            if reader.u8()? != 0 {
                return Err(ProjectError::InvalidEvent);
            }
            let value = reader.i32()?;
            sequence.add_controller(CtrlEvent { tick, channel, kind: controller_from_bytes(kind_tag, kind_data)?, value }).map_err(map_midi_error)?;
        }
        if !reader.is_finished() {
            return Err(ProjectError::TrailingData);
        }
        Ok(Self { session, sequence, midi_track })
    }
}

pub fn encode_project(session: &Session, sequence: &Sequence, selected_midi_track: TrackId, output: &mut [u8]) -> Result<usize, ProjectError> {
    let track_count = u16::try_from(session.track_count()).map_err(|_| ProjectError::TrackLimit)?;
    let note_count = u32::try_from(sequence.notes().len()).map_err(|_| ProjectError::EventLimit)?;
    let controller_count = u32::try_from(sequence.controllers().len()).map_err(|_| ProjectError::EventLimit)?;
    if sequence.event_count() > MAX_NATIVE_EVENTS {
        return Err(ProjectError::EventLimit);
    }
    let midi_track = session.track(selected_midi_track).ok_or(ProjectError::InvalidMidiTrack)?;
    if !midi_track.kind().is_midi() {
        return Err(ProjectError::InvalidMidiTrack);
    }
    let session_name = session.name().as_bytes();
    let session_name_len = bounded_name_len(session_name)?;

    let mut writer = Writer::new(output);
    writer.bytes(MAGIC)?;
    writer.u16(VERSION)?;
    writer.u16(0)?;
    writer.u32(session.sample_rate_hz())?;
    writer.u16(session_name_len)?;
    writer.u16(track_count)?;
    writer.u32(note_count)?;
    writer.u32(controller_count)?;
    writer.u64(selected_midi_track.0)?;
    writer.bytes(session_name)?;

    for track in session.tracks() {
        let name = track.name().as_bytes();
        writer.u64(track.id().0)?;
        writer.u8(track_kind_to_u8(track.kind()))?;
        let flags = u8::from(track.is_muted()) | (u8::from(track.is_solo()) << 1) | (u8::from(track.is_record_armed()) << 2);
        writer.u8(flags)?;
        writer.u16(bounded_name_len(name)?)?;
        writer.bytes(name)?;
    }
    for note in sequence.notes() {
        writer.u8(note.pitch)?;
        writer.u8(note.velocity)?;
        writer.u8(note.release_velocity)?;
        writer.u8(note.channel)?;
        writer.i64(note.start)?;
        writer.i64(note.length)?;
    }
    for event in sequence.controllers() {
        let (kind, data) = controller_to_bytes(event.kind);
        writer.i64(event.tick)?;
        writer.u8(event.channel)?;
        writer.u8(kind)?;
        writer.u8(data)?;
        writer.u8(0)?;
        writer.i32(event.value)?;
    }
    Ok(writer.position())
}

fn bounded_name_len(name: &[u8]) -> Result<u16, ProjectError> {
    if name.len() > MAX_NAME_BYTES {
        return Err(ProjectError::InvalidTrack);
    }
    u16::try_from(name.len()).map_err(|_| ProjectError::InvalidTrack)
}

fn decode_name(bytes: &[u8]) -> Result<&str, ProjectError> {
    core::str::from_utf8(bytes).map_err(|_| ProjectError::InvalidUtf8)
}

fn map_session_error(error: SessionError) -> ProjectError {
    match error {
        SessionError::TrackLimit => ProjectError::TrackLimit,
        SessionError::OutOfMemory => ProjectError::OutOfMemory,
        _ => ProjectError::InvalidTrack,
    }
}

fn map_midi_error(error: MidiError) -> ProjectError {
    match error {
        MidiError::EventLimit => ProjectError::EventLimit,
        MidiError::OutOfMemory => ProjectError::OutOfMemory,
        _ => ProjectError::InvalidEvent,
    }
}

const fn track_kind_to_u8(kind: TrackKind) -> u8 {
    match kind {
        TrackKind::Audio => 0,
        TrackKind::Aux => 1,
        TrackKind::Master => 2,
        TrackKind::Midi => 3,
        TrackKind::Instrument => 4,
        TrackKind::Vca => 5,
        TrackKind::Folder => 6,
        TrackKind::Video => 7,
    }
}

fn track_kind_from_u8(value: u8) -> Result<TrackKind, ProjectError> {
    match value {
        0 => Ok(TrackKind::Audio),
        1 => Ok(TrackKind::Aux),
        2 => Ok(TrackKind::Master),
        3 => Ok(TrackKind::Midi),
        4 => Ok(TrackKind::Instrument),
        5 => Ok(TrackKind::Vca),
        6 => Ok(TrackKind::Folder),
        7 => Ok(TrackKind::Video),
        _ => Err(ProjectError::InvalidTrack),
    }
}

const fn controller_to_bytes(kind: CtrlKind) -> (u8, u8) {
    match kind {
        CtrlKind::Cc(controller) => (0, controller),
        CtrlKind::PitchBend => (1, 0),
        CtrlKind::ChannelPressure => (2, 0),
        CtrlKind::PolyPressure(pitch) => (3, pitch),
        CtrlKind::Program => (4, 0),
    }
}

fn controller_from_bytes(kind: u8, data: u8) -> Result<CtrlKind, ProjectError> {
    match kind {
        0 => Ok(CtrlKind::Cc(data)),
        1 if data == 0 => Ok(CtrlKind::PitchBend),
        2 if data == 0 => Ok(CtrlKind::ChannelPressure),
        3 => Ok(CtrlKind::PolyPressure(data)),
        4 if data == 0 => Ok(CtrlKind::Program),
        _ => Err(ProjectError::InvalidEvent),
    }
}

struct Writer<'output> {
    output: &'output mut [u8],
    position: usize,
}

impl<'output> Writer<'output> {
    const fn new(output: &'output mut [u8]) -> Self {
        Self { output, position: 0 }
    }

    const fn position(&self) -> usize {
        self.position
    }

    fn bytes(&mut self, bytes: &[u8]) -> Result<(), ProjectError> {
        let end = self.position.checked_add(bytes.len()).ok_or(ProjectError::BufferTooSmall)?;
        let target = self.output.get_mut(self.position..end).ok_or(ProjectError::BufferTooSmall)?;
        target.copy_from_slice(bytes);
        self.position = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), ProjectError> {
        self.bytes(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), ProjectError> {
        self.bytes(&value.to_be_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), ProjectError> {
        self.bytes(&value.to_be_bytes())
    }

    fn i32(&mut self, value: i32) -> Result<(), ProjectError> {
        self.bytes(&value.to_be_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), ProjectError> {
        self.bytes(&value.to_be_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), ProjectError> {
        self.bytes(&value.to_be_bytes())
    }
}

struct Reader<'input> {
    input: &'input [u8],
    position: usize,
}

impl<'input> Reader<'input> {
    const fn new(input: &'input [u8]) -> Self {
        Self { input, position: 0 }
    }

    fn bytes(&mut self, length: usize) -> Result<&'input [u8], ProjectError> {
        let end = self.position.checked_add(length).ok_or(ProjectError::InvalidHeader)?;
        let bytes = self.input.get(self.position..end).ok_or(ProjectError::InvalidHeader)?;
        self.position = end;
        Ok(bytes)
    }

    fn u8(&mut self) -> Result<u8, ProjectError> {
        self.bytes(1)?.first().copied().ok_or(ProjectError::InvalidHeader)
    }

    fn u16(&mut self) -> Result<u16, ProjectError> {
        let bytes: [u8; 2] = self.bytes(2)?.try_into().map_err(|_| ProjectError::InvalidHeader)?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, ProjectError> {
        let bytes: [u8; 4] = self.bytes(4)?.try_into().map_err(|_| ProjectError::InvalidHeader)?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn i32(&mut self) -> Result<i32, ProjectError> {
        let bytes: [u8; 4] = self.bytes(4)?.try_into().map_err(|_| ProjectError::InvalidHeader)?;
        Ok(i32::from_be_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, ProjectError> {
        let bytes: [u8; 8] = self.bytes(8)?.try_into().map_err(|_| ProjectError::InvalidHeader)?;
        Ok(u64::from_be_bytes(bytes))
    }

    fn i64(&mut self) -> Result<i64, ProjectError> {
        let bytes: [u8; 8] = self.bytes(8)?.try_into().map_err(|_| ProjectError::InvalidHeader)?;
        Ok(i64::from_be_bytes(bytes))
    }

    fn is_finished(&self) -> bool {
        self.position == self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Result<Project, ProjectError> {
        let mut session = Session::new("Demo", 48_000).map_err(map_session_error)?;
        session.add_track("Audio 1", TrackKind::Audio).map_err(map_session_error)?;
        let midi_track = session.add_track("MIDI 1", TrackKind::Midi).map_err(map_session_error)?;
        session.set_mix_state(midi_track, false, true, true).map_err(map_session_error)?;
        let mut sequence = Sequence::with_event_limit(8);
        sequence.add_note(Note { pitch: 60, velocity: 100, release_velocity: 64, channel: 0, start: 0, length: 960 }).map_err(map_midi_error)?;
        sequence.add_controller(CtrlEvent { tick: 480, channel: 0, kind: CtrlKind::Cc(1), value: 72 }).map_err(map_midi_error)?;
        Ok(Project { session, sequence, midi_track })
    }

    #[test]
    fn scf1_round_trip_preserves_tracks_mix_and_midi() -> Result<(), ProjectError> {
        let original = project()?;
        let mut output = [0_u8; 4_096];
        let length = original.encode(&mut output)?;
        let decoded = Project::decode(&output[..length])?;
        assert_eq!(decoded.session, original.session);
        assert_eq!(decoded.sequence.notes(), original.sequence.notes());
        assert_eq!(decoded.sequence.controllers(), original.sequence.controllers());
        assert_eq!(decoded.midi_track, original.midi_track);
        assert_eq!(decoded.session.track(decoded.midi_track).map(|track| track.is_record_armed()), Some(true));
        Ok(())
    }

    #[test]
    fn hostile_headers_and_trailing_data_are_rejected() -> Result<(), ProjectError> {
        let original = project()?;
        let mut output = [0_u8; 4_096];
        let length = original.encode(&mut output)?;
        assert_eq!(Project::decode(&output[..3]), Err(ProjectError::InvalidHeader));
        output[0] = b'X';
        assert_eq!(Project::decode(&output[..length]), Err(ProjectError::InvalidMagic));
        output[0] = b'S';
        output[14..16].copy_from_slice(&257_u16.to_be_bytes());
        assert_eq!(Project::decode(&output[..length]), Err(ProjectError::TrackLimit));
        let length = original.encode(&mut output)?;
        output[length] = 0;
        assert_eq!(Project::decode(&output[..=length]), Err(ProjectError::TrailingData));
        Ok(())
    }

    #[test]
    fn short_output_is_reported_without_partial_success() -> Result<(), ProjectError> {
        let original = project()?;
        let mut output = [0_u8; 16];
        assert_eq!(original.encode(&mut output), Err(ProjectError::BufferTooSmall));
        Ok(())
    }
}
