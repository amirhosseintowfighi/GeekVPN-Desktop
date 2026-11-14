//! «تنظیمات پیشرفته»: what helps a connection through filtering, applied
//! the same way to the running config and to the delay test, so a test
//! says what the connection will do.
//!
//! - Fragment: the TLS ClientHello split into small pieces, sent apart, so
//!   a filter that reads the SNI from the first packet sees no name. Xray
//!   does it in a `freedom` outbound the server's outbound dials through.
//! - Mux: several connections over one (Mux.Cool). Off by default: some
//!   servers do not accept it, and it never applies to XTLS Vision, XHTTP
//!   or gRPC, which have their own or forbid it.
//! - DNS: the resolver names are looked up with, through the tunnel.
//! - IPv6: refused in TUN mode, so nothing leaves beside an IPv4 tunnel.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tuning {
    pub fragment: bool,
    /// Bytes per piece, `"100-200"` (a range Xray picks from) or `"50"`.
    pub fragment_length: String,
    /// Milliseconds between pieces, the same form.
    pub fragment_interval: String,
    pub mux: bool,
    /// The resolver's IP address (DNS over HTTPS to it, through the tunnel).
    pub dns: String,
    pub block_ipv6: bool,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            fragment: false,
            fragment_length: "100-200".into(),
            fragment_interval: "10-20".into(),
            mux: false,
            dns: DEFAULT_DNS.into(),
            block_ipv6: false,
        }
    }
}

pub const DEFAULT_DNS: &str = "1.1.1.1";

/// `"100-200"` or `"50"`: whole numbers, low ≤ high, within `max`.
pub fn range_ok(s: &str, max: u32) -> bool {
    let parse = |x: &str| x.trim().parse::<u32>().ok().filter(|n| *n >= 1 && *n <= max);
    match s.split_once('-') {
        Some((a, b)) => matches!((parse(a), parse(b)), (Some(a), Some(b)) if a <= b),
        None => parse(s).is_some(),
    }
}

impl Tuning {
    /// The settings as saved, with anything malformed put back to its
    /// default (a hand-edited file must not break connecting).
    pub fn cleaned(mut self) -> Self {
        let d = Self::default();
        if !range_ok(&self.fragment_length, 2000) {
            self.fragment_length = d.fragment_length;
        }
        if !range_ok(&self.fragment_interval, 1000) {
            self.fragment_interval = d.fragment_interval;
        }
        if self.dns.trim().parse::<IpAddr>().is_err() {
            self.dns = d.dns;
        }
        self.dns = self.dns.trim().to_string();
        self
    }

    /// Fragment and Mux on the outbound tagged `proxy`.
    pub fn apply(&self, config: &mut Value) {
        let Some(outs) = config["outbounds"].as_array_mut() else { return };
        let mut needs_fragment = false;
        for o in outs.iter_mut().filter(|o| o["tag"] == "proxy") {
            let security = o["streamSettings"]["security"].as_str().unwrap_or_default().to_string();
            let network = o["streamSettings"]["network"].as_str().unwrap_or_default().to_string();
            if self.fragment && matches!(security.as_str(), "tls" | "reality") {
                o["streamSettings"]["sockopt"]["dialerProxy"] = json!("fragment");
                needs_fragment = true;
            }
            let vision = o["settings"]["vnext"][0]["users"][0]["flow"].as_str().is_some_and(|f| !f.is_empty());
            if self.mux && !vision && !matches!(network.as_str(), "xhttp" | "grpc") {
                o["mux"] = json!({ "enabled": true, "concurrency": 8, "xudpConcurrency": 16, "xudpProxyUDP443": "reject" });
            }
        }
        if needs_fragment {
            outs.push(json!({
                "tag": "fragment",
                "protocol": "freedom",
                "settings": { "fragment": { "packets": "tlshello", "length": self.fragment_length, "interval": self.fragment_interval } },
                "streamSettings": { "sockopt": { "tcpNoDelay": true } },
            }));
        }
    }

    /// Windows' TUN (hev) hands the core every packet, DNS included, and the
    /// system's resolver is often the router, which is not reachable through
    /// the server. Port 53 goes to Xray's own resolver instead, which asks
    /// `dns` over HTTPS through the tunnel.
    pub fn with_dns_out(&self, config: &mut Value) {
        config["dns"] = json!({ "servers": [format!("https://{}/dns-query", self.dns)], "tag": "dns-internal", "queryStrategy": if self.block_ipv6 { "UseIPv4" } else { "UseIP" } });
        if let Some(outs) = config["outbounds"].as_array_mut() {
            outs.push(json!({ "tag": "dns-out", "protocol": "dns", "settings": {} }));
        }
        if let Some(rules) = config["routing"]["rules"].as_array_mut() {
            let mut ours = vec![
                json!({ "type": "field", "inboundTag": ["dns-internal"], "outboundTag": "proxy" }),
                json!({ "type": "field", "port": "53", "outboundTag": "dns-out" }),
            ];
            if self.block_ipv6 {
                ours.push(json!({ "type": "field", "ip": ["::/0"], "outboundTag": "block" }));
            }
            ours.append(rules);
            *rules = ours;
        }
        if self.block_ipv6 {
            if let Some(outs) = config["outbounds"].as_array_mut() {
                if !outs.iter().any(|o| o["tag"] == "block") {
                    outs.push(json!({ "tag": "block", "protocol": "blackhole", "settings": {} }));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::Server;

    const TLS_WS: &str = "vless://b831381d-6324-4d53-ad4f-8cda48b30811@cdn.example.com:443?encryption=none&security=tls&sni=cdn.example.com&type=ws&host=cdn.example.com&path=%2Fws#a";
    const VISION: &str = "vless://b831381d-6324-4d53-ad4f-8cda48b30811@r.example.com:443?encryption=none&security=reality&sni=www.example.com&fp=chrome&pbk=abc&sid=01&type=tcp&flow=xtls-rprx-vision#b";
    const PLAIN: &str = "vless://b831381d-6324-4d53-ad4f-8cda48b30811@p.example.com:80?encryption=none&security=none&type=ws&path=%2F#c";

    fn cfg(link: &str) -> Value {
        crate::delay_config(&Server::parse(link).unwrap(), None)
    }

    #[test]
    fn fragment_only_where_there_is_a_client_hello() {
        let t = Tuning { fragment: true, ..Tuning::default() };
        let mut c = cfg(TLS_WS);
        t.apply(&mut c);
        assert_eq!(c["outbounds"][0]["streamSettings"]["sockopt"]["dialerProxy"], "fragment");
        assert_eq!(c["outbounds"][1]["settings"]["fragment"]["packets"], "tlshello");
        let mut p = cfg(PLAIN);
        t.apply(&mut p);
        assert_eq!(p["outbounds"].as_array().unwrap().len(), 1, "no TLS, nothing to split");
    }

    #[test]
    fn mux_never_with_vision() {
        let t = Tuning { mux: true, ..Tuning::default() };
        let mut v = cfg(VISION);
        t.apply(&mut v);
        assert!(v["outbounds"][0]["mux"].is_null());
        let mut w = cfg(TLS_WS);
        t.apply(&mut w);
        assert_eq!(w["outbounds"][0]["mux"]["enabled"], true);
        let mut off = cfg(TLS_WS);
        Tuning::default().apply(&mut off);
        assert_eq!(off, cfg(TLS_WS), "defaults change nothing");
    }

    #[test]
    fn bad_values_fall_back() {
        let t = Tuning { fragment_length: "300-100".into(), fragment_interval: "x".into(), dns: "dns.google".into(), ..Tuning::default() }.cleaned();
        assert_eq!((t.fragment_length.as_str(), t.fragment_interval.as_str(), t.dns.as_str()), ("100-200", "10-20", "1.1.1.1"));
        assert!(range_ok("50", 2000) && range_ok("10-20", 1000) && !range_ok("0-5", 10));
    }

    #[test]
    fn windows_tun_resolves_through_the_tunnel() {
        let t = Tuning { dns: "8.8.8.8".into(), block_ipv6: true, ..Tuning::default() };
        let up = crate::Upstream { port: 1, username: "u".into(), password: "p".into() };
        let mut c = crate::tun_upstream_config(&Server::parse(TLS_WS).unwrap(), None, &up);
        t.with_dns_out(&mut c);
        assert_eq!(c["dns"]["servers"][0], "https://8.8.8.8/dns-query");
        let r = c["routing"]["rules"].as_array().unwrap();
        assert_eq!(r[0]["inboundTag"][0], "dns-internal");
        assert_eq!(r[1]["outboundTag"], "dns-out");
        assert_eq!(r[2]["outboundTag"], "block");
    }
}
