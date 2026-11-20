#!/usr/bin/env bash
# Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
# Rust's generic GNU-cc driver adds -no-pie.  The Amiga GCC16 driver is
# non-PIE by construction and rejects that host-only spelling.
set -euo pipefail

compiler=${M68K_REAL_CC:-m68k-amigaos-gcc}
filtered=()
for argument in "$@"; do
    case $argument in
        -no-pie|-Wl,--as-needed|-Wl,-Bstatic|-Wl,-Bdynamic|-Wl,--eh-frame-hdr|-Wl,-z,noexecstack)
            ;;
        *)
            filtered+=("$argument")
            ;;
    esac
done

exec "$compiler" "${filtered[@]}"
