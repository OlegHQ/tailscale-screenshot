#!/usr/bin/env bash
# Assemble an unpacked extension for one browser by copying the shared src/
# and dropping in that browser's manifest as manifest.json.
#
#   build.sh <chrome|firefox>   ->   extension/build/<browser>/
set -euo pipefail

browser="${1:?usage: build.sh <chrome|firefox>}"
ext="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src="$ext/src"
out="$ext/build/$browser"
manifest="$ext/manifest.$browser.json"

[ -f "$manifest" ] || { echo "no manifest for '$browser': $manifest" >&2; exit 1; }

rm -rf "$out"
mkdir -p "$out"
# Copy everything from src except a stray generated manifest.json...
cp -R "$src"/. "$out"/
rm -f "$out/manifest.json"
# ...then drop in the browser-specific one.
cp "$manifest" "$out/manifest.json"

echo "built $browser -> $out"
