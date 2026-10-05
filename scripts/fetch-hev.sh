#!/usr/bin/env bash
# Fetches hev-socks5-tunnel + wintun.dll for the Windows bundle.
# Linux/macOS bundles still use sing-box (scripts/build-singbox.sh).
#
#   scripts/fetch-hev.sh [rust-target-triple]   (default: this machine)
#
# The Windows app bundles:
#   apps/desktop/src-tauri/binaries/hev-socks5-tunnel-<triple>.exe
#   apps/desktop/src-tauri/binaries/wintun-<triple>.dll
# On x86_64-pc-windows-msvc they are also copied without the triple suffix
# so `tauri dev` finds them.
set -euo pipefail

HEV_VERSION="2.18.0"
# hev-socks5-tunnel 2.18.0 win64 zip (contains hev exe + wintun.dll + msys-2.0.dll)
HEV_URL="https://github.com/heiher/hev-socks5-tunnel/releases/download/${HEV_VERSION}/hev-socks5-tunnel-win64.zip"
# sha256 of the zip above (verified 2026-10-05)
HEV_ZIP_SHA256="2b8cdcfdfafca4bd3732c759749547e0672217c568a0240e2570f28ca8f58bd6"
# sha256 of wintun.dll / hev exe / msys dll inside that zip (for extra check)
WINTUN_SHA256="e5da8447dc2c320edc0fc52fa01885c103de8c118481f683643cacc3220dafce"
HEV_EXE_SHA256="bd78baca0619a7a7dafd15aae0443b78be8a8152e7aa35752914b5b181e4ee9e"
MSYS_SHA256="1410599ee2efcede0869abec8910398d688755952e59cf7983b15cac6de78201"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TRIPLE="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
BIN="$ROOT/apps/desktop/src-tauri/binaries"
mkdir -p "$BIN"

case "$TRIPLE" in
  *windows*) : ;;
  *)
    echo "fetch-hev: $TRIPLE is not Windows, nothing to do (sing-box remains for this target)" >&2
    exit 0
    ;;
esac

EXT=".exe"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

ZIP="$WORK/hev-win64.zip"
echo "fetch-hev: downloading hev-socks5-tunnel $HEV_VERSION for $TRIPLE ..." >&2
if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o "$ZIP" "$HEV_URL"
elif command -v wget >/dev/null 2>&1; then
  wget -q -O "$ZIP" "$HEV_URL"
else
  echo "fetch-hev: need curl or wget" >&2; exit 1
fi

# sha256 check
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | awk '{print $1}'
  else
    # PowerShell fallback (Windows CI)
    powershell.exe -NoProfile -Command "(Get-FileHash -Algorithm SHA256 '$1').Hash.ToLower()" 2>/dev/null | tr -d '\r\n'
  fi
}

got="$(sha256_of "$ZIP")"
if [[ "${got,,}" != "${HEV_ZIP_SHA256,,}" ]]; then
  echo "fetch-hev: zip sha256 mismatch: got $got, want $HEV_ZIP_SHA256" >&2
  exit 1
fi

# unzip: prefer unzip, fallback to powershell Expand-Archive, then 7z
if command -v unzip >/dev/null 2>&1; then
  unzip -q "$ZIP" -d "$WORK/unzipped"
elif command -v powershell.exe >/dev/null 2>&1; then
  powershell.exe -NoProfile -Command "Expand-Archive -Path '$ZIP' -DestinationPath '$WORK/unzipped' -Force" >/dev/null
elif command -v 7z >/dev/null 2>&1; then
  7z x -y -o"$WORK/unzipped" "$ZIP" >/dev/null
else
  echo "fetch-hev: need unzip or powershell or 7z to extract zip" >&2; exit 1
fi

# zip contains hev-socks5-tunnel/hev-socks5-tunnel.exe + wintun.dll + msys-2.0.dll
HEV_SRC="$(find "$WORK/unzipped" -name 'hev-socks5-tunnel.exe' -print -quit)"
WINTUN_SRC="$(find "$WORK/unzipped" -name 'wintun.dll' -print -quit)"
MSYS_SRC="$(find "$WORK/unzipped" -name 'msys-2.0.dll' -print -quit)"
if [[ -z "$HEV_SRC" || -z "$WINTUN_SRC" || -z "$MSYS_SRC" ]]; then
  echo "fetch-hev: could not find hev-socks5-tunnel.exe, wintun.dll or msys-2.0.dll in zip" >&2
  ls -R "$WORK/unzipped" >&2 || true
  exit 1
fi

for file in "$HEV_SRC" "$WINTUN_SRC" "$MSYS_SRC"; do
  # verify inner hashes
  :
done
# optional extra verify (warn only if mismatch, zip check already guarantees)
for pair in "$HEV_SRC:$HEV_EXE_SHA256" "$WINTUN_SRC:$WINTUN_SHA256" "$MSYS_SRC:$MSYS_SHA256"; do
  src="${pair%%:*}"; want="${pair##*:}"
  got2="$(sha256_of "$src")"
  if [[ "${got2,,}" != "${want,,}" ]]; then
    echo "fetch-hev: warning: $(basename "$src") sha256 $got2 != $want" >&2
  fi
done

OUT_HEV="$BIN/hev-socks5-tunnel-$TRIPLE$EXT"
OUT_WINTUN="$BIN/wintun-$TRIPLE.dll"
OUT_MSYS="$BIN/msys-2.0-$TRIPLE.dll"
cp -f "$HEV_SRC" "$OUT_HEV"
cp -f "$WINTUN_SRC" "$OUT_WINTUN"
cp -f "$MSYS_SRC" "$OUT_MSYS"

# For native Windows dev (triple matches host), also place without suffix so tauri dev / sidecar lookup works
HOST_TRIPLE="$(rustc -vV 2>/dev/null | sed -n 's/^host: //p' || echo x86_64-pc-windows-msvc)"
if [[ "$TRIPLE" == "$HOST_TRIPLE" ]]; then
  cp -f "$HEV_SRC" "$BIN/hev-socks5-tunnel$EXT"
  cp -f "$WINTUN_SRC" "$BIN/wintun.dll"
  cp -f "$MSYS_SRC" "$BIN/msys-2.0.dll"
fi

echo "built $OUT_HEV"
echo "built $OUT_WINTUN"
echo "built $OUT_MSYS"
