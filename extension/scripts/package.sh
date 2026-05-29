#!/usr/bin/env bash
# Build + package one browser into an installable artifact under dist/.
#   package.sh firefox  ->  extension/dist/tailscale-screenshot.xpi
#   package.sh chrome   ->  extension/dist/tailscale-screenshot-chrome.zip
set -euo pipefail

browser="${1:?usage: package.sh <chrome|firefox>}"
ext="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$ext/build/$browser"
dist="$ext/dist"

case "$browser" in
  firefox) filename="tailscale-screenshot.xpi" ;;
  chrome)  filename="tailscale-screenshot-chrome.zip" ;;
  *) echo "unknown browser: $browser" >&2; exit 1 ;;
esac

"$ext/scripts/build.sh" "$browser"
mkdir -p "$dist"
npx --yes web-ext build \
  --source-dir "$out" \
  --artifacts-dir "$dist" \
  --filename "$filename" \
  --overwrite-dest

echo "packaged $browser -> $dist/$filename"
