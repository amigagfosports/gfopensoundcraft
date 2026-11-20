# GFOpenSoundCraft for AmigaOS 3.x

This directory is the native 68040 port, not a host wrapper.  It is kept as a
standalone Cargo workspace while the portable SoundCraft crates are separated
from their current desktop-only dependencies.

The first milestone proves the platform boundary and GFOpenRust linkage:

- AmigaDOS D0/A0 startup and the Exec-pool allocator come from GFOpenRust,
  pinned by commit;
- the output is a static, big-endian 32-bit m68k ELF with no unresolved
  symbols or relocations;
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

The current executable is a platform receipt.  It does not yet claim the
egui interface, playback engine or session model have been ported.  The next
slices are the native window/event adapter, AHI render callback, CAMD MIDI
queue and the first portable session/model subset.  Release candidates must
use GFOpenRust's pinned patched compiler and pass on AmigaOS.
