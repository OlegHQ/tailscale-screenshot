#!/usr/bin/env bash
# Live-dev loop: launch the extension in Firefox Developer Edition with web-ext
# and auto-reload on every source save.
#
# web-ext watches its --source-dir, so we run it straight against src/ with the
# Firefox manifest copied in as manifest.json (gitignored). Editing popup.*
# triggers an automatic reload — no rebuild step, no second process.
set -euo pipefail

ext="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src="$ext/src"

cp "$ext/manifest.firefox.json" "$src/manifest.json"
trap 'rm -f "$src/manifest.json"' EXIT

ff="${FIREFOX_BIN:-/Applications/Firefox Developer Edition.app/Contents/MacOS/firefox}"

npx --yes web-ext run \
  --source-dir "$src" \
  --firefox "$ff" \
  --browser-console \
  "$@"
