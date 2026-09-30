# Xray core patches

Copied from GeekVPN-Android (`scripts/xray-patches/`), so both clients run the
same core. `scripts/build-geekcore.sh` copies `github.com/xtls/xray-core` at
the version `core/geekcore/go.mod` pins, applies every `*.patch` here, runs
the `*_test.go` files here, and builds against that copy. A patch that no
longer applies fails the build.

## `allow-plain-vless.patch`

Xray refuses a VLESS outbound with no TLS/REALITY and no VLESS encryption to a
public address. GeekVPN's tunnel services still hand out such links (ws,
`security=none`, behind Cloudflare's plain-HTTP ports), so without this patch
they cannot connect. Plain Trojan stays refused. Drop it once every tunnel
service uses TLS (or VLESS encryption).

Android's second patch (`connection-count.patch`) is not needed here: the
desktop app counts connections from the core's own stats.
