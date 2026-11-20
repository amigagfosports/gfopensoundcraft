# AmigaOS port boundary

The AmigaOS entry point is the one additional platform-boundary exception to
the repository's unsafe-code rule.  Only `src/main.rs` may allow the
`no_mangle` unsafe attribute needed to export `gf_amiga_main`; it must contain
no unsafe block.  All Amiga ABI calls and memory ownership stay in the pinned
GFOpenRust runtime dependency behind its safe callback API.

This port is native 68k AmigaOS 3.x.  Do not add a host wrapper, webview,
JavaScript shell or remote UI in place of native work.
