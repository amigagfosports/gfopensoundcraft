#!/usr/bin/env bash
# Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
# Rust's built-in m68k target describes a compiler driver.  The local runner
# carries GNU m68k binutils, so translate its -Wl arguments for direct ld.
set -euo pipefail

linker=${M68K_REAL_LINKER:-m68k-linux-gnu-ld}
filtered=()
for argument in "$@"; do
    case $argument in
        -no-pie|-nodefaultlibs|-nostdlib)
            ;;
        -Wl,*)
            IFS=',' read -r -a forwarded <<< "${argument#-Wl,}"
            filtered+=("${forwarded[@]}")
            ;;
        *)
            filtered+=("$argument")
            ;;
    esac
done

exec "$linker" "${filtered[@]}"
