#!/usr/bin/env bash
# Builds core/geekcore for one target into apps/desktop/src-tauri/binaries/,
# named the way Tauri's sidecar bundling expects (geekcore-<rust triple>).
#
#   scripts/build-geekcore.sh [rust-target-triple]   (default: this machine)
#
# The repository's go.mod is never modified: the build runs in a scratch copy
# whose xray-core is the pinned version with scripts/xray-patches applied.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TRIPLE="${1:-$(rustc -vV | sed -n 's/^host: //p')}"

# macOS ships one universal app: build both slices and join them.
if [[ "$TRIPLE" == universal-apple-darwin ]]; then
  "$0" aarch64-apple-darwin
  "$0" x86_64-apple-darwin
  BIN="$ROOT/apps/desktop/src-tauri/binaries"
  lipo -create -output "$BIN/geekcore-universal-apple-darwin" \
    "$BIN/geekcore-aarch64-apple-darwin" "$BIN/geekcore-x86_64-apple-darwin"
  echo "built $BIN/geekcore-universal-apple-darwin"
  exit 0
fi
export GOTOOLCHAIN="${GOTOOLCHAIN:-auto}" CGO_ENABLED=0

case "$TRIPLE" in
  x86_64-unknown-linux-gnu)   GOOS=linux   GOARCH=amd64 ;;
  aarch64-unknown-linux-gnu)  GOOS=linux   GOARCH=arm64 ;;
  x86_64-pc-windows-msvc)     GOOS=windows GOARCH=amd64 ;;
  aarch64-pc-windows-msvc)    GOOS=windows GOARCH=arm64 ;;
  x86_64-apple-darwin)        GOOS=darwin  GOARCH=amd64 ;;
  aarch64-apple-darwin)       GOOS=darwin  GOARCH=arm64 ;;
  *) echo "unsupported target $TRIPLE" >&2; exit 1 ;;
esac
EXT=""; [[ "$GOOS" == windows ]] && EXT=".exe"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cp -a "$ROOT/core/geekcore/." "$WORK/"
cd "$WORK"

XRAY_MODULE="github.com/xtls/xray-core"
XRAY_SRC="$(go mod download -json "$XRAY_MODULE" | sed -n 's/.*"Dir": "\(.*\)".*/\1/p')"
cp -a "$XRAY_SRC" "$WORK/xray-core"
chmod -R u+w "$WORK/xray-core"
for p in "$ROOT"/scripts/xray-patches/*.patch; do
  patch -d "$WORK/xray-core" -p1 --forward < "$p"
done
cp "$ROOT"/scripts/xray-patches/*_test.go "$WORK/xray-core/infra/conf/"
(cd "$WORK/xray-core" && GOOS= GOARCH= go test ./infra/conf/ -run 'TestGeekVPN')
go mod edit -replace="$XRAY_MODULE=./xray-core"
go mod tidy

OUT="$ROOT/apps/desktop/src-tauri/binaries/geekcore-$TRIPLE$EXT"
mkdir -p "$(dirname "$OUT")"
GOOS=$GOOS GOARCH=$GOARCH go build -trimpath \
  -ldflags='-s -w -buildid= -checklinkname=0' -o "$OUT" .
echo "built $OUT"
