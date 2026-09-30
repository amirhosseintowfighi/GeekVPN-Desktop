//! The technical half of «گزارش مشکل»: the customer's own words, then facts
//! about the computer and the connection, then the newest lines of the
//! engine's log, with everything that could point at a server or a person
//! taken out. Pure, so what leaves the computer is tested (the Android app's
//! `ProblemReport`).

use std::sync::LazyLock;

use regex::Regex;

/// The ticket API takes 4000 characters.
pub const MAX_CHARS: usize = 3_900;
const MAX_LINE: usize = 300;
const LOG_HEADER: &str = "--- log ---\n";

#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub app_version: String,
    /// `windows 11 (x86_64)`.
    pub os: String,
    /// `wifi`, `ethernet`, …
    pub network: String,
    /// `proxy` or `tun`, and the kill switch.
    pub mode: String,
    pub route: String,
    pub auto_server: bool,
    pub connected: bool,
    /// `geekcore 0.1.0, xray 25.9.11, sing-box 1.14.2`.
    pub core: String,
    /// `vless ws tls :443 (direct)`: shape and port only, never the address.
    pub config: Option<String>,
    pub last_failure: Option<String>,
    pub last_failure_minutes_ago: Option<u64>,
}

pub fn compose(description: &str, f: &Facts, log: &[String]) -> String {
    let words = description.trim();
    let mut head = if words.is_empty() { String::new() } else { format!("{words}\n\n") };
    head += "--- report ---\n";
    head += &format!("app: GeekVPN desktop {}\n", f.app_version);
    head += &format!("os: {}\n", f.os);
    head += &format!("network: {}\n", f.network);
    head += &format!(
        "mode: {}, route: {}, auto server: {}\n",
        f.mode,
        f.route,
        if f.auto_server { "on" } else { "off" }
    );
    head += &format!("connected: {}\n", if f.connected { "yes" } else { "no" });
    head += &format!("core: {}\n", f.core);
    if let Some(c) = &f.config {
        head += &format!("config: {}\n", redact(c));
    }
    if let Some(failure) = &f.last_failure {
        let ago = f
            .last_failure_minutes_ago
            .map(|m| format!(" ({m} min ago)"))
            .unwrap_or_default();
        head += &format!("last failure{ago}: {}\n", redact(failure));
    }
    let head_len = head.chars().count();
    if head_len + LOG_HEADER.len() >= MAX_CHARS {
        return head.chars().take(MAX_CHARS).collect();
    }
    let room = MAX_CHARS - head_len - LOG_HEADER.len();
    let mut kept = std::collections::VecDeque::new();
    let mut used = 0;
    for line in log.iter().rev() {
        let clean: String = redact(line).chars().take(MAX_LINE).collect();
        let n = clean.chars().count() + 1;
        if used + n > room {
            break;
        }
        used += n;
        kept.push_front(clean);
    }
    if kept.is_empty() {
        head.trim_end().to_string()
    } else {
        head + LOG_HEADER + &Vec::from(kept).join("\n")
    }
}

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).expect("valid pattern"));
    };
}

re!(URL, r"\b[a-zA-Z][a-zA-Z0-9+.\-]*://\S+");
re!(EMAIL, r"\b[\w.+\-]+@[\w\-]+(\.[\w\-]+)+\b");
re!(
    UUID,
    r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b"
);
re!(BEARER, r"Bearer\s+\S+");
// Four or more groups, or a "::": a clock time ("12:34:56") has three.
re!(
    IPV6,
    r"\b(?:[0-9a-fA-F]{1,4}:){3,7}[0-9a-fA-F]{1,4}\b|[0-9a-fA-F:]*::[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4})*"
);
re!(IPV4, r"\b(?:\d{1,3}\.){3}\d{1,3}\b");
re!(
    HOST,
    r"\b(?:[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}\b"
);
// Base64 or hex runs of 32+ characters: keys, short IDs, tokens.
re!(SECRET, r"[A-Za-z0-9+/_\-]{32,}={0,2}");

/// File names in the engine's messages: `geoip.dat`, `main.go`.
const CODE_SUFFIXES: &[&str] = &[
    "go", "rs", "dat", "json", "exe", "dll", "so", "dylib", "srs",
];

/// Links, UUIDs, IP addresses, host names, e-mail addresses and long
/// key-like strings become placeholders.
pub fn redact(text: &str) -> String {
    let out = URL.replace_all(text, "<link>");
    let out = EMAIL.replace_all(&out, "<email>");
    let out = UUID.replace_all(&out, "<uuid>");
    let out = BEARER.replace_all(&out, "Bearer <token>");
    let out = IPV6.replace_all(&out, "<ip>");
    let out = IPV4.replace_all(&out, "<ip>");
    let out = HOST.replace_all(&out, |c: &regex::Captures| {
        let m = &c[0];
        // Host names in configs and logs are lower case; `Tunnel.connect`
        // or `geoip.dat` are code.
        let code = CODE_SUFFIXES.contains(&m.rsplit('.').next().unwrap_or(""))
            || m.chars().any(|ch| ch.is_ascii_uppercase());
        if code {
            m.to_string()
        } else {
            "<host>".to_string()
        }
    });
    SECRET.replace_all(&out, "<secret>").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_that_points_at_a_server_or_a_person_leaves() {
        let line = "2026/09/30 12:34:56 [Warning] failed to dial vless://b831381d-6324-4d53-ad4f-8cda48b30811@de1.example.com:443?type=ws \
                    tcp:de1.example.com:443 via 104.16.12.3 and [2606:4700::6810:c03] mail me@x.org key=AbCdEfGhIjKlMnOpQrStUvWxYz0123456789abcd";
        let r = redact(line);
        for leaked in [
            "b831381d",
            "de1.example.com",
            "104.16.12.3",
            "2606:4700",
            "me@x.org",
            "AbCdEfGh",
        ] {
            assert!(!r.contains(leaked), "{leaked} in {r}");
        }
        assert!(
            r.contains("12:34:56"),
            "a clock time is not an address: {r}"
        );
        assert!(r.contains("<link>") && r.contains("<ip>") && r.contains("<host>"));
        assert_eq!(
            redact("uuid 8cda48b3-6324-4d53-ad4f-8cda48b30811"),
            "uuid <uuid>"
        );
        assert_eq!(
            redact("open geoip.dat: not found"),
            "open geoip.dat: not found"
        );
        assert_eq!(
            redact("Authorization: Bearer eyJhbGciOi.x.y"),
            "Authorization: Bearer <token>"
        );
    }

    #[test]
    fn fits_the_ticket_and_keeps_the_newest_lines() {
        let facts = Facts {
            app_version: "0.1.0".into(),
            os: "linux (x86_64)".into(),
            network: "wifi".into(),
            mode: "tun, kill switch".into(),
            route: "smart".into(),
            core: "xray 25.9".into(),
            config: Some("vless ws tls :443 (direct)".into()),
            last_failure: Some("failed to dial 1.2.3.4:443".into()),
            last_failure_minutes_ago: Some(3),
            ..Default::default()
        };
        let log: Vec<String> = (0..500)
            .map(|i| format!("line {i} {}", "warn ".repeat(8)))
            .collect();
        let r = compose("  از صبح وصل نمی‌شود.  ", &facts, &log);
        assert!(r.chars().count() <= MAX_CHARS);
        assert!(r.starts_with("از صبح وصل نمی‌شود.\n\n--- report ---\n"));
        assert!(r.contains("last failure (3 min ago): failed to dial <ip>:443"));
        assert!(r.ends_with(log[499].as_str()), "the newest line is kept");
        assert!(!r.contains("line 0 "), "the oldest are dropped");

        let bare = compose("x", &Facts::default(), &[]);
        assert!(!bare.contains("--- log ---"));
    }
}
