// Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
#![cfg_attr(target_arch = "m68k", no_main)]
#![cfg_attr(target_arch = "m68k", no_std)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

extern crate alloc;

#[cfg(any(target_arch = "m68k", test))]
pub mod midi;
#[cfg(any(target_arch = "m68k", test))]
mod platform;
#[cfg(any(target_arch = "m68k", test))]
pub mod session;

#[cfg(target_arch = "m68k")]
use alloc::vec::Vec;
#[cfg(target_arch = "m68k")]
use midi::{Note, Sequence};
#[cfg(target_arch = "m68k")]
use platform::{BINDINGS, STARTUP_LINES};
#[cfg(target_arch = "m68k")]
use session::{Session, TrackKind};

#[cfg(target_arch = "m68k")]
#[allow(unsafe_code, reason = "the fixed AmigaDOS entry symbol is the port's isolated ABI boundary")]
#[unsafe(no_mangle)]
pub extern "C" fn gf_amiga_main(_args: *const u8, _length: i32) -> i32 {
    amiga_runtime::run(|runtime| {
        let allocation_proof: Vec<_> = BINDINGS.iter().collect();
        for line in STARTUP_LINES {
            runtime.put_str(line);
        }

        let Ok(mut session) = Session::new("Untitled", 48_000) else {
            return 20;
        };
        if session.add_track("Audio 1", TrackKind::Audio).is_err() || session.add_track("MIDI 1", TrackKind::Midi).is_err() {
            return 20;
        }
        runtime.put_str(c"portable model: audio and MIDI tracks ready\n");

        let mut midi = Sequence::with_event_limit(64);
        if midi.add_note(Note { pitch: 60, velocity: 100, release_velocity: 64, channel: 0, start: 0, length: i64::from(midi::PPQ) }).is_err() {
            return 20;
        }
        runtime.put_str(c"sequencer model: one quarter-note ready\n");

        if allocation_proof.len() == BINDINGS.len() && session.track_count() == 2 && session.sample_rate_hz() == 48_000 && midi.event_count() == 1 {
            0
        } else {
            20
        }
    })
}

#[cfg(target_arch = "m68k")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_arch = "m68k"))]
fn main() {}
