//! «قوانین دامنه و IP»: the customer's own routing rules, one site or
//! address each, sent direct, through the VPN, or blocked. Both engines get
//! the same rules: Xray in proxy mode, sing-box in TUN mode.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Direct,
    Proxy,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomRule {
    /// As the customer typed it, normalised by [`parse_rule`].
    pub value: String,
    pub action: RuleAction,
}

/// What a rule matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// The domain and every subdomain: `example.com`, `*.example.com`.
    Suffix(String),
    /// Only this exact name: `full:example.com`.
    Full(String),
    /// Any name containing the word: `keyword:ads`.
    Keyword(String),
    /// An address or a range: `1.2.3.4`, `10.0.0.0/8`, `2001:db8::/32`.
    Cidr(String),
}

pub const MAX_RULES: usize = 200;

/// Reads one rule the way the customer may type it, or says in Persian why
/// it is not one.
pub fn parse_rule(input: &str) -> Result<Target, String> {
    let v = input.trim().to_ascii_lowercase();
    if v.is_empty() {
        return Err("دامنه یا IP را بنویس.".into());
    }
    if let Some(rest) = v.strip_prefix("full:") {
        return domain(rest).map(Target::Full);
    }
    if let Some(rest) = v.strip_prefix("keyword:") {
        let word = rest.trim();
        return if !word.is_empty()
            && word.len() <= 64
            && word
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        {
            Ok(Target::Keyword(word.into()))
        } else {
            Err("کلمه‌ی کلیدی فقط حروف انگلیسی، عدد، خط تیره و نقطه می‌گیرد.".into())
        };
    }
    if let Some(cidr) = cidr(&v) {
        return Ok(Target::Cidr(cidr));
    }
    let name = v
        .strip_prefix("domain:")
        .or_else(|| v.strip_prefix("*."))
        .unwrap_or(&v);
    // A pasted address: keep only the host.
    let name = name.split("://").last().unwrap_or(name);
    let name = name.split(['/', '?', '#']).next().unwrap_or(name);
    domain(name).map(Target::Suffix)
}

fn domain(v: &str) -> Result<String, String> {
    let v = v.trim().trim_end_matches('.');
    let labels: Vec<&str> = v.split('.').collect();
    let ok = v.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && labels
            .last()
            .is_some_and(|tld| tld.chars().any(|c| c.is_ascii_alphabetic()));
    if ok {
        Ok(v.into())
    } else {
        Err("این دامنه یا IP درست نیست. مثل example.com یا 1.2.3.0/24 بنویس.".into())
    }
}

fn cidr(v: &str) -> Option<String> {
    let (addr, bits) = match v.split_once('/') {
        Some((a, b)) => (a, Some(b.parse::<u8>().ok()?)),
        None => (v, None),
    };
    let ip: IpAddr = addr.parse().ok()?;
    let max = if ip.is_ipv4() { 32 } else { 128 };
    let bits = bits.unwrap_or(max);
    (bits <= max).then(|| format!("{ip}/{bits}"))
}

/// The rule as it is shown and stored.
pub fn display(t: &Target) -> String {
    match t {
        Target::Suffix(d) => d.clone(),
        Target::Full(d) => format!("full:{d}"),
        Target::Keyword(k) => format!("keyword:{k}"),
        Target::Cidr(c) => c.clone(),
    }
}

/// Xray routing rules, first match wins, so they go before the route's own.
pub fn xray_rules(rules: &[CustomRule]) -> Vec<Value> {
    rules
        .iter()
        .filter_map(|r| {
            let tag = match r.action {
                RuleAction::Direct => "direct",
                RuleAction::Proxy => "proxy",
                RuleAction::Block => "block",
            };
            Some(match parse_rule(&r.value).ok()? {
                Target::Suffix(d) => json!({ "type": "field", "outboundTag": tag, "domain": [format!("domain:{d}")] }),
                Target::Full(d) => json!({ "type": "field", "outboundTag": tag, "domain": [format!("full:{d}")] }),
                // A plain string is Xray's substring match.
                Target::Keyword(k) => json!({ "type": "field", "outboundTag": tag, "domain": [k] }),
                Target::Cidr(c) => json!({ "type": "field", "outboundTag": tag, "ip": [c] }),
            })
        })
        .collect()
}

/// sing-box route rules and the DNS rules that go with them: a site sent
/// direct is also looked up by the local resolver, as its traffic is.
pub fn singbox_rules(rules: &[CustomRule]) -> (Vec<Value>, Vec<Value>) {
    let mut route = vec![];
    let mut dns = vec![];
    for r in rules {
        let Ok(target) = parse_rule(&r.value) else {
            continue;
        };
        let mut m = match &target {
            Target::Suffix(d) => json!({ "domain_suffix": [d] }),
            Target::Full(d) => json!({ "domain": [d] }),
            Target::Keyword(k) => json!({ "domain_keyword": [k] }),
            Target::Cidr(c) => json!({ "ip_cidr": [c] }),
        };
        let is_domain = !matches!(target, Target::Cidr(_));
        match r.action {
            RuleAction::Direct => {
                if is_domain {
                    let mut d = m.clone();
                    d["server"] = json!("local");
                    dns.push(d);
                }
                m["outbound"] = json!("direct");
            }
            RuleAction::Proxy => {
                if is_domain {
                    let mut d = m.clone();
                    d["server"] = json!("remote");
                    dns.push(d);
                }
                m["outbound"] = json!("proxy");
            }
            RuleAction::Block => {
                if is_domain {
                    let mut d = m.clone();
                    d["action"] = json!("predefined");
                    d["rcode"] = json!("NXDOMAIN");
                    dns.push(d);
                }
                m["action"] = json!("reject");
            }
        }
        route.push(m);
    }
    (route, dns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_people_type() {
        assert_eq!(
            parse_rule(" Digikala.com "),
            Ok(Target::Suffix("digikala.com".into()))
        );
        assert_eq!(
            parse_rule("*.corp.example"),
            Ok(Target::Suffix("corp.example".into()))
        );
        assert_eq!(
            parse_rule("https://www.example.com/path?q=1"),
            Ok(Target::Suffix("www.example.com".into()))
        );
        assert_eq!(
            parse_rule("full:api.example.com"),
            Ok(Target::Full("api.example.com".into()))
        );
        assert_eq!(parse_rule("keyword:ads"), Ok(Target::Keyword("ads".into())));
        assert_eq!(parse_rule("1.2.3.4"), Ok(Target::Cidr("1.2.3.4/32".into())));
        assert_eq!(
            parse_rule("10.0.0.0/8"),
            Ok(Target::Cidr("10.0.0.0/8".into()))
        );
        assert_eq!(
            parse_rule("2001:db8::/32"),
            Ok(Target::Cidr("2001:db8::/32".into()))
        );
        for bad in [
            "",
            "localhost",
            "exa mple.com",
            "1.2.3.4/40",
            "-a.com",
            "keyword:",
            "a.123",
            "\"},{\"x",
        ] {
            assert!(parse_rule(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn both_engines_get_the_same_rules() {
        let rules = vec![
            CustomRule {
                value: "digikala.com".into(),
                action: RuleAction::Direct,
            },
            CustomRule {
                value: "10.0.0.0/8".into(),
                action: RuleAction::Proxy,
            },
            CustomRule {
                value: "keyword:ads".into(),
                action: RuleAction::Block,
            },
            CustomRule {
                value: "not a rule".into(),
                action: RuleAction::Block,
            },
        ];
        let x = xray_rules(&rules);
        assert_eq!(x.len(), 3, "a broken rule is skipped, not sent");
        assert_eq!(
            x[0],
            json!({ "type": "field", "outboundTag": "direct", "domain": ["domain:digikala.com"] })
        );
        assert_eq!(x[1]["ip"], json!(["10.0.0.0/8"]));
        assert_eq!(x[2]["outboundTag"], "block");

        let (route, dns) = singbox_rules(&rules);
        assert_eq!(
            route[0],
            json!({ "domain_suffix": ["digikala.com"], "outbound": "direct" })
        );
        assert_eq!(
            route[1],
            json!({ "ip_cidr": ["10.0.0.0/8"], "outbound": "proxy" })
        );
        assert_eq!(
            route[2],
            json!({ "domain_keyword": ["ads"], "action": "reject" })
        );
        assert_eq!(
            dns[0],
            json!({ "domain_suffix": ["digikala.com"], "server": "local" })
        );
        assert_eq!(dns.len(), 2, "addresses need no DNS rule");
    }
}
