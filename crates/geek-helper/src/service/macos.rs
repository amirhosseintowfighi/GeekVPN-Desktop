//! launchd. (SMAppService, the modern way, needs a signed and notarized
//! app; until the Apple certificate is in place, the app installs this
//! daemon once through an administrator prompt.)

use super::{copy_binaries, sh};
use crate::paths::{install_dir, HELPER_FILE};

const LABEL: &str = "com.geekvpn.helper";
const PLIST: &str = "/Library/LaunchDaemons/com.geekvpn.helper.plist";

pub fn plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array><string>{}</string><string>run</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardErrorPath</key><string>/Library/Logs/GeekVPN-helper.log</string>
</dict>
</plist>
"#,
        install_dir().join(HELPER_FILE).display()
    )
}

pub fn install() -> Result<(), String> {
    let _ = sh("launchctl", &["bootout", &format!("system/{LABEL}")]);
    copy_binaries()?;
    std::fs::write(PLIST, plist()).map_err(|e| format!("{PLIST}: {e}"))?;
    sh("launchctl", &["bootstrap", "system", PLIST])
}

pub fn uninstall() -> Result<(), String> {
    let _ = sh("launchctl", &["bootout", &format!("system/{LABEL}")]);
    let _ = std::fs::remove_file(PLIST);
    let _ = std::fs::remove_dir_all(install_dir());
    Ok(())
}
