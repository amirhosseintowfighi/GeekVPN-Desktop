#!/usr/bin/env bash
# The geo files Xray's routing reads (geosite:category-ir, geoip:ir,
# geoip:private), from the same source as the Android app's AAR.
#
# Versions are pinned in geo.lock.json (tag + sha256). Without that file
# the script still works but warns and only checks size — CI must have the
# lock file. Update it: scripts/fetch-geo.sh --update-lock
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="$ROOT/apps/desktop/src-tauri/resources/geo"
LOCK="$ROOT/scripts/geo.lock.json"
mkdir -p "$DIR"

update_lock=false
[[ "${1:-}" == --update-lock ]] && update_lock=true

# Pinned versions (update via --update-lock). Fallback to latest if no lock.
if [[ -f "$LOCK" && "$update_lock" == false ]]; then
  base=$(jq -r '.v2ray_rules_dat.url' "$LOCK" 2>/dev/null || echo "")
  geosite_tag=$(jq -r '.v2ray_rules_dat.tag' "$LOCK" 2>/dev/null || echo "")
  geoip_private_url=$(jq -r '.geoip_private.url' "$LOCK" 2>/dev/null || echo "")
  if [[ -n "$base" && -n "$geoip_private_url" ]]; then
    echo "using pinned geo from $LOCK ($geosite_tag)"
    curl -fsSL "$base/geoip.dat" -o "$DIR/geoip.dat"
    curl -fsSL "$base/geosite.dat" -o "$DIR/geosite.dat"
    curl -fsSL "$geoip_private_url" -o "$DIR/geoip-only-cn-private.dat"
    # Verify sha256 if present in lock file
    if command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1; then
      for f in geoip.dat geosite.dat geoip-only-cn-private.dat; do
        expected=$(jq -r --arg k "$f" '.sha256[$k] // empty' "$LOCK" 2>/dev/null || echo "")
        if [[ -n "$expected" ]]; then
          if command -v sha256sum >/dev/null 2>&1; then
            actual=$(sha256sum "$DIR/$f" | cut -d' ' -f1)
          else
            actual=$(shasum -a 256 "$DIR/$f" | cut -d' ' -f1)
          fi
          if [[ "$actual" != "$expected" ]]; then
            echo "sha256 mismatch for $f: expected $expected, got $actual" >&2
            exit 1
          fi
          echo "sha256 ok: $f"
        fi
      done
    fi
  else
    echo "warning: geo.lock.json malformed, falling back to latest" >&2
    base=https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download
    curl -fsSL "$base/geoip.dat" -o "$DIR/geoip.dat"
    curl -fsSL "$base/geosite.dat" -o "$DIR/geosite.dat"
    curl -fsSL https://raw.githubusercontent.com/Loyalsoldier/geoip/release/geoip-only-cn-private.dat \
      -o "$DIR/geoip-only-cn-private.dat"
  fi
else
  if [[ ! -f "$LOCK" ]]; then
    echo "warning: $LOCK not found — using latest without hash verification (not for CI)" >&2
  fi
  base=https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download
  curl -fsSL "$base/geoip.dat" -o "$DIR/geoip.dat"
  curl -fsSL "$base/geosite.dat" -o "$DIR/geosite.dat"
  curl -fsSL https://raw.githubusercontent.com/Loyalsoldier/geoip/release/geoip-only-cn-private.dat \
    -o "$DIR/geoip-only-cn-private.dat"
  if [[ "$update_lock" == true ]]; then
    tag=$(curl -fsSL https://api.github.com/repos/Loyalsoldier/v2ray-rules-dat/releases/latest | jq -r '.tag_name' 2>/dev/null || echo "latest")
    if command -v sha256sum >/dev/null 2>&1; then
      sha_geoip=$(sha256sum "$DIR/geoip.dat" | cut -d' ' -f1)
      sha_geosite=$(sha256sum "$DIR/geosite.dat" | cut -d' ' -f1)
      sha_private=$(sha256sum "$DIR/geoip-only-cn-private.dat" | cut -d' ' -f1)
    else
      sha_geoip=$(shasum -a 256 "$DIR/geoip.dat" | cut -d' ' -f1)
      sha_geosite=$(shasum -a 256 "$DIR/geosite.dat" | cut -d' ' -f1)
      sha_private=$(shasum -a 256 "$DIR/geoip-only-cn-private.dat" | cut -d' ' -f1)
    fi
    jq -n \
      --arg tag "$tag" \
      --arg base "$base" \
      --arg priv_url "https://raw.githubusercontent.com/Loyalsoldier/geoip/release/geoip-only-cn-private.dat" \
      --arg s1 "$sha_geoip" --arg s2 "$sha_geosite" --arg s3 "$sha_private" \
      '{ v2ray_rules_dat: { tag: $tag, url: $base }, geoip_private: { url: $priv_url }, sha256: { "geoip.dat": $s1, "geosite.dat": $s2, "geoip-only-cn-private.dat": $s3 } }' > "$LOCK"
    echo "wrote $LOCK ($tag)"
  fi
fi

for f in geoip.dat geosite.dat geoip-only-cn-private.dat; do
  [[ $(stat -c %s "$DIR/$f" 2>/dev/null || stat -f %z "$DIR/$f") -gt 10000 ]] || { echo "$f too small" >&2; exit 1; }
done
echo "geo data in $DIR"
