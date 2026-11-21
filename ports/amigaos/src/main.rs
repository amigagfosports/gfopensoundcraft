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
#[cfg(any(target_arch = "m68k", test))]
pub mod synth;

#[cfg(target_arch = "m68k")]
use alloc::vec::Vec;
#[cfg(target_arch = "m68k")]
use midi::{Note, Sequence};
#[cfg(target_arch = "m68k")]
use platform::{BINDINGS, STARTUP_LINES};
#[cfg(target_arch = "m68k")]
use session::{Session, TrackId, TrackKind};

#[cfg(target_arch = "m68k")]
const EDITOR_MARKUP: &core::ffi::CStr = c"<column id=app><row id=transport><button id=play>Play note</button><button id=stop>Stop</button><button id=record>Arm MIDI</button><div class=spacer></div><div class=badge>AkikoPlus OpenSynth</div></row><row id=workspace><column id=tracks><div class=heading>Tracks</div><div class='track selected'><span>Audio 1</span><span class=kind>AUDIO</span></div><div class='track selected'><span>MIDI 1</span><span class=kind>MIDI</span></div><button disabled>Add MIDI track (next)</button></column><column id=timeline><row id=ruler><div>1</div><div>2</div><div>3</div><div>4</div></row><div class=lane><div class=wave></div></div><div class=lane><div class=note></div></div><div id=transportstate>Stopped - 120 BPM - 4/4 - 960 PPQ</div></column><column id=mixer><div class=heading>Inspector</div><row><label>Session</label><div>Untitled</div></row><row><label>Rate</label><div>48 kHz</div></row><row><label>Tracks</label><div>2</div></row><row><label>MIDI</label><div id=recordstate>Safe</div></row><div class=heading>Output</div><div class=output>GFOpenSynth</div><div class=help>Uses AkikoPlus when present, with the native fallback selected by the library.</div></column></row><row id=status><div id=statusline>Ready - audition the bounded native sequence</div><div class=spacer></div><div>Native 68040</div></row></column>";

#[cfg(target_arch = "m68k")]
const EDITOR_CSS: &core::ffi::CStr = c"*{box-sizing:border-box}#app{width:100%;height:100%;background:#20242b;color:#edf1f7}#transport{height:50px;padding:7px;gap:7px;background:#303640;border-bottom:1px solid #080a0e;align-items:center}#transport button{height:34px;min-width:88px;background:#48515e;color:#ffffff;border:1px solid #727d8e}.spacer{flex:1}.badge{padding:8px 11px;background:#7053a6;color:#ffffff;border-radius:4px}#workspace{flex:1;min-height:0}#tracks{width:224px;padding:10px;gap:7px;background:#292f38;border-right:1px solid #080a0e}.heading{font-weight:bold;color:#b9c3d2;padding:7px 2px;border-bottom:1px solid #596372}.track{height:42px;padding:8px;background:#373e49;border:1px solid #596271}.track.selected{background:#355d71}.kind{float:right;color:#b9d8e7}.hidden{display:none}#tracks button{height:34px;background:#3f765e;color:#ffffff;border:1px solid #65a88b}#timeline{flex:1;min-width:0;padding:10px;gap:8px;background:#171b21}#ruler{height:28px;background:#2c323b;color:#aeb8c8;justify-content:space-around;align-items:center}.lane{height:92px;background:#242a32;border:1px solid #4e5867;padding:11px}.wave{height:54px;background:#3d6e7f;border-top:8px solid #5d9bad;border-bottom:8px solid #5d9bad}.note{width:146px;height:38px;margin:18px 0 0 64px;background:#8a68c2;border:1px solid #b89ae7}#transportstate{padding:9px;background:#2c323b;color:#d4dbe5}#mixer{width:230px;padding:10px;gap:8px;background:#292f38;border-left:1px solid #080a0e}#mixer row{height:30px;align-items:center}#mixer label{width:75px;color:#b9c3d2}.output{padding:10px;background:#7053a6}.help{color:#bbc5d1;line-height:1.35}#status{height:32px;padding:7px 10px;background:#303640;border-top:1px solid #080a0e;color:#d2dae5}";

#[cfg(target_arch = "m68k")]
struct SoundEditor<'runtime> {
    session: Session,
    midi_track: TrackId,
    note: Note,
    output: amiga_gfopensynth::Synth<'runtime>,
    title: &'static core::ffi::CStr,
    status: &'static core::ffi::CStr,
    auditioned: bool,
    record_armed: bool,
}

#[cfg(target_arch = "m68k")]
impl amiga_gfopenlayout::EventSink for SoundEditor<'_> {
    fn on_event(&mut self, event: amiga_gfopenlayout::Event<'_>) -> bool {
        if event.kind != amiga_gfopenlayout::EventKind::Click {
            return false;
        }
        match event.registration {
            0 => {
                let Ok(messages) = synth::note_messages(self.note) else {
                    return false;
                };
                self.output.send_short(messages[0].message);
                self.output.send_short(messages[1].message);
                self.title = c"SoundCraft - Auditioned C4";
                self.status = c"Sent note on/off through GFOpenSynth";
                self.auditioned = true;
            }
            1 => {
                self.output.reset();
                self.title = c"SoundCraft - Stopped";
                self.status = c"Reset the native synthesiser output";
                self.auditioned = false;
            }
            2 => {
                self.record_armed = !self.record_armed;
                if self.session.set_mix_state(self.midi_track, false, false, self.record_armed).is_err() {
                    return false;
                }
                self.title = if self.record_armed { c"SoundCraft - MIDI armed" } else { c"SoundCraft - MIDI safe" };
                self.status = if self.record_armed { c"MIDI 1 is armed for recording" } else { c"MIDI 1 is no longer record armed" };
            }
            _ => return false,
        }
        true
    }

    fn click_ids(&self) -> &'static [&'static core::ffi::CStr] {
        &[c"play", c"stop", c"record"]
    }

    fn window_title(&self) -> Option<&core::ffi::CStr> {
        Some(self.title)
    }

    fn refresh(&mut self, ui: &mut amiga_gfopenlayout::Ui<'_, '_>) {
        let _ = ui.set_text(c"statusline", self.status);
        let _ = ui.set_text(
            c"transportstate",
            if self.auditioned { c"Audition complete - 120 BPM - 4/4 - 960 PPQ" } else { c"Stopped - 120 BPM - 4/4 - 960 PPQ" },
        );
        let _ = ui.set_text(c"recordstate", if self.record_armed { c"Armed" } else { c"Safe" });
    }
}

#[cfg(target_arch = "m68k")]
#[allow(unsafe_code, reason = "the fixed AmigaDOS entry symbol is the port's isolated ABI boundary")]
#[unsafe(no_mangle)]
pub extern "C" fn gf_amiga_main(_args: *const u8, _length: i32) -> i32 {
    amiga_runtime::run(|runtime| {
        let allocation_proof: Vec<_> = BINDINGS.iter().collect();
        for line in STARTUP_LINES {
            runtime.put_str(line);
        }

        let mut session = match Session::new("Untitled", 48_000) {
            Ok(session) => session,
            Err(_) => {
                runtime.put_str(c"session model failed\n");
                return 20;
            }
        };
        if session.add_track("Audio 1", TrackKind::Audio).is_err() {
            runtime.put_str(c"track model failed\n");
            return 20;
        }
        let Ok(midi_track) = session.add_track("MIDI 1", TrackKind::Midi) else {
            runtime.put_str(c"MIDI track model failed\n");
            return 20;
        };
        runtime.put_str(c"portable model: audio and MIDI tracks ready\n");

        let mut midi = Sequence::with_event_limit(64);
        if midi.add_note(Note { pitch: 60, velocity: 100, release_velocity: 64, channel: 0, start: 0, length: i64::from(midi::PPQ) }).is_err() {
            runtime.put_str(c"sequencer model failed\n");
            return 20;
        }
        runtime.put_str(c"sequencer model: one quarter-note ready\n");

        let Some(note) = midi.notes().first().copied() else {
            return 20;
        };
        let Ok(messages) = synth::note_messages(note) else {
            return 20;
        };
        let Ok(mut output) = amiga_gfopensynth::Synth::open(runtime) else {
            runtime.put_str(c"GFOpenSynth unavailable\n");
            return 20;
        };
        output.reset();
        output.send_short(messages[0].message);
        output.send_short(messages[1].message);
        let Ok(silence) = synth::all_notes_off(0) else {
            return 20;
        };
        output.send_short(silence);
        runtime.put_str(c"synth adapter: GFOpenSynth message path ready\n");

        if allocation_proof.len() != BINDINGS.len() || session.track_count() != 2 || session.sample_rate_hz() != 48_000 || midi.event_count() != 1 {
            return 20;
        }
        let mut editor = SoundEditor {
            session,
            midi_track,
            note,
            output,
            title: c"SoundCraft - Untitled",
            status: c"Ready - audition the bounded native sequence",
            auditioned: false,
            record_armed: false,
        };
        let Ok(window) = amiga_gfopenlayout::Window::open(runtime, c"SoundCraft - Untitled", EDITOR_MARKUP, EDITOR_CSS, 940, 500) else {
            runtime.put_str(c"SoundCraft: could not open the native editor\n");
            return 21;
        };
        if !window.run_with(runtime, &mut editor) {
            runtime.put_str(c"SoundCraft: could not register native controls\n");
            return 22;
        }
        runtime.put_str(c"SoundCraft: native editor closed\n");
        0
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
