#!/bin/sh
# Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/.." && pwd)
out=${OUT:-"$root/build-amiga-package"}
version=0.1.0-uat2
archive="GFOpenSoundCraft-${version}-68040"
stage=$(mktemp -d "${TMPDIR:-/tmp}/gfopensoundcraft-package.XXXXXX")
trap 'rm -rf "$stage"' EXIT HUP INT TERM

mkdir -p "$out" "$stage/$archive/SoundCraft"
OUT="$stage/plate" "$root/ports/amigaos/build-amiga.sh"

install -m 0755 "$stage/plate/SoundCraft" "$stage/$archive/SoundCraft/SoundCraft"
install -m 0644 "$stage/plate/README.txt" "$stage/$archive/SoundCraft/README.txt"
install -m 0644 "$root/LICENSE-MIT" "$stage/$archive/SoundCraft/LICENSE-MIT"
install -m 0644 "$root/LICENSE-APACHE" "$stage/$archive/SoundCraft/LICENSE-APACHE"
install -m 0644 "$root/ATTRIBUTION.md" "$stage/$archive/SoundCraft/ATTRIBUTION.md"
install -m 0644 "$here/generated/icons/SoundCraft.info" "$stage/$archive/SoundCraft/SoundCraft.info"
install -m 0644 "$here/generated/icons/README.txt.info" "$stage/$archive/SoundCraft/README.txt.info"
install -m 0644 "$here/generated/icons/SoundCraftDrawer.info" "$stage/$archive/SoundCraft.info"
install -m 0644 "$here/generated/installer/Install-SoundCraft" "$stage/$archive/Install-SoundCraft"
install -m 0644 "$here/generated/installer/Install-SoundCraft.info" "$stage/$archive/Install-SoundCraft.info"

(cd "$stage" && lha aq "$archive.lha" "$archive")
install -m 0644 "$stage/$archive.lha" "$out/$archive.lha"
(cd "$out" && sha256sum "$archive.lha" > "$archive.lha.sha256")
install -m 0644 "$root/ports/amigaos/RELEASE-NOTES.txt" "$out/$archive.readme"

echo "GFOpenSoundCraft UAT package: $out/$archive.lha"
