#!/usr/bin/env bash
# Builds sing-box, TUN mode's engine, for one target into
# apps/desktop/src-tauri/binaries/geekvpn-sing-box-<rust triple>.
#
#   scripts/build-singbox.sh [rust-target-triple]   (default: this machine)
#
# From the official module at a pinned version: `go install` checks every
# module against go.sum and the Go checksum database, so what is built is
# exactly sing-box's tagged source. The name keeps it apart from a sing-box
# the user may have installed themselves (/usr/bin in a .deb).
set -euo pipefail

SING_BOX_VERSION="v1.14.2"
# TUN's userspace stack for UDP, and the API the Connections page reads.
TAGS="with_gvisor,with_clash_api"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TRIPLE="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
BIN="$ROOT/apps/desktop/src-tauri/binaries"

if [[ "$TRIPLE" == universal-apple-darwin ]]; then
  "$0" aarch64-apple-darwin
  "$0" x86_64-apple-darwin
  lipo -create -output "$BIN/geekvpn-sing-box-universal-apple-darwin" \
    "$BIN/geekvpn-sing-box-aarch64-apple-darwin" "$BIN/geekvpn-sing-box-x86_64-apple-darwin"
  echo "built $BIN/geekvpn-sing-box-universal-apple-darwin"
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
# `go install` refuses GOBIN when cross-compiling; build in a scratch module
# that requires exactly the pinned version instead.
cd "$WORK"
go mod init geekvpn/singbox-build >/dev/null 2>&1
# The command package, so its own imports are required too; with nothing
# else in this module, every version is the one sing-box itself selects.
go get "github.com/sagernet/sing-box/cmd/sing-box@$SING_BOX_VERSION" 2>&1 | grep -v "^go: \(downloading\|added\|upgraded\)" || true
GOOS=$GOOS GOARCH=$GOARCH go build -trimpath -tags "$TAGS" \
  -ldflags "-s -w -buildid= -X github.com/sagernet/sing-box/constant.Version=${SING_BOX_VERSION#v}" \
  -o "$BIN/geekvpn-sing-box-$TRIPLE$EXT" github.com/sagernet/sing-box/cmd/sing-box
echo "built $BIN/geekvpn-sing-box-$TRIPLE$EXT"
