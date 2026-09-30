#!/bin/sh
# Remove the helper service with the package; an upgrade reinstalls it in
# postinst. Stopping it also takes down the kill switch.
if [ -x /usr/bin/geekvpn-helper ]; then
  /usr/bin/geekvpn-helper uninstall || true
fi
exit 0
