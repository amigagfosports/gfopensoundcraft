#!/bin/sh
# Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
set -eu

port=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$port/../.." && pwd)
tool_dir=${1:?usage: package-qualification.sh /path/to/m68k-amigaos-bin}
name=GFOpenSoundCraft-0.1.0-q1
dist="$root/dist/amigaos"
stage=$(mktemp -d "${TMPDIR:-/tmp}/gfopensoundcraft-package.XXXXXX")
drawer="$stage/$name"
trap 'rm -rf "$stage"' EXIT HUP INT TERM

test -x "$tool_dir/m68k-amigaos-ld"
test -x "$tool_dir/m68k-amigaos-nm"

M68K_AMIGA_HUNK=1 \
M68K_REAL_LINKER="$tool_dir/m68k-amigaos-ld" \
M68K_NM="$tool_dir/m68k-amigaos-nm" \
M68K_LINK_TMPDIR="$port/target-hunk/link-tmp" \
CARGO_TARGET_DIR="$port/target-hunk" \
    "$port/check.sh"

mkdir -p "$drawer"
install -m 0755 "$port/target-hunk/m68k-unknown-none-elf/release/gfopensoundcraft-amiga" "$drawer/SoundCraft"
install -m 0644 "$port/QUALIFICATION-NOTES.txt" "$drawer/README.txt"

mkdir -p "$dist"
(cd "$stage" && lha aq "$name.lha" "$name")
install -m 0644 "$stage/$name.lha" "$dist/$name.lha"
(cd "$stage" && sha256sum "$name/SoundCraft" "$name/README.txt") > "$dist/SHA256SUMS"
(cd "$dist" && sha256sum "$name.lha") >> "$dist/SHA256SUMS"

echo "Qualification package: $dist/$name.lha"
