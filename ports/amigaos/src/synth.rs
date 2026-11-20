// Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
//! Translation from SoundCraft's native MIDI subset to GFOpenSynth messages.

use crate::midi::{CtrlEvent, CtrlKind, Note};
use amiga_gfopensynth::{MessageError, ShortMessage};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduledMessage {
    pub tick: i64,
    pub message: ShortMessage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SynthError {
    InvalidMessage(MessageError),
}

impl From<MessageError> for SynthError {
    fn from(error: MessageError) -> Self {
        Self::InvalidMessage(error)
    }
}

pub fn note_messages(note: Note) -> Result<[ScheduledMessage; 2], SynthError> {
    let on = ShortMessage::note_on(note.channel, note.pitch, note.velocity)?;
    let off = ShortMessage::note_off(note.channel, note.pitch, note.release_velocity)?;
    Ok([ScheduledMessage { tick: note.start, message: on }, ScheduledMessage { tick: note.end(), message: off }])
}

pub fn controller_message(event: CtrlEvent) -> Result<ScheduledMessage, SynthError> {
    let message = match event.kind {
        CtrlKind::Cc(controller) => ShortMessage::control_change(event.channel, controller, to_data(event.value)?)?,
        CtrlKind::PitchBend => {
            let value = i16::try_from(event.value).map_err(|_| SynthError::InvalidMessage(MessageError::InvalidPitchBend))?;
            ShortMessage::pitch_bend(event.channel, value)?
        }
        CtrlKind::ChannelPressure => ShortMessage::channel_pressure(event.channel, to_data(event.value)?)?,
        CtrlKind::PolyPressure(pitch) => ShortMessage::poly_pressure(event.channel, pitch, to_data(event.value)?)?,
        CtrlKind::Program => ShortMessage::program_change(event.channel, to_data(event.value)?)?,
    };
    Ok(ScheduledMessage { tick: event.tick, message })
}

pub fn all_notes_off(channel: u8) -> Result<ShortMessage, SynthError> {
    ShortMessage::all_notes_off(channel).map_err(Into::into)
}

fn to_data(value: i32) -> Result<u8, SynthError> {
    let value = u8::try_from(value).map_err(|_| SynthError::InvalidMessage(MessageError::InvalidData))?;
    if value > 127 {
        return Err(SynthError::InvalidMessage(MessageError::InvalidData));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note() -> Note {
        Note { pitch: 60, velocity: 100, release_velocity: 64, channel: 2, start: 960, length: 480 }
    }

    #[test]
    fn notes_become_timestamped_on_and_off_messages() {
        let messages = note_messages(note())
            .map(|messages| [(messages[0].tick, messages[0].message.packed()), (messages[1].tick, messages[1].message.packed())]);
        assert_eq!(messages, Ok([(960, 0x0064_3c92), (1_440, 0x0040_3c82)]));
    }

    #[test]
    fn every_native_controller_kind_has_a_typed_message() {
        let message =
            |kind, value| controller_message(CtrlEvent { tick: 240, channel: 3, kind, value }).map(|event| (event.tick, event.message.packed()));
        assert_eq!(message(CtrlKind::Cc(7), 100), Ok((240, 0x0064_07b3)));
        assert_eq!(message(CtrlKind::PitchBend, 0), Ok((240, 0x0040_00e3)));
        assert_eq!(message(CtrlKind::ChannelPressure, 80), Ok((240, 0x0000_50d3)));
        assert_eq!(message(CtrlKind::PolyPressure(60), 90), Ok((240, 0x005a_3ca3)));
        assert_eq!(message(CtrlKind::Program, 12), Ok((240, 0x0000_0cc3)));
    }

    #[test]
    fn invalid_unchecked_model_values_still_fail_closed() {
        let mut invalid = note();
        invalid.channel = 16;
        assert_eq!(note_messages(invalid), Err(SynthError::InvalidMessage(MessageError::InvalidChannel)));
        assert_eq!(
            controller_message(CtrlEvent { tick: 0, channel: 0, kind: CtrlKind::Program, value: 128 }),
            Err(SynthError::InvalidMessage(MessageError::InvalidData))
        );
    }
}
