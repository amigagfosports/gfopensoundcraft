#!/usr/bin/env bash
# Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
# Rust's generic GNU-cc driver adds -no-pie.  The Amiga GCC16 driver is
# non-PIE by construction and rejects that host-only spelling.
set -euo pipefail

compiler=${M68K_REAL_CC:-m68k-amigaos-gcc}
filtered=()
for argument in "$@"; do
    if [[ $argument != -no-pie ]]; then
        filtered+=("$argument")
    fi
done

exec "$compiler" "${filtered[@]}"
