//! The clean-IP scanner's rules, as the Android app has them (`CdnTarget`,
//! `ScanStore`, `CloudflareCheck`, `ScanConfig`): which servers may take a
//! Cloudflare address instead of their own, how results rank, and whether
//! an address is Cloudflare's at all.

use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::link::Server;

/// A server that reaches its origin through Cloudflare: any healthy
/// Cloudflare address carries it, as long as SNI and Host still name the
/// customer's domain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CdnTarget {
    pub sni: String,
    pub host: String,
    pub port: u16,
}

impl CdnTarget {
    /// Transports Cloudflare proxies (an HTTP request underneath), over TLS,
    /// with a domain (not an IP) in both SNI and Host.
    pub fn of(s: &Server) -> Option<Self> {
        if !matches!(s.network.as_str(), "ws" | "grpc" | "xhttp" | "httpupgrade") || s.security != "tls" || s.port == 0 {
            return None;
        }
        let sni = if s.sni.trim().is_empty() { s.address.trim() } else { s.sni.trim() };
        let host = s.host.split(',').next().unwrap_or("").trim();
        let host = if host.is_empty() { sni } else { host };
        (is_domain(sni) && is_domain(host)).then(|| Self { sni: sni.to_lowercase(), host: host.to_lowercase(), port: s.port })
    }

    /// Results are kept per domain and network: `sni|port|network`.
    pub fn result_key(&self, network: &str) -> String {
        format!("{}|{}|{network}", self.sni, self.port)
    }

    /// Whether the domain is on Cloudflare is checked per SNI and Host.
    pub fn verdict_key(&self) -> String {
        format!("{}|{}", self.sni, self.host)
    }
}

/// A DNS name with a dot and letters in its last label; never an IP literal.
pub fn is_domain(v: &str) -> bool {
    if v.len() > 253 || !v.contains('.') {
        return false;
    }
    let labels: Vec<&str> = v.trim_end_matches('.').split('.').collect();
    labels.iter().all(|l| {
        !l.is_empty() && l.len() <= 63 && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !l.starts_with('-') && !l.ends_with('-')
    }) && labels.last().is_some_and(|l| l.chars().any(|c| c.is_ascii_alphabetic()))
}

/// Which server an override belongs to. Not the link's hash: a subscription
/// refresh can reword a link while the server stays the same. The source,
/// name, original address and port together change only when it really does.
pub fn profile_key(source_id: &str, s: &Server) -> String {
    let identity = [source_id, &s.name, &s.address, &s.port.to_string()].join("\u{0}");
    let digest = Sha256::digest(identity.as_bytes());
    digest[..12].iter().map(|b| format!("{b:02x}")).collect()
}

/// One clean address, as the scanner reports it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CleanIp {
    pub ip: String,
    pub port: u16,
    pub ping_ms: i64,
    pub latency_ms: i64,
    pub jitter_ms: f64,
    #[serde(rename = "downloadKBps")]
    pub download_kbps: i64,
    pub colo: Option<String>,
}

impl Default for CleanIp {
    fn default() -> Self {
        Self { ip: String::new(), port: 443, ping_ms: 0, latency_ms: 0, jitter_ms: 0.0, download_kbps: 0, colo: None }
    }
}

/// The best addresses kept per domain and network.
pub const KEEP: usize = 5;
/// Cloudflare's routing and the operators' filtering both move within a day.
pub const STALE_AFTER_MS: i64 = 24 * 60 * 60 * 1000;

/// Lowest latency first; jitter breaks ties (a steady link beats a lucky
/// one), then ping. Duplicates keep their first appearance.
pub fn rank(results: &[CleanIp]) -> Vec<CleanIp> {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<CleanIp> = results.iter().filter(|r| seen.insert(r.ip.clone())).cloned().collect();
    out.sort_by(|a, b| {
        let score = |r: &CleanIp| r.latency_ms as f64 + r.jitter_ms * 2.0;
        score(a).total_cmp(&score(b)).then(a.ping_ms.cmp(&b.ping_ms))
    });
    out
}

/// The Go scanner's config for `target`: 300 addresses at most, done after
/// five clean ones, addresses found before tried first.
pub fn scan_config(target: &CdnTarget, prefer: &[String], download: bool, stop_after: u32) -> Value {
    json!({
        "sni": target.sni,
        "host": target.host,
        "port": target.port,
        "maxIps": 300,
        "stopAfter": stop_after,
        "fingerprint": "chrome",
        "preferIps": prefer,
        "jitter": { "enable": true, "maxMs": 50, "samples": 5, "intervalMs": 200 },
        "download": { "enable": download },
    })
}

/// Cloudflare's IPv4 ranges (the bundled cf-scanner list) as sorted
/// `[start, end]` pairs.
pub struct CfRanges(Vec<(u32, u32)>);

impl CfRanges {
    /// One `a.b.c.d/n` (or bare address) per line; `#` comments and bad
    /// lines are skipped.
    pub fn parse(text: &str) -> Self {
        let mut v: Vec<(u32, u32)> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| {
                let (ip, bits) = l.split_once('/').unwrap_or((l, "32"));
                let bits: u32 = bits.parse().ok().filter(|b| *b <= 32)?;
                let base = u32::from(ip.parse::<Ipv4Addr>().ok()?);
                let size = if bits == 0 { u64::from(u32::MAX) + 1 } else { 1u64 << (32 - bits) };
                let start = (u64::from(base) & !(size - 1)) as u32;
                Some((start, (u64::from(start) + size - 1) as u32))
            })
            .collect();
        v.sort_unstable();
        Self(v)
    }

    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        let ip = u32::from(ip);
        let i = self.0.partition_point(|(start, _)| *start <= ip);
        // A wider range that starts earlier may still cover the address.
        self.0[..i].iter().rev().take(8).any(|(_, end)| ip <= *end)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// 0/8, 10/8, 100.64/10, 127/8, 169.254/16, 172.16/12, 192.168/16: a filtered
/// name is often answered with one of these, which is no verdict.
pub fn is_private(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    a == 0 || a == 10 || a == 127 || (a == 100 && (64..=127).contains(&b)) || (a == 169 && b == 254) || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}

/// Whether a name's addresses are Cloudflare's: `Some(true)` if any public
/// one is, `Some(false)` if none is, `None` without a public address.
pub fn verdict(ranges: &CfRanges, addrs: &[Ipv4Addr]) -> Option<bool> {
    let public: Vec<&Ipv4Addr> = addrs.iter().filter(|a| !is_private(**a)).collect();
    (!public.is_empty()).then(|| public.iter().any(|a| ranges.contains(**a)))
}

/// When the running connection counts as bad enough to move servers
/// («Failover»; the Android app's `FailoverThreshold`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailoverThreshold {
    Off,
    /// Only when the connection stops carrying traffic.
    Lost,
    #[serde(rename = "1000")]
    Ms1000,
    #[default]
    #[serde(rename = "2000")]
    Ms2000,
    #[serde(rename = "3000")]
    Ms3000,
}

impl FailoverThreshold {
    fn millis(self) -> i64 {
        match self {
            Self::Off => -1,
            Self::Lost => 0,
            Self::Ms1000 => 1000,
            Self::Ms2000 => 2000,
            Self::Ms3000 => 3000,
        }
    }
}

/// Decides from the running connection's delay checks when to fail over.
/// Two bad checks in a row fail over; after a failover the next one waits a
/// cooldown, doubled each time it turns bad again without a good check in
/// between (2 minutes up to 30), so a network that is simply down does not
/// retest every few seconds. Clock-free: the caller passes `now`.
pub struct FailoverPolicy {
    threshold: FailoverThreshold,
    bad_in_a_row: u32,
    cooldown_ms: i64,
    not_before: i64,
}

impl FailoverPolicy {
    pub const BAD_IN_A_ROW: u32 = 2;
    pub const BASE_COOLDOWN_MS: i64 = 2 * 60 * 1000;
    pub const MAX_COOLDOWN_MS: i64 = 30 * 60 * 1000;

    pub fn new(threshold: FailoverThreshold) -> Self {
        Self { threshold, bad_in_a_row: 0, cooldown_ms: Self::BASE_COOLDOWN_MS, not_before: 0 }
    }

    /// One check: `delay_ms` > 0 answered, anything else failed. True means
    /// fail over now.
    pub fn on_check(&mut self, delay_ms: i64, now: i64) -> bool {
        if self.threshold == FailoverThreshold::Off {
            return false;
        }
        let limit = self.threshold.millis();
        let bad = delay_ms <= 0 || (limit > 0 && delay_ms > limit);
        if !bad {
            self.bad_in_a_row = 0;
            self.cooldown_ms = Self::BASE_COOLDOWN_MS;
            return false;
        }
        self.bad_in_a_row += 1;
        self.bad_in_a_row >= Self::BAD_IN_A_ROW && now >= self.not_before
    }

    /// The failover ran, whether or not it found a better server.
    pub fn on_failover(&mut self, now: i64) {
        self.bad_in_a_row = 0;
        self.not_before = now + self.cooldown_ms;
        self.cooldown_ms = (self.cooldown_ms * 2).min(Self::MAX_COOLDOWN_MS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(link: &str) -> Server {
        Server::parse(link).unwrap()
    }

    #[test]
    fn only_cdn_transports_over_tls_with_a_domain() {
        let ws = server("vless://u@cdn.example.com:443?type=ws&security=tls&host=Cdn.Example.com&path=%2F#a");
        assert_eq!(CdnTarget::of(&ws), Some(CdnTarget { sni: "cdn.example.com".into(), host: "cdn.example.com".into(), port: 443 }));
        // Written with an IP and the domain in SNI: still a target.
        let ip = server("vless://u@104.16.1.1:8443?type=ws&security=tls&sni=cdn.example.com#b");
        assert_eq!(CdnTarget::of(&ip).unwrap().sni, "cdn.example.com");
        assert!(CdnTarget::of(&server("vless://u@cdn.example.com:443?type=tcp&security=tls#c")).is_none());
        assert!(CdnTarget::of(&server("vless://u@cdn.example.com:80?type=ws&security=none#d")).is_none());
        assert!(CdnTarget::of(&server("vless://u@1.2.3.4:443?type=ws&security=tls#e")).is_none());
        assert!(is_domain("a.b-c.ir") && !is_domain("1.2.3.4") && !is_domain("localhost") && !is_domain("-a.com"));
    }

    #[test]
    fn profile_key_survives_a_reworded_link() {
        let a = server("vless://u@cdn.example.com:443?type=ws&security=tls&path=%2Fa#Germany");
        let b = server("vless://u@cdn.example.com:443?type=ws&security=tls&path=%2Fb#Germany");
        assert_ne!(a.id, b.id);
        assert_eq!(profile_key("geek-1", &a), profile_key("geek-1", &b));
        assert_ne!(profile_key("geek-1", &a), profile_key("geek-2", &a));
    }

    #[test]
    fn ranks_by_latency_and_jitter_and_drops_duplicates() {
        let ip = |ip: &str, lat, jit| CleanIp { ip: ip.into(), latency_ms: lat, jitter_ms: jit, ..Default::default() };
        let r = rank(&[ip("a", 100, 30.0), ip("b", 120, 1.0), ip("a", 90, 0.0), ip("c", 80, 0.0)]);
        assert_eq!(r.iter().map(|x| x.ip.as_str()).collect::<Vec<_>>(), ["c", "b", "a"]);
        let parsed: CleanIp = serde_json::from_str(r#"{"ip":"104.18.1.1","port":443,"pingMs":80,"latencyMs":120,"jitterMs":3.5,"downloadKBps":900,"colo":"FRA"}"#).unwrap();
        assert_eq!((parsed.download_kbps, parsed.colo.as_deref()), (900, Some("FRA")));
    }

    #[test]
    fn cloudflare_ranges_and_verdicts() {
        let r = CfRanges::parse("# comment\n104.16.0.0/13\n1.1.1.0/24\nnot an ip\n172.64.0.0/13\n");
        assert_eq!(r.len(), 3);
        assert!(r.contains("104.18.34.121".parse().unwrap()));
        assert!(!r.contains("8.8.8.8".parse().unwrap()));
        let v = |ips: &[&str]| verdict(&r, &ips.iter().map(|i| i.parse().unwrap()).collect::<Vec<_>>());
        assert_eq!(v(&["104.18.34.121"]), Some(true));
        assert_eq!(v(&["5.6.7.8"]), Some(false));
        // A filtered name answered with a private address: no verdict.
        assert_eq!(v(&["10.10.34.35"]), None);
        let shipped = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/desktop/src-tauri/resources/cfscan/ipv4.txt"));
        if let Ok(text) = shipped {
            let all = CfRanges::parse(&text);
            assert!(all.len() > 1000 && all.contains("104.16.1.1".parse().unwrap()));
        }
    }

    #[test]
    fn failover_needs_two_bad_checks_and_backs_off() {
        let mut p = FailoverPolicy::new(FailoverThreshold::Ms2000);
        assert!(!p.on_check(2500, 0));
        assert!(p.on_check(-1, 1));
        p.on_failover(1);
        // Bad again at once: inside the 2-minute cooldown.
        assert!(!p.on_check(-1, 2) && !p.on_check(-1, 3));
        let t1 = 1 + FailoverPolicy::BASE_COOLDOWN_MS;
        assert!(p.on_check(-1, t1));
        p.on_failover(t1);
        // Still bad: the next cooldown is doubled.
        assert!(!p.on_check(-1, t1 + 1) && !p.on_check(-1, t1 + 2 * FailoverPolicy::BASE_COOLDOWN_MS - 1));
        assert!(p.on_check(-1, t1 + 2 * FailoverPolicy::BASE_COOLDOWN_MS));
        // A good check resets the count.
        assert!(!p.on_check(300, t1 * 10) && !p.on_check(-1, t1 * 10 + 1));
        let mut lost = FailoverPolicy::new(FailoverThreshold::Lost);
        assert!(!lost.on_check(9000, 0) && !lost.on_check(9000, 1));
        let mut off = FailoverPolicy::new(FailoverThreshold::Off);
        assert!(!off.on_check(-1, 0) && !off.on_check(-1, 1));
    }
}
