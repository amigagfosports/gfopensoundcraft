# GFOpenSoundCraft for AmigaOS 3.x

This directory is the native 68040 port, not a host wrapper.  It is kept as a
standalone Cargo workspace while the portable SoundCraft crates are separated
from their current desktop-only dependencies.

The first milestone proves the platform boundary and GFOpenRust linkage:

- AmigaDOS D0/A0 startup and the Exec-pool allocator come from GFOpenRust,
  pinned by commit;
- the developer gate emits a static, big-endian 32-bit m68k ELF, while the
  qualification gate emits a native Amiga Hunk executable; both have no
  unresolved symbols;
- application code contains no unsafe block; the one exported entry attribute
  is isolated and documented;
- the six platform services are assigned before their implementations begin.

| SoundCraft service | Native backend |
| --- | --- |
| Windows, controls and layout | GFOpenLayout and OpenGadTools |
| Playback and recording | AHI and OpenAudio |
| MIDI input/output | camd.library |
| Audio and media formats | GFOpenDataTypes |
| Built-in instruments | AkikoPlus OpenSynth |
| Installation and updates | GFOpenUp |

Run the complete current gate with:

```sh
ports/amigaos/check.sh
```

With an Amiga-capable binutils prefix, build the native qualification drawer
and LHA archive with:

```sh
ports/amigaos/package-qualification.sh /path/to/m68k-amigaos-bin
```

The current executable now carries the first bounded session-model subset: a
session name and sample rate, the eight canonical SoundCraft track kinds,
stable session-wide track ids, and track rename/mute/solo/record-arm state.
Names, track count and allocations are bounded and return errors rather than
panicking.  The full `craft-scraft-format` schema is still the canonical
desktop model and remains `std`-bound; this subset is an adapter, not a second
file format.

The sequencer-data subset uses the same 960 PPQ note and controller fields as
`craft-midi`, with validation, deterministic ordering, timeline queries and a
per-sequence event ceiling capped at 65,536.  The linked smoke path creates a
quarter-note sequence through the native allocator.

The native synth adapter translates every current note and controller kind to
validated `gfopensynth.library` version 1 messages.  Its linked smoke path
opens and resets the library, sends one note on/off pair, sends all-notes-off,
and closes the synth through GFOpenRust's lifetime-bound safe API.  AkikoPlus
selection and the 68k/AHI fallback remain the library's responsibility.  This
path passed with AmigaDOS `RC=0` in an isolated 68040/FPU lab on 20 November
2026.  The receipt is in `evidence/native-68040-q1-20261120.txt`; it proves the
library message path, not audible output.

The Q2 slice adds a native GFOpenLayout editor rather than attempting to carry
the desktop egui stack onto the 68k.  It renders transport, track, timeline and
inspector panels.  Play sends the bounded model's C4 note-on/note-off pair via
GFOpenSynth, Stop resets the synth output, and MIDI arm updates the real session
model.  All three controls and clean close were exercised in the isolated Team
1 68040/FPU lab on 21 November 2026.  The exact receipt is in
`evidence/native-68040-window-q2-20261121.txt`.

The UAT1 slice adds bounded SCF1 persistence at `PROGDIR:SoundCraft.scf`.  Save and
Open preserve the session, tracks, mix state, notes, controllers and MIDI arm
state.  The parser rejects truncated, malformed, over-limit and trailing data
without replacing the live project.  The exact Kitchen-built candidate passed
native save, reopen, three-byte hostile-file rejection and clean close in an
isolated 68040/FPU lab on 22 November 2026.  It also remains usable when
`gfopensynth.library` is absent; Play reports that audition is unavailable
instead of terminating.  The receipt is in
`evidence/native-68040-uat1-20261122.txt`.

The UAT2 package adds a generated Installer, application/drawer/ReadMe
GlowIcons, licences and attribution.  The exact package passed Pretend, fresh
install and update walks with Installer 47.19 in the Team 1 68040/FPU lab.  A
97-byte `SoundCraft.scf` placed in the installed drawer retained its exact
SHA-256 across the update.  The installed executable is byte-identical to the
packaged executable.  See `evidence/native-68040-uat1-20261122.txt`.

The port does not yet claim timed transport playback, audio clips, SMF file
I/O, MIDI device I/O, AHI recording/rendering or CAMD queues.  The visible Add
MIDI track button is disabled and labelled as the next slice.  Synth-present
startup and note/reset message delivery now pass with the qualified
GFOpenSynth library and SoundFont, but the muted lab cannot prove audible
output.  UAT2 is therefore not promoted to RC yet.
Release candidates must use GFOpenRust's pinned patched compiler and pass on
AmigaOS.
