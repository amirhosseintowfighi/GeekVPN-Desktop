#!/usr/bin/env bash
# The geo files Xray's routing reads (geosite:category-ir, geoip:ir,
# geoip:private), from the same source as the Android app's AAR.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="$ROOT/apps/desktop/src-tauri/resources/geo"
mkdir -p "$DIR"
base=https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download
curl -fsSL "$base/geoip.dat" -o "$DIR/geoip.dat"
curl -fsSL "$base/geosite.dat" -o "$DIR/geosite.dat"
curl -fsSL https://raw.githubusercontent.com/Loyalsoldier/geoip/release/geoip-only-cn-private.dat \
  -o "$DIR/geoip-only-cn-private.dat"
for f in geoip.dat geosite.dat geoip-only-cn-private.dat; do
  [[ $(stat -c %s "$DIR/$f" 2>/dev/null || stat -f %z "$DIR/$f") -gt 10000 ]] || { echo "$f too small" >&2; exit 1; }
done
echo "geo data in $DIR"
