//! «اشتراک در شبکه‌ی محلی»: the running tunnel offered to the phone or TV
//! on the same Wi-Fi, through one `mixed` inbound (HTTP and SOCKS5 on the
//! same port) on every address, locked with a username and password unless
//! the customer turns that off.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const DEFAULT_LAN_PORT: u16 = 10820;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LanShare {
    pub on: bool,
    pub port: u16,
    /// Ask for `user`/`pass`. Android's own Wi-Fi proxy setting cannot send
    /// them, so it can be turned off; then anyone on the network may use it.
    pub auth: bool,
    pub user: String,
    pub pass: String,
}

impl Default for LanShare {
    fn default() -> Self {
        Self { on: false, port: DEFAULT_LAN_PORT, auth: true, user: String::new(), pass: String::new() }
    }
}

impl LanShare {
    /// Whether the settings can be used: a non-privileged port and, with
    /// auth, a username and password of plain characters.
    pub fn valid(&self) -> bool {
        let plain = |s: &str| !s.is_empty() && s.len() <= 32 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        self.port >= 1024 && (!self.auth || (plain(&self.user) && plain(&self.pass)))
    }

    /// The inbound, added to a running config's.
    pub fn apply(&self, config: &mut Value) {
        if !self.on || !self.valid() {
            return;
        }
        let settings = if self.auth {
            json!({ "udp": true, "auth": "password", "accounts": [{ "user": self.user, "pass": self.pass }] })
        } else {
            json!({ "udp": true, "auth": "noauth" })
        };
        let inbound = json!({
            "tag": "lan", "listen": "0.0.0.0", "port": self.port, "protocol": "mixed", "settings": settings,
            "sniffing": { "enabled": true, "destOverride": ["http", "tls", "quic"], "routeOnly": true },
        });
        if let Some(list) = config["inbounds"].as_array_mut() {
            list.push(inbound);
        }
    }

    /// A `socks://` link for v2rayNG / v2rayN / Shadowrocket (the
    /// credentials base64'd in the user part, as v2rayN writes it).
    pub fn link(&self, ip: &str) -> String {
        use base64::Engine;
        let user = if self.auth { format!("{}@", base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", self.user, self.pass))) } else { String::new() };
        format!("socks://{user}{ip}:{}#GeekVPN-LAN", self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> LanShare {
        LanShare { on: true, user: "geek".into(), pass: "s3cret".into(), ..LanShare::default() }
    }

    #[test]
    fn adds_one_locked_mixed_inbound() {
        let mut config = json!({ "inbounds": [{ "tag": "socks" }] });
        on().apply(&mut config);
        let lan = &config["inbounds"][1];
        assert_eq!(lan["protocol"], "mixed");
        assert_eq!(lan["listen"], "0.0.0.0");
        assert_eq!(lan["port"], DEFAULT_LAN_PORT);
        assert_eq!(lan["settings"]["accounts"][0]["user"], "geek");
    }

    #[test]
    fn off_or_invalid_adds_nothing() {
        for s in [LanShare::default(), LanShare { pass: String::new(), ..on() }, LanShare { port: 80, ..on() }, LanShare { user: "a b".into(), ..on() }] {
            let mut config = json!({ "inbounds": [] });
            s.apply(&mut config);
            assert_eq!(config["inbounds"].as_array().unwrap().len(), 0, "{s:?}");
        }
        let mut open = json!({ "inbounds": [] });
        LanShare { auth: false, user: String::new(), pass: String::new(), ..on() }.apply(&mut open);
        assert_eq!(open["inbounds"][0]["settings"]["auth"], "noauth");
    }

    #[test]
    fn links_as_v2rayn_writes_them() {
        assert_eq!(on().link("192.168.1.5"), "socks://Z2VlazpzM2NyZXQ=@192.168.1.5:10820#GeekVPN-LAN");
        assert_eq!(LanShare { auth: false, ..on() }.link("10.0.0.2"), "socks://10.0.0.2:10820#GeekVPN-LAN");
    }
}
