//! «تست نشت»: whether traffic, IPv6 and DNS really leave through the VPN.
//!
//! - IP: Cloudflare's trace through the tunnel, and (proxy mode) without it,
//!   so the customer sees their VPN address beside their real one.
//! - IPv6: Cloudflare's trace on its IPv6 address. Answering from Iran means
//!   IPv6 goes around the tunnel.
//! - DNS (TUN mode only; in proxy mode other programs' DNS is not the VPN's
//!   job): the bash.ws leak test. Ten made-up names under a fresh id are
//!   looked up with the system resolver, and bash.ws reports which DNS
//!   servers asked for them. A resolver in Iran is a leak.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::servers::ServersState;
use crate::tools::{parse_trace, ExitIp};
use crate::tunnel::TunnelState;

const TRACE_V4: &str = "https://1.1.1.1/cdn-cgi/trace";
const TRACE_V6: &str = "https://[2606:4700:4700::1111]/cdn-cgi/trace";
const DNS_HOST: &str = "bash.ws";
const HOME: &str = "IR";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Ok,
    Leak,
    /// Could not be measured (the note says why).
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub key: &'static str,
    pub verdict: Verdict,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resolver {
    pub ip: String,
    #[serde(default)]
    pub country: String,
    #[serde(default, alias = "country_name")]
    pub country_name: String,
    #[serde(default)]
    pub asn: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeakReport {
    pub tun: bool,
    pub vpn: Option<ExitIp>,
    /// Proxy mode: the address without the VPN.
    pub real: Option<ExitIp>,
    pub ipv6: Option<ExitIp>,
    pub resolvers: Vec<Resolver>,
    pub checks: Vec<Check>,
}

fn client(proxy: Option<u16>, timeout: u64) -> Result<reqwest::Client, String> {
    let b = reqwest::Client::builder().connect_timeout(Duration::from_secs(6)).timeout(Duration::from_secs(timeout));
    let b = match proxy {
        Some(port) => b.proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?),
        None => b.no_proxy(),
    };
    b.build().map_err(|e| e.to_string())
}

async fn trace(c: &reqwest::Client, url: &str) -> Option<ExitIp> {
    let body = c.get(url).send().await.ok()?.error_for_status().ok()?.text().await.ok()?;
    parse_trace(&body)
}

/// bash.ws answers a list of entries; the DNS servers are `type: "dns"`.
pub fn parse_resolvers(body: &str) -> Option<Vec<Resolver>> {
    #[derive(Deserialize)]
    struct Entry {
        #[serde(default, rename = "type")]
        kind: String,
        #[serde(flatten)]
        resolver: Resolver,
    }
    let entries: Vec<Entry> = serde_json::from_str(body).ok()?;
    Some(entries.into_iter().filter(|e| e.kind == "dns" && e.resolver.ip.parse::<std::net::IpAddr>().is_ok()).map(|e| e.resolver).collect())
}

async fn dns_test(c: &reqwest::Client) -> Result<Vec<Resolver>, String> {
    let id = c
        .get(format!("https://{DNS_HOST}/id"))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|_| "سرویس تست DNS جواب نداد.".to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let id = id.trim();
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err("سرویس تست DNS جواب درستی نداد.".into());
    }
    // The names do not exist; only the question reaching bash.ws matters.
    for i in 1..=10 {
        let name = format!("{i}.{id}.{DNS_HOST}");
        let _ = tokio::time::timeout(Duration::from_secs(3), tokio::net::lookup_host((name.as_str(), 80))).await;
    }
    let body = c
        .get(format!("https://{DNS_HOST}/dnsleak/test/{id}?json"))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|_| "نتیجه‌ی تست DNS نیامد.".to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    parse_resolvers(&body).ok_or_else(|| "نتیجه‌ی تست DNS خوانده نشد.".into())
}

/// The verdicts, from what was measured.
pub fn judge(tun: bool, vpn: Option<&ExitIp>, real: Option<&ExitIp>, ipv6: Option<&ExitIp>, dns: Option<&Result<Vec<Resolver>, String>>) -> Vec<Check> {
    let mut out = Vec::new();
    out.push(match vpn {
        None => Check { key: "ip", verdict: Verdict::Unknown, note: "از داخل VPN جوابی نیامد.".into() },
        Some(v) if v.country == HOME => Check { key: "ip", verdict: Verdict::Leak, note: "سایت‌ها تو را از ایران می‌بینند؛ ترافیک از VPN نمی‌رود.".into() },
        Some(v) if real.is_some_and(|r| r.ip == v.ip) => Check { key: "ip", verdict: Verdict::Leak, note: "IP بیرونی با و بدون VPN یکی است.".into() },
        Some(_) => Check { key: "ip", verdict: Verdict::Ok, note: "سایت‌ها IP سرور VPN را می‌بینند، نه IP تو را.".into() },
    });
    out.push(match ipv6 {
        None => Check { key: "ipv6", verdict: Verdict::Ok, note: "IPv6 بیرون نمی‌رود (یا شبکه IPv6 ندارد).".into() },
        Some(v) if v.country == HOME => Check { key: "ipv6", verdict: Verdict::Leak, note: "IPv6 از کنار VPN بیرون می‌رود. در «تنظیمات پیشرفته» «مسدود کردن IPv6» را روشن کن.".into() },
        Some(_) => Check { key: "ipv6", verdict: Verdict::Ok, note: "IPv6 هم از VPN می‌رود.".into() },
    });
    out.push(match (tun, dns) {
        (false, _) => Check {
            key: "dns",
            verdict: Verdict::Unknown,
            note: "در حالت پروکسی سیستم فقط برنامه‌هایی که پروکسی را می‌خوانند از VPN می‌روند و DNS بقیه به VPN ربطی ندارد. برای پوشش کامل حالت TUN را روشن کن.".into(),
        },
        (true, None) => Check { key: "dns", verdict: Verdict::Unknown, note: "تست DNS انجام نشد.".into() },
        (true, Some(Err(e))) => Check { key: "dns", verdict: Verdict::Unknown, note: e.clone() },
        (true, Some(Ok(list))) if list.is_empty() => Check { key: "dns", verdict: Verdict::Unknown, note: "هیچ سرور DNSی دیده نشد.".into() },
        (true, Some(Ok(list))) => match list.iter().find(|r| r.country == HOME) {
            Some(_) => Check { key: "dns", verdict: Verdict::Leak, note: "یک سرور DNS ایرانی نام سایت‌هایی که باز می‌کنی را می‌بیند.".into() },
            None => Check { key: "dns", verdict: Verdict::Ok, note: format!("{} سرور DNS دیده شد و هیچ‌کدام در ایران نیست.", list.len()) },
        },
    });
    out
}

#[tauri::command]
pub async fn leak_test(state: State<'_, ServersState>) -> Result<LeakReport, String> {
    let (tun, proxy) = match &*state.tunnel.state.lock().await {
        TunnelState::On { http_port, .. } if *http_port > 0 => (false, Some(*http_port)),
        TunnelState::On { .. } => (true, None),
        _ => return Err("اول وصل شو؛ تست نشت روی اتصال روشن انجام می‌شود.".into()),
    };
    let through = client(proxy, 12)?;
    let direct = client(None, 8)?;
    let vpn_f = trace(&through, TRACE_V4);
    // Proxy mode: the address without the proxy, for comparison.
    let real_f = async {
        if tun {
            None
        } else {
            trace(&direct, TRACE_V4).await
        }
    };
    // IPv6 as the system sends it (in TUN mode, as the TUN does).
    let v6_f = trace(&direct, TRACE_V6);
    let dns_f = async {
        if tun {
            Some(dns_test(&direct).await)
        } else {
            None
        }
    };
    let (vpn, real, ipv6, dns) = tokio::join!(vpn_f, real_f, v6_f, dns_f);
    let mut checks = judge(tun, vpn.as_ref(), real.as_ref(), ipv6.as_ref(), dns.as_ref());
    if tun && only_listed_apps(&state).await {
        mark_only_apps(&mut checks);
    }
    let resolvers = dns.and_then(Result::ok).unwrap_or_default();
    Ok(LeakReport { tun, vpn, real, ipv6, resolvers, checks })
}

/// TUN with «فقط برنامه‌های انتخاب‌شده» (sing-box; Windows' TUN has no
/// per-app routing): GeekVPN itself is not one of them, so what it measures
/// is the network without the VPN, not a leak.
pub async fn only_listed_apps(state: &ServersState) -> bool {
    !cfg!(windows) && state.store.lock().await.data.apps.mode == geek_config::AppMode::Only
}

fn mark_only_apps(checks: &mut [Check]) {
    for c in checks.iter_mut().filter(|c| c.key != "ipv6") {
        c.verdict = Verdict::Unknown;
        c.note = "فقط برنامه‌هایی که در «تونل برنامه‌ای» انتخاب کرده‌ای از VPN می‌روند و خود GeekVPN جزوشان نیست؛ این تست از بیرون VPN انجام شد و معنی ندارد.".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ip: &str, c: &str) -> ExitIp {
        ExitIp { ip: ip.into(), country: c.into(), colo: String::new() }
    }

    #[test]
    fn reads_bash_ws() {
        let body = r#"[{"ip":"203.0.113.9","country":"DE","country_name":"Germany","asn":"AS13335 Cloudflare","type":"ip"},
            {"ip":"172.70.1.2","country":"DE","country_name":"Germany","asn":"AS13335 Cloudflare","type":"dns"},
            {"ip":"","country":"","country_name":"","asn":"DNS is not leaking.","type":"conclusion"}]"#;
        let r = parse_resolvers(body).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].ip, "172.70.1.2");
        assert_eq!(r[0].country_name, "Germany");
        assert!(parse_resolvers("<html>").is_none());
    }

    #[test]
    fn judges_what_was_seen() {
        let v = |c: &[Check], k: &str| c.iter().find(|x| x.key == k).unwrap().verdict.clone();
        let clean = judge(true, Some(&at("203.0.113.9", "DE")), None, None, Some(&Ok(vec![Resolver { ip: "1.1.1.1".into(), country: "DE".into(), country_name: String::new(), asn: String::new() }])));
        assert_eq!((v(&clean, "ip"), v(&clean, "ipv6"), v(&clean, "dns")), (Verdict::Ok, Verdict::Ok, Verdict::Ok));

        let leaky = judge(true, Some(&at("5.1.1.1", "IR")), None, Some(&at("2a01::1", "IR")), Some(&Ok(vec![Resolver { ip: "217.218.127.127".into(), country: "IR".into(), country_name: String::new(), asn: String::new() }])));
        assert_eq!((v(&leaky, "ip"), v(&leaky, "ipv6"), v(&leaky, "dns")), (Verdict::Leak, Verdict::Leak, Verdict::Leak));

        let proxy = judge(false, Some(&at("203.0.113.9", "DE")), Some(&at("203.0.113.9", "DE")), None, None);
        assert_eq!((v(&proxy, "ip"), v(&proxy, "dns")), (Verdict::Leak, Verdict::Unknown));

        let mut only = leaky.clone();
        mark_only_apps(&mut only);
        assert_eq!((v(&only, "ip"), v(&only, "ipv6"), v(&only, "dns")), (Verdict::Unknown, Verdict::Leak, Verdict::Unknown));
    }
}
