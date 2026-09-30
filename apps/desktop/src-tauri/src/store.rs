//! Servers, where they came from, and the choices around them, on disk.
//!
//! Only share links are stored, never parsed configs: a link is what the
//! panel gave us, and re-parsing on load means a parser fix reaches every
//! saved server without a migration.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use geek_config::{profile_key, AppRouting, CdnTarget, CleanIp, CustomRule, FailoverThreshold, Route, Server, STALE_AFTER_MS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "kind")]
pub enum SourceKind {
    /// One of the account's services, kept in step with `/api/miniapp/subscriptions`.
    Account {
        subscription_id: String,
        tier: Option<String>,
        state: String,
        expires_at: Option<String>,
        quota_gib: Option<f64>,
        used_gib: f64,
    },
    /// A subscription URL the user added.
    Link,
    /// Share links pasted one by one.
    Manual,
}

impl SourceKind {
    /// What «گزارش مشکل» says about where a server came from.
    pub fn label(&self) -> &str {
        match self {
            SourceKind::Account { tier: Some(t), .. } => t,
            SourceKind::Account { .. } => "account service",
            SourceKind::Link => "subscription link",
            SourceKind::Manual => "manual link",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub name: String,
    /// Flattened: the UI reads `kind` and the account fields beside `id`.
    #[serde(flatten)]
    pub kind: SourceKind,
    /// The subscription URL (account services and added links).
    pub url: Option<String>,
    pub links: Vec<String>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Persisted {
    pub sources: Vec<Source>,
    pub favorites: HashSet<String>,
    pub selected: Option<String>,
    /// «سرور: خودکار»: connect to the fastest answering server.
    pub auto_select: bool,
    pub route: Route,
    /// Last real-delay result per server id; ≤ 0 means no answer.
    pub delays: HashMap<String, i64>,
    pub sort_by_ping: bool,
    /// «حالت اتصال».
    pub mode: Mode,
    /// Kill Switch (TUN mode only, through the helper).
    pub kill_switch: bool,
    /// «حالت سخت‌گیر»: the kill switch holds after a crash or a reboot.
    pub strict: bool,
    /// «اجازه به شبکه‌ی محلی» while the kill switch is on.
    pub allow_lan: bool,
    /// «تونل برنامه‌ای» (TUN mode only).
    pub apps: AppRouting,
    /// «اتصال خودکار بعد از اجرا».
    pub auto_connect: bool,
    /// «بستن = رفتن به کنار ساعت».
    pub close_to_tray: bool,
    /// Ctrl+Shift+K (⌘⇧K) connects and disconnects from anywhere.
    pub shortcut: bool,
    /// «هشدار تمام شدن سرویس»: 80% used or 3 days left.
    pub expiry_alert: bool,
    /// Subscription id → the day (YYYY-MM-DD) it was last warned about, so
    /// the warning comes once a day, not on every refresh.
    pub expiry_warned: HashMap<String, String>,
    /// «Failover»: when a running connection moves to a better server.
    pub failover: FailoverThreshold,
    /// The clean-IP scanner's memory.
    pub scan: ScanData,
    /// «قوانین دامنه و IP», in order; the first that matches decides.
    pub rules: Vec<CustomRule>,
}

/// Results per (domain, network), overrides per (server, network), and
/// whether each domain is on Cloudflare. Overrides live here, never in the
/// link, so the server's own address is always one click away and a
/// subscription refresh leaves them alone (the Android app's `ScanStore`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScanData {
    pub results: HashMap<String, ScanRecord>,
    pub overrides: HashMap<String, IpOverride>,
    pub verdicts: HashMap<String, CdnVerdict>,
    /// The scanner's «تست دانلود» switch.
    pub download_test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRecord {
    pub results: Vec<CleanIp>,
    pub scanned_at: i64,
}

impl ScanRecord {
    pub fn is_stale(&self, now_ms: i64) -> bool {
        now_ms - self.scanned_at > STALE_AFTER_MS
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpOverride {
    pub ip: String,
    pub applied_at: i64,
    pub latency_ms: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdnVerdict {
    pub behind: bool,
    pub checked_at: i64,
}

/// How other programs reach the tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// The system proxy: no administrator rights, only programs that read it.
    #[default]
    Proxy,
    /// A TUN device through geekvpn-helper: every program.
    Tun,
}

impl Default for Persisted {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            favorites: HashSet::new(),
            selected: None,
            auto_select: true,
            route: Route::Smart,
            delays: HashMap::new(),
            sort_by_ping: false,
            mode: Mode::Proxy,
            kill_switch: false,
            strict: false,
            allow_lan: true,
            apps: AppRouting::default(),
            auto_connect: false,
            close_to_tray: true,
            shortcut: true,
            expiry_alert: true,
            expiry_warned: HashMap::new(),
            failover: FailoverThreshold::default(),
            scan: ScanData::default(),
            rules: Vec::new(),
        }
    }
}

/// A server as the UI lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerView {
    #[serde(flatten)]
    pub server: Server,
    pub source_id: String,
    pub favorite: bool,
    pub delay_ms: Option<i64>,
    pub cdn: bool,
}

pub struct Store {
    path: PathBuf,
    pub data: Persisted,
}

impl Store {
    pub fn load(path: PathBuf) -> Self {
        let data = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self { path, data }
    }

    /// Written atomically (temp file, then rename): a crash mid-write must
    /// not cost the user their server list.
    pub fn save(&self) -> Result<(), String> {
        let tmp = self.path.with_extension("json.tmp");
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let bytes = serde_json::to_vec_pretty(&self.data).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }

    /// Every server, favourites first, then by ping if asked, else in
    /// subscription order. Unparseable links are skipped.
    pub fn servers(&self) -> Vec<ServerView> {
        let mut out: Vec<ServerView> = self
            .data
            .sources
            .iter()
            .flat_map(|src| {
                src.links.iter().filter_map(move |l| Server::parse(l).ok()).map(move |s| (src.id.clone(), s))
            })
            .map(|(source_id, server)| ServerView {
                favorite: self.data.favorites.contains(&server.id),
                delay_ms: self.data.delays.get(&server.id).copied(),
                cdn: server.cdn_fronted(),
                source_id,
                server,
            })
            .collect();
        let ping = |v: &ServerView| match v.delay_ms {
            Some(ms) if ms > 0 => ms,
            _ => i64::MAX,
        };
        // Stable sorts: ties keep subscription order.
        if self.data.sort_by_ping {
            out.sort_by_key(ping);
        }
        out.sort_by_key(|v| !v.favorite);
        out
    }

    /// The server and its source.
    pub fn find_view(&self, id: &str) -> Option<ServerView> {
        self.servers().into_iter().find(|v| v.server.id == id)
    }

    /// Clean addresses apply to `direct` account services and to links the
    /// customer added: tunnel and elite services reach our own servers,
    /// where a Cloudflare address means nothing.
    pub fn scan_allowed(&self, source_id: &str) -> bool {
        self.data.sources.iter().find(|s| s.id == source_id).is_some_and(|s| match &s.kind {
            SourceKind::Account { tier, .. } => tier.as_deref() == Some("direct"),
            SourceKind::Link | SourceKind::Manual => true,
        })
    }

    /// The scanner's target for a server, when it may use clean addresses.
    pub fn scan_target(&self, v: &ServerView) -> Option<CdnTarget> {
        self.scan_allowed(&v.source_id).then(|| CdnTarget::of(&v.server)).flatten()
    }

    pub fn override_key(v: &ServerView, network: &str) -> String {
        format!("{}|{network}", profile_key(&v.source_id, &v.server))
    }

    /// The clean address `v` uses on `network` instead of its own: only for
    /// a domain known to be on Cloudflare, since a Cloudflare address in
    /// front of a server the CDN does not carry breaks it.
    pub fn address_override(&self, v: &ServerView, network: &str) -> Option<String> {
        let target = self.scan_target(v)?;
        if !self.data.scan.verdicts.get(&target.verdict_key()).is_some_and(|d| d.behind) {
            return None;
        }
        self.data.scan.overrides.get(&Self::override_key(v, network)).map(|o| o.ip.clone())
    }

    /// Fresh clean addresses for `v`'s domain on `network`, best first.
    pub fn fresh_ips(&self, v: &ServerView, network: &str, now_ms: i64) -> Vec<String> {
        let Some(target) = self.scan_target(v) else { return vec![] };
        match self.data.scan.results.get(&target.result_key(network)) {
            Some(r) if !r.is_stale(now_ms) => r.results.iter().map(|c| c.ip.clone()).collect(),
            _ => vec![],
        }
    }


    /// Candidates for «سرور: خودکار»: every server that answered its last
    /// test, fastest first.
    pub fn fastest(&self) -> Vec<ServerView> {
        let mut all: Vec<(i64, ServerView)> =
            self.servers().into_iter().filter_map(|v| v.delay_ms.filter(|ms| *ms > 0).map(|ms| (ms, v))).collect();
        all.sort_by_key(|(ms, _)| *ms);
        all.into_iter().map(|(_, v)| v).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(links: &[&str]) -> Store {
        let mut s = Store::load(std::env::temp_dir().join(format!("geek-store-{}.json", std::process::id())));
        s.data.sources = vec![Source {
            id: "m".into(),
            name: "manual".into(),
            kind: SourceKind::Manual,
            url: None,
            links: links.iter().map(|l| l.to_string()).collect(),
            updated_at: None,
        }];
        s
    }

    const A: &str = "trojan://a@a.example.com:443#A";
    const B: &str = "trojan://b@b.example.com:443#B";
    const C: &str = "trojan://c@c.example.com:443#C";

    #[test]
    fn favourites_first_then_ping_order_when_asked() {
        let mut s = store_with(&[A, B, C]);
        let id = |l: &str| Server::parse(l).unwrap().id;
        s.data.delays = HashMap::from([(id(A), 300), (id(B), 90), (id(C), -1)]);
        assert_eq!(s.servers().iter().map(|v| v.server.name.as_str()).collect::<Vec<_>>(), ["A", "B", "C"]);
        s.data.sort_by_ping = true;
        assert_eq!(s.servers().iter().map(|v| v.server.name.as_str()).collect::<Vec<_>>(), ["B", "A", "C"]);
        s.data.favorites.insert(id(C));
        assert_eq!(s.servers()[0].server.name, "C");
        // Auto-select never picks a server that did not answer.
        assert_eq!(s.fastest().iter().map(|x| x.server.name.as_str()).collect::<Vec<_>>(), ["B", "A"]);
    }

    #[test]
    fn round_trips_through_disk() {
        let s = store_with(&[A]);
        s.save().unwrap();
        let back = Store::load(s.path.clone());
        assert_eq!(back.servers().len(), 1);
        let _ = std::fs::remove_file(&s.path);
    }
}

#[cfg(test)]
mod wire {
    use super::SourceKind;
    use crate::tunnel::TunnelState;

    /// The UI reads camelCase; serde's `rename_all` on a tagged enum renames
    /// only the variants, which once left `since_ms` unreadable in the UI.
    #[test]
    fn tagged_enums_serialise_their_fields_in_camel_case() {
        let on = serde_json::to_value(TunnelState::On {
            server_id: "s".into(),
            server_name: "n".into(),
            since_ms: 1,
            mode: super::Mode::Tun,
            http_port: 2,
            socks_port: 3,
            delay_ms: 4,
            kill_switch: true,
            note: None,
        })
        .unwrap();
        assert_eq!(on["status"], "on");
        assert_eq!(on["sinceMs"], 1);
        assert_eq!(on["httpPort"], 2);
        assert_eq!(on["mode"], "tun");
        assert_eq!(on["killSwitch"], true);
        let acc = serde_json::to_value(SourceKind::Account {
            subscription_id: "x".into(),
            tier: None,
            state: "active".into(),
            expires_at: None,
            quota_gib: Some(40.0),
            used_gib: 1.0,
        })
        .unwrap();
        assert_eq!(acc["kind"], "account");
        assert_eq!(acc["quotaGib"], 40.0);
        assert!(acc.get("subscriptionId").is_some());

        // A source carries its kind flattened, as `Source` in lib/servers.ts.
        let src = serde_json::to_value(super::Source {
            id: "geek-x".into(),
            name: "n".into(),
            kind: SourceKind::Account {
                subscription_id: "x".into(),
                tier: None,
                state: "active".into(),
                expires_at: None,
                quota_gib: Some(40.0),
                used_gib: 1.0,
            },
            url: None,
            links: vec![],
            updated_at: None,
        })
        .unwrap();
        assert_eq!(src["kind"], "account");
        assert_eq!(src["quotaGib"], 40.0);
        let back: super::Source = serde_json::from_value(src).unwrap();
        assert!(matches!(back.kind, SourceKind::Account { .. }));
    }
}
