// Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
//! Bounded native sequencer data matching SoundCraft's shared `craft-midi` model.

use alloc::vec::Vec;
use core::fmt;

pub const PPQ: u32 = 960;
pub const MAX_NATIVE_EVENTS: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Note {
    pub pitch: u8,
    pub velocity: u8,
    pub release_velocity: u8,
    pub channel: u8,
    pub start: i64,
    pub length: i64,
}

impl Note {
    pub const fn end(self) -> i64 {
        self.start.saturating_add(self.length)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CtrlKind {
    Cc(u8),
    PitchBend,
    ChannelPressure,
    PolyPressure(u8),
    Program,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CtrlEvent {
    pub tick: i64,
    pub channel: u8,
    pub kind: CtrlKind,
    pub value: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MidiError {
    InvalidPitch,
    InvalidVelocity,
    InvalidReleaseVelocity,
    InvalidChannel,
    InvalidLength,
    InvalidController,
    InvalidControllerValue,
    EventLimit,
    OutOfMemory,
}

impl fmt::Display for MidiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPitch => "MIDI pitch must be between 0 and 127",
            Self::InvalidVelocity => "note velocity must be between 1 and 127",
            Self::InvalidReleaseVelocity => "release velocity must be between 0 and 127",
            Self::InvalidChannel => "MIDI channel must be between 0 and 15",
            Self::InvalidLength => "note length must be positive",
            Self::InvalidController => "controller or pressure pitch must be between 0 and 127",
            Self::InvalidControllerValue => "controller value is outside its MIDI range",
            Self::EventLimit => "native MIDI event limit reached",
            Self::OutOfMemory => "not enough memory for MIDI data",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sequence {
    notes: Vec<Note>,
    controllers: Vec<CtrlEvent>,
    event_limit: usize,
}

impl Default for Sequence {
    fn default() -> Self {
        Self::with_event_limit(MAX_NATIVE_EVENTS)
    }
}

impl Sequence {
    pub fn with_event_limit(event_limit: usize) -> Self {
        Self { notes: Vec::new(), controllers: Vec::new(), event_limit: event_limit.min(MAX_NATIVE_EVENTS) }
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn controllers(&self) -> &[CtrlEvent] {
        &self.controllers
    }

    pub fn event_count(&self) -> usize {
        self.notes.len().saturating_add(self.controllers.len())
    }

    pub const fn event_limit(&self) -> usize {
        self.event_limit
    }

    pub fn add_note(&mut self, note: Note) -> Result<(), MidiError> {
        validate_note(note)?;
        self.reserve_event(true)?;
        self.notes.push(note);
        Ok(())
    }

    pub fn add_controller(&mut self, event: CtrlEvent) -> Result<(), MidiError> {
        validate_controller(event)?;
        self.reserve_event(false)?;
        self.controllers.push(event);
        Ok(())
    }

    pub fn sort(&mut self) {
        self.notes.sort_by_key(|note| (note.start, note.pitch, note.channel, note.length, note.velocity));
        self.controllers.sort_by_key(|event| event.tick);
    }

    pub fn end_tick(&self) -> i64 {
        let note_end = self.notes.iter().map(|note| note.end()).max();
        let controller_end = self.controllers.iter().map(|event| event.tick).max();
        match (note_end, controller_end) {
            (Some(note), Some(controller)) => note.max(controller),
            (Some(end), None) | (None, Some(end)) => end,
            (None, None) => 0,
        }
    }

    pub fn notes_in(&self, start: i64, end: i64) -> impl Iterator<Item = &Note> + '_ {
        self.notes.iter().filter(move |note| note.start >= start && note.start < end)
    }

    pub fn notes_overlapping(&self, start: i64, end: i64) -> impl Iterator<Item = &Note> + '_ {
        self.notes.iter().filter(move |note| note.start < end && note.end() > start)
    }

    fn reserve_event(&mut self, note: bool) -> Result<(), MidiError> {
        if self.event_count() >= self.event_limit {
            return Err(MidiError::EventLimit);
        }
        if note {
            self.notes.try_reserve(1).map_err(|_| MidiError::OutOfMemory)
        } else {
            self.controllers.try_reserve(1).map_err(|_| MidiError::OutOfMemory)
        }
    }
}

fn validate_note(note: Note) -> Result<(), MidiError> {
    if note.pitch > 127 {
        return Err(MidiError::InvalidPitch);
    }
    if !(1..=127).contains(&note.velocity) {
        return Err(MidiError::InvalidVelocity);
    }
    if note.release_velocity > 127 {
        return Err(MidiError::InvalidReleaseVelocity);
    }
    if note.channel > 15 {
        return Err(MidiError::InvalidChannel);
    }
    if note.length <= 0 {
        return Err(MidiError::InvalidLength);
    }
    Ok(())
}

fn validate_controller(event: CtrlEvent) -> Result<(), MidiError> {
    if event.channel > 15 {
        return Err(MidiError::InvalidChannel);
    }
    let valid = match event.kind {
        CtrlKind::Cc(controller) => {
            if controller > 127 {
                return Err(MidiError::InvalidController);
            }
            (0..=127).contains(&event.value)
        }
        CtrlKind::PitchBend => (-8_192..=8_191).contains(&event.value),
        CtrlKind::ChannelPressure | CtrlKind::Program => (0..=127).contains(&event.value),
        CtrlKind::PolyPressure(pitch) => {
            if pitch > 127 {
                return Err(MidiError::InvalidController);
            }
            (0..=127).contains(&event.value)
        }
    };
    if valid { Ok(()) } else { Err(MidiError::InvalidControllerValue) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(pitch: u8, start: i64, length: i64) -> Note {
        Note { pitch, velocity: 100, release_velocity: 64, channel: 0, start, length }
    }

    #[test]
    fn note_and_controller_data_match_craft_midi_semantics() {
        let mut sequence = Sequence::with_event_limit(8);
        assert_eq!(sequence.add_note(note(64, 960, 480)), Ok(()));
        assert_eq!(sequence.add_note(note(60, 0, 960)), Ok(()));
        assert_eq!(sequence.add_controller(CtrlEvent { tick: 480, channel: 0, kind: CtrlKind::PitchBend, value: 200 }), Ok(()));
        sequence.sort();
        assert_eq!(sequence.notes().iter().map(|event| event.pitch).collect::<Vec<_>>(), [60, 64]);
        assert_eq!(sequence.event_count(), 3);
        assert_eq!(sequence.event_limit(), 8);
        assert_eq!(sequence.end_tick(), 1_440);
        assert_eq!(sequence.controllers().len(), 1);
    }

    #[test]
    fn range_queries_use_half_open_timeline_rules() {
        let mut sequence = Sequence::with_event_limit(8);
        assert_eq!(sequence.add_note(note(60, 0, 960)), Ok(()));
        assert_eq!(sequence.add_note(note(64, 960, 480)), Ok(()));
        assert_eq!(sequence.notes_in(0, 960).count(), 1);
        assert_eq!(sequence.notes_in(960, 1_440).count(), 1);
        assert_eq!(sequence.notes_overlapping(959, 961).count(), 2);
        assert_eq!(sequence.notes_overlapping(1_440, 2_000).count(), 0);
    }

    #[test]
    fn malformed_notes_are_refused() {
        let mut sequence = Sequence::with_event_limit(8);
        assert_eq!(sequence.add_note(note(128, 0, 1)), Err(MidiError::InvalidPitch));
        let mut event = note(60, 0, 1);
        event.velocity = 0;
        assert_eq!(sequence.add_note(event), Err(MidiError::InvalidVelocity));
        event.velocity = 1;
        event.release_velocity = 128;
        assert_eq!(sequence.add_note(event), Err(MidiError::InvalidReleaseVelocity));
        event.release_velocity = 0;
        event.channel = 16;
        assert_eq!(sequence.add_note(event), Err(MidiError::InvalidChannel));
        event.channel = 0;
        event.length = 0;
        assert_eq!(sequence.add_note(event), Err(MidiError::InvalidLength));
        assert_eq!(sequence.event_count(), 0);
    }

    #[test]
    fn malformed_controllers_are_refused() {
        let mut sequence = Sequence::with_event_limit(8);
        let controller = |channel, kind, value| CtrlEvent { tick: 0, channel, kind, value };
        assert_eq!(sequence.add_controller(controller(16, CtrlKind::Program, 0)), Err(MidiError::InvalidChannel));
        assert_eq!(sequence.add_controller(controller(0, CtrlKind::Cc(128), 0)), Err(MidiError::InvalidController));
        assert_eq!(sequence.add_controller(controller(0, CtrlKind::PolyPressure(128), 0)), Err(MidiError::InvalidController));
        assert_eq!(sequence.add_controller(controller(0, CtrlKind::PitchBend, 8_192)), Err(MidiError::InvalidControllerValue));
        assert_eq!(sequence.add_controller(controller(0, CtrlKind::ChannelPressure, -1)), Err(MidiError::InvalidControllerValue));
        assert_eq!(sequence.add_controller(controller(0, CtrlKind::Program, 128)), Err(MidiError::InvalidControllerValue));
    }

    #[test]
    fn configured_event_limit_is_atomic() {
        let mut sequence = Sequence::with_event_limit(2);
        assert_eq!(sequence.add_note(note(60, 0, 10)), Ok(()));
        assert_eq!(sequence.add_controller(CtrlEvent { tick: 0, channel: 0, kind: CtrlKind::Cc(1), value: 64 }), Ok(()));
        assert_eq!(sequence.add_note(note(62, 10, 10)), Err(MidiError::EventLimit));
        assert_eq!(sequence.event_count(), 2);
    }

    #[test]
    fn empty_sequence_ends_at_zero_and_note_end_saturates() {
        assert_eq!(Sequence::default().end_tick(), 0);
        assert_eq!(note(60, i64::MAX - 4, 10).end(), i64::MAX);
    }
}
