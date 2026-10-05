//! pf: our rules live in the anchor `com.apple/250.GeekVPN`. macOS's own
//! /etc/pf.conf evaluates `com.apple/*`, so the anchor takes effect without
//! editing the system's ruleset. pf is enabled with a reference (`-E`),
//! and that reference alone is dropped on release (`-X`), so a pf the user
//! or another app enabled stays on.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use geek_ipc::KillSwitch;

use super::{LAN_V4, LAN_V6};

const ANCHOR: &str = "com.apple/250.GeekVPN";

#[derive(Default)]
pub struct Firewall {
    token: Option<String>,
}

pub fn ruleset(ks: &KillSwitch, tun: Option<&str>) -> String {
    let mut r = vec!["pass out quick on lo0 all".to_string()];
    if let Some(t) = tun {
        r.push(format!("pass out quick on {t} all"));
    }
    // sing-box runs as root; pf has no way to name a process.
    r.push("pass out quick all user root".into());
    r.push("pass out quick proto udp from any port 68 to any port 67".into());
    r.push("pass out quick inet6 proto icmp6 all icmp6-type { routersol, neighbrsol, neighbradv }".into());
    if ks.allow_lan {
        r.push(format!("pass out quick inet to {{ {} }}", LAN_V4.join(" ")));
        r.push(format!("pass out quick inet6 to {{ {} }}", LAN_V6.join(" ")));
    }
    r.push("block drop out all".into());
    r.join("\n") + "\n"
}

fn pfctl(args: &[&str], stdin: Option<&str>) -> Result<String, String> {
    let mut child = Command::new("pfctl")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("pfctl: {e}"))?;
    if let Some(s) = stdin {
        child.stdin.take().ok_or("pfctl: no stdin")?.write_all(s.as_bytes()).map_err(|e| format!("pfctl: {e}"))?;
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().map_err(|e| format!("pfctl: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if out.status.success() {
        Ok(text)
    } else {
        Err(format!("pfctl: {}", text.trim()))
    }
}

impl Firewall {
    #[allow(dead_code)]
    pub fn engage(&mut self, ks: &KillSwitch, tun: Option<&str>, _engine: &Path) -> Result<(), String> {
        self.engage_with_apps(ks, tun, _engine, &[])
    }
    pub fn engage_with_apps(&mut self, ks: &KillSwitch, tun: Option<&str>, _engine: &Path, _extra: &[String]) -> Result<(), String> {
        pfctl(&["-a", ANCHOR, "-f", "-"], Some(&ruleset(ks, tun)))?;
        if self.token.is_none() {
            let out = pfctl(&["-E"], None)?;
            self.token = out.lines().find_map(|l| l.strip_prefix("Token : ").map(|t| t.trim().to_string()));
        }
        Ok(())
    }

    pub fn release(&mut self) -> Result<(), String> {
        let flushed = pfctl(&["-a", ANCHOR, "-F", "all"], None).map(|_| ());
        if let Some(t) = self.token.take() {
            let _ = pfctl(&["-X", &t], None);
        }
        flushed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_last_and_passes_the_tunnel() {
        let r = ruleset(&KillSwitch { allow_lan: true, strict: false }, Some("utun7"));
        assert!(r.contains("pass out quick on utun7 all"));
        assert!(r.trim_end().ends_with("block drop out all"));
        assert!(r.contains("10.0.0.0/8 172.16.0.0/12"));
    }
}
