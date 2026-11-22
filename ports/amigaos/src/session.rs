// Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
//! Bounded, allocation-aware subset of the SoundCraft session model.
//!
//! The desktop schema remains canonical.  This subset preserves the same sample-rate range,
//! track kinds, 256-character name limit and session-wide stable id semantics while the full
//! `craft-scraft-format` crate still requires `std`.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

pub const MAX_NAME_CHARS: usize = 256;
pub const MAX_TRACKS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TrackId(pub u64);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TrackKind {
    #[default]
    Audio,
    Aux,
    Master,
    Midi,
    Instrument,
    Vca,
    Folder,
    Video,
}

impl TrackKind {
    pub const ALL: [Self; 8] = [Self::Audio, Self::Aux, Self::Master, Self::Midi, Self::Instrument, Self::Vca, Self::Folder, Self::Video];

    pub const fn has_playlist(self) -> bool {
        matches!(self, Self::Audio | Self::Midi | Self::Instrument | Self::Video)
    }

    pub const fn is_midi(self) -> bool {
        matches!(self, Self::Midi | Self::Instrument)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Track {
    id: TrackId,
    name: String,
    kind: TrackKind,
    muted: bool,
    solo: bool,
    record_armed: bool,
}

impl Track {
    pub const fn id(&self) -> TrackId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn kind(&self) -> TrackKind {
        self.kind
    }

    pub const fn is_muted(&self) -> bool {
        self.muted
    }

    pub const fn is_solo(&self) -> bool {
        self.solo
    }

    pub const fn is_record_armed(&self) -> bool {
        self.record_armed
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionError {
    InvalidSampleRate,
    NameTooLong,
    NameContainsNul,
    TrackLimit,
    IdExhausted,
    MissingTrack,
    InvalidTrackId,
    DuplicateTrackId,
    OutOfMemory,
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSampleRate => "sample rate must be between 8000 and 768000 Hz",
            Self::NameTooLong => "name exceeds 256 characters",
            Self::NameContainsNul => "name contains a NUL character",
            Self::TrackLimit => "native session track limit reached",
            Self::IdExhausted => "session id space exhausted",
            Self::MissingTrack => "track does not exist",
            Self::InvalidTrackId => "track id must be non-zero",
            Self::DuplicateTrackId => "track id already exists",
            Self::OutOfMemory => "not enough memory for session data",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    name: String,
    sample_rate_hz: u32,
    tracks: Vec<Track>,
    next_id: u64,
}

impl Session {
    pub fn new(name: &str, sample_rate_hz: u32) -> Result<Self, SessionError> {
        if !(8_000..=768_000).contains(&sample_rate_hz) {
            return Err(SessionError::InvalidSampleRate);
        }
        Ok(Self { name: copy_name(name)?, sample_rate_hz, tracks: Vec::new(), next_id: 1 })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|track| track.id == id)
    }

    pub fn add_track(&mut self, name: &str, kind: TrackKind) -> Result<TrackId, SessionError> {
        if self.tracks.len() >= MAX_TRACKS {
            return Err(SessionError::TrackLimit);
        }
        let id = TrackId(self.next_id.max(1));
        let next_id = id.0.checked_add(1).ok_or(SessionError::IdExhausted)?;
        let name = copy_name(name)?;
        self.tracks.try_reserve_exact(1).map_err(|_| SessionError::OutOfMemory)?;
        self.tracks.push(Track { id, name, kind, muted: false, solo: false, record_armed: false });
        self.next_id = next_id;
        Ok(id)
    }

    pub fn restore_track(
        &mut self,
        id: TrackId,
        name: &str,
        kind: TrackKind,
        muted: bool,
        solo: bool,
        record_armed: bool,
    ) -> Result<(), SessionError> {
        if id.0 == 0 {
            return Err(SessionError::InvalidTrackId);
        }
        if self.tracks.len() >= MAX_TRACKS {
            return Err(SessionError::TrackLimit);
        }
        if self.track(id).is_some() {
            return Err(SessionError::DuplicateTrackId);
        }
        let next_id = id.0.checked_add(1).ok_or(SessionError::IdExhausted)?;
        let name = copy_name(name)?;
        self.tracks.try_reserve_exact(1).map_err(|_| SessionError::OutOfMemory)?;
        self.tracks.push(Track { id, name, kind, muted, solo, record_armed });
        self.next_id = self.next_id.max(next_id);
        Ok(())
    }

    pub fn rename_track(&mut self, id: TrackId, name: &str) -> Result<(), SessionError> {
        let Some(index) = self.tracks.iter().position(|track| track.id == id) else {
            return Err(SessionError::MissingTrack);
        };
        let name = copy_name(name)?;
        if let Some(track) = self.tracks.get_mut(index) {
            track.name = name;
            Ok(())
        } else {
            Err(SessionError::MissingTrack)
        }
    }

    pub fn set_mix_state(&mut self, id: TrackId, muted: bool, solo: bool, record_armed: bool) -> Result<(), SessionError> {
        let Some(track) = self.tracks.iter_mut().find(|track| track.id == id) else {
            return Err(SessionError::MissingTrack);
        };
        track.muted = muted;
        track.solo = solo;
        track.record_armed = record_armed;
        Ok(())
    }

    pub fn remove_track(&mut self, id: TrackId) -> Result<Track, SessionError> {
        let Some(index) = self.tracks.iter().position(|track| track.id == id) else {
            return Err(SessionError::MissingTrack);
        };
        Ok(self.tracks.remove(index))
    }
}

fn copy_name(value: &str) -> Result<String, SessionError> {
    if value.chars().any(|character| character == '\0') {
        return Err(SessionError::NameContainsNul);
    }
    if value.chars().count() > MAX_NAME_CHARS {
        return Err(SessionError::NameTooLong);
    }
    let mut copied = String::new();
    copied.try_reserve_exact(value.len()).map_err(|_| SessionError::OutOfMemory)?;
    copied.push_str(value);
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session() -> Result<Session, SessionError> {
        Session::new("Song", 48_000)
    }

    #[test]
    fn builds_and_edits_a_soundcraft_aligned_track_index() -> Result<(), SessionError> {
        let mut session = test_session()?;
        let audio = session.add_track("Audio 1", TrackKind::Audio);
        let midi = session.add_track("MIDI 1", TrackKind::Midi);
        assert_eq!(audio, Ok(TrackId(1)));
        assert_eq!(midi, Ok(TrackId(2)));
        assert_eq!(session.track_count(), 2);
        assert_eq!(session.name(), "Song");
        assert_eq!(session.sample_rate_hz(), 48_000);
        assert_eq!(TrackKind::ALL.len(), 8);

        assert_eq!(session.rename_track(TrackId(2), "Keys"), Ok(()));
        assert_eq!(session.set_mix_state(TrackId(2), false, true, true), Ok(()));
        let keys = session.track(TrackId(2));
        assert_eq!(keys.map(Track::name), Some("Keys"));
        assert_eq!(keys.map(Track::kind), Some(TrackKind::Midi));
        assert_eq!(keys.map(Track::is_solo), Some(true));
        assert_eq!(keys.map(Track::is_record_armed), Some(true));
        assert_eq!(keys.map(Track::is_muted), Some(false));
        Ok(())
    }

    #[test]
    fn ids_remain_stable_after_removing_a_track() -> Result<(), SessionError> {
        let mut session = test_session()?;
        assert_eq!(session.add_track("One", TrackKind::Audio), Ok(TrackId(1)));
        assert_eq!(session.add_track("Two", TrackKind::Audio), Ok(TrackId(2)));
        assert_eq!(session.remove_track(TrackId(1)).map(|track| track.id()), Ok(TrackId(1)));
        assert_eq!(session.add_track("Three", TrackKind::Audio), Ok(TrackId(3)));
        let ids: Vec<_> = session.tracks().iter().map(Track::id).collect();
        assert_eq!(ids, [TrackId(2), TrackId(3)]);
        Ok(())
    }

    #[test]
    fn restored_ids_and_mix_state_survive_and_advance_allocation() -> Result<(), SessionError> {
        let mut session = test_session()?;
        assert_eq!(session.restore_track(TrackId(7), "Lead", TrackKind::Instrument, true, false, true), Ok(()));
        let lead = session.track(TrackId(7));
        assert_eq!(lead.map(Track::name), Some("Lead"));
        assert_eq!(lead.map(Track::kind), Some(TrackKind::Instrument));
        assert_eq!(lead.map(Track::is_muted), Some(true));
        assert_eq!(lead.map(Track::is_record_armed), Some(true));
        assert_eq!(session.add_track("Next", TrackKind::Midi), Ok(TrackId(8)));
        assert_eq!(session.restore_track(TrackId(7), "Again", TrackKind::Midi, false, false, false), Err(SessionError::DuplicateTrackId));
        assert_eq!(session.restore_track(TrackId(0), "Zero", TrackKind::Midi, false, false, false), Err(SessionError::InvalidTrackId));
        Ok(())
    }

    #[test]
    fn hostile_values_are_bounded_and_reported() -> Result<(), SessionError> {
        assert_eq!(Session::new("Song", 7_999), Err(SessionError::InvalidSampleRate));
        assert_eq!(Session::new("Song", 768_001), Err(SessionError::InvalidSampleRate));

        let mut session = test_session()?;
        let long_name = "x".repeat(MAX_NAME_CHARS + 1);
        assert_eq!(session.add_track(&long_name, TrackKind::Audio), Err(SessionError::NameTooLong));
        assert_eq!(session.add_track("bad\0name", TrackKind::Audio), Err(SessionError::NameContainsNul));
        assert_eq!(session.rename_track(TrackId(99), "none"), Err(SessionError::MissingTrack));
        assert_eq!(session.set_mix_state(TrackId(99), false, false, false), Err(SessionError::MissingTrack));
        assert_eq!(session.remove_track(TrackId(99)), Err(SessionError::MissingTrack));
        Ok(())
    }

    #[test]
    fn native_track_count_is_bounded() -> Result<(), SessionError> {
        let mut session = test_session()?;
        for index in 0..MAX_TRACKS {
            let result = session.add_track("Track", TrackKind::ALL[index % TrackKind::ALL.len()]);
            assert_eq!(result, Ok(TrackId(index as u64 + 1)));
        }
        assert_eq!(session.add_track("Too many", TrackKind::Audio), Err(SessionError::TrackLimit));
        Ok(())
    }

    #[test]
    fn track_kind_capabilities_match_the_canonical_model() {
        assert!(TrackKind::Audio.has_playlist());
        assert!(TrackKind::Video.has_playlist());
        assert!(!TrackKind::Aux.has_playlist());
        assert!(TrackKind::Midi.is_midi());
        assert!(TrackKind::Instrument.is_midi());
        assert!(!TrackKind::Audio.is_midi());
    }
}
