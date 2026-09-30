#!/bin/sh
# The package manager already runs as root: put geekvpn-helper in place as a
# service now, so TUN mode works without a second password prompt. A
# failure (no systemd, say) must not fail the package; the app offers
# «نصب سرویس» again.
if [ -x /usr/bin/geekvpn-helper ]; then
  /usr/bin/geekvpn-helper install || echo "geekvpn: helper service not installed; TUN mode will offer to install it" >&2
fi
exit 0
