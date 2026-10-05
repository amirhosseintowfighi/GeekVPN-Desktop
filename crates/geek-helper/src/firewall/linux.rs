//! nftables: one table of our own, `inet geekvpn`, with an output chain
//! that drops by default. Replaced in one `nft -f` transaction, removed by
//! deleting the table; nobody else's rules are touched.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use geek_config::LINUX_MARK;
use geek_ipc::KillSwitch;

use super::{LAN_V4, LAN_V6};
use crate::paths::TUN_NAME;

#[derive(Default)]
pub struct Firewall;

pub fn ruleset(ks: &KillSwitch) -> String {
    let mut rules = vec![
        r#"oifname "lo" accept"#.to_string(),
        format!(r#"oifname "{TUN_NAME}" accept"#),
        // sing-box marks every socket it opens (route.default_mark).
        format!("meta mark {LINUX_MARK:#x} accept"),
        "udp sport 68 udp dport 67 accept".into(),
        "udp dport 547 accept".into(),
        // Without neighbour discovery, IPv6 on the LAN stops working even
        // for the tunnel's own packets.
        "icmpv6 type { nd-router-solicit, nd-neighbor-solicit, nd-neighbor-advert } accept".into(),
    ];
    if ks.allow_lan {
        rules.push(format!("ip daddr {{ {} }} accept", LAN_V4.join(", ")));
        rules.push(format!("ip6 daddr {{ {} }} accept", LAN_V6.join(", ")));
    }
    let body: String = rules.iter().map(|r| format!("    {r}\n")).collect();
    // `table` + `delete table` first: replaces an existing table, and does
    // not fail when there is none.
    format!(
        "table inet geekvpn\ndelete table inet geekvpn\ntable inet geekvpn {{\n  chain output {{\n    type filter hook output priority 0; policy drop;\n{body}  }}\n}}\n"
    )
}

fn nft(script: &str) -> Result<(), String> {
    let mut child = Command::new("nft")
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("nft: {e} (is nftables installed?)"))?;
    child.stdin.take().ok_or("nft: no stdin")?.write_all(script.as_bytes()).map_err(|e| format!("nft: {e}"))?;
    let out = child.wait_with_output().map_err(|e| format!("nft: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("nft: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

impl Firewall {
    pub fn engage(&mut self, ks: &KillSwitch, _tun: Option<&str>, _engine: &Path) -> Result<(), String> {
        self.engage_with_apps(ks, _tun, _engine, &[])
    }
    pub fn engage_with_apps(&mut self, ks: &KillSwitch, _tun: Option<&str>, _engine: &Path, _extra: &[String]) -> Result<(), String> {
        nft(&ruleset(ks))
    }

    pub fn release(&mut self) -> Result<(), String> {
        nft("table inet geekvpn\ndelete table inet geekvpn\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_all_but_the_tunnel_and_what_the_user_allowed() {
        let strict = ruleset(&KillSwitch { allow_lan: false, strict: true });
        assert!(strict.contains("policy drop;"));
        assert!(strict.contains(r#"oifname "geekvpn0" accept"#));
        assert!(strict.contains("meta mark 0x2024 accept"));
        assert!(!strict.contains("192.168.0.0/16"));
        let lan = ruleset(&KillSwitch { allow_lan: true, strict: false });
        assert!(lan.contains("ip daddr { 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, 169.254.0.0/16, 224.0.0.0/4 } accept"));
        // nft itself checks the syntax (-c parses, applies nothing), when it
        // is installed and we are root: even a check needs CAP_NET_ADMIN,
        // and without it older nft (Ubuntu 22.04's) fails silently, which
        // could not be told apart from a syntax error.
        let root = Command::new("id").arg("-u").output().is_ok_and(|o| o.stdout.trim_ascii() == b"0");
        let nft = Command::new("nft").args(["-c", "-f", "-"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
        if let (true, Ok(mut c)) = (root, nft) {
            c.stdin.take().unwrap().write_all(lan.as_bytes()).unwrap();
            let out = c.wait_with_output().unwrap();
            let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
            assert!(out.status.success(), "nft -c: {:?} {said}", out.status);
        }
    }
}
