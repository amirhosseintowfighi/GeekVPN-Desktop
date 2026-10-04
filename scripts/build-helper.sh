#!/usr/bin/env bash
# Builds geekvpn-helper (crates/geek-helper) for one target into
# apps/desktop/src-tauri/binaries/geekvpn-helper-<rust triple>, where Tauri
# bundles it next to the app. The app installs it as a service from there.
#
#   scripts/build-helper.sh [rust-target-triple] [--debug]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TRIPLE="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
PROFILE=release; FLAG=--release
[[ "${2:-}" == --debug ]] && { PROFILE=debug; FLAG=; }
BIN="$ROOT/apps/desktop/src-tauri/binaries"
mkdir -p "$BIN"

if [[ "$TRIPLE" == universal-apple-darwin ]]; then
  "$0" aarch64-apple-darwin "${2:-}"
  "$0" x86_64-apple-darwin "${2:-}"
  lipo -create -output "$BIN/geekvpn-helper-universal-apple-darwin" \
    "$BIN/geekvpn-helper-aarch64-apple-darwin" "$BIN/geekvpn-helper-x86_64-apple-darwin"
  echo "built $BIN/geekvpn-helper-universal-apple-darwin"
  exit 0
fi
EXT=""; [[ "$TRIPLE" == *windows* ]] && EXT=".exe"

cd "$ROOT"
# shellcheck disable=SC2086
cargo build --locked $FLAG -p geek-helper --target "$TRIPLE"
cp "target/$TRIPLE/$PROFILE/geekvpn-helper$EXT" "$BIN/geekvpn-helper-$TRIPLE$EXT"
echo "built $BIN/geekvpn-helper-$TRIPLE$EXT"
