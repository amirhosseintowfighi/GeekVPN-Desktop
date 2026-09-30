//! systemd.

use super::{copy_binaries, sh};
use crate::paths::{install_dir, HELPER_FILE};

const UNIT: &str = "geekvpn-helper.service";
const UNIT_PATH: &str = "/etc/systemd/system/geekvpn-helper.service";

pub fn unit() -> String {
    format!(
        "[Unit]\n\
         Description=GeekVPN helper (TUN mode and kill switch)\n\
         After=network-pre.target\n\
         Wants=network-pre.target\n\n\
         [Service]\n\
         ExecStart={} run\n\
         Restart=on-failure\n\
         RestartSec=1\n\n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        install_dir().join(HELPER_FILE).display()
    )
}

pub fn install() -> Result<(), String> {
    if std::process::Command::new("systemctl").arg("--version").output().is_err() {
        return Err("systemd is needed for TUN mode".into());
    }
    let _ = sh("systemctl", &["stop", UNIT]);
    copy_binaries()?;
    std::fs::write(UNIT_PATH, unit()).map_err(|e| format!("{UNIT_PATH}: {e}"))?;
    sh("systemctl", &["daemon-reload"])?;
    sh("systemctl", &["enable", "--now", UNIT])
}

pub fn uninstall() -> Result<(), String> {
    let _ = sh("systemctl", &["disable", "--now", UNIT]);
    let _ = std::fs::remove_file(UNIT_PATH);
    let _ = sh("systemctl", &["daemon-reload"]);
    let _ = std::fs::remove_dir_all(install_dir());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn unit_runs_the_installed_copy() {
        assert!(super::unit().contains("ExecStart=/usr/local/lib/geekvpn/geekvpn-helper run\n"));
    }
}
