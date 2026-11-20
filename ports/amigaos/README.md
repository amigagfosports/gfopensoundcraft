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

The port does not yet claim the egui interface, timed transport playback,
audio clips, SMF file I/O, MIDI device I/O or session file I/O have been
ported.  The next slices are the native window/event adapter, AHI render
callback and CAMD MIDI queue.  Release candidates must use GFOpenRust's pinned
patched compiler and pass on AmigaOS.
