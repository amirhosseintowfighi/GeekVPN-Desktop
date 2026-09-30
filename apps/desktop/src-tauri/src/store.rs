//! Servers, where they came from, and the choices around them, on disk.
//!
//! Only share links are stored, never parsed configs: a link is what the
//! panel gave us, and re-parsing on load means a parser fix reaches every
//! saved server without a migration.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use geek_config::{AppRouting, Route, Server};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub name: String,
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

    pub fn find(&self, id: &str) -> Option<Server> {
        self.data
            .sources
            .iter()
            .flat_map(|s| s.links.iter())
            .filter_map(|l| Server::parse(l).ok())
            .find(|s| s.id == id)
    }

    /// Candidates for «سرور: خودکار»: every server that answered its last
    /// test, fastest first.
    pub fn fastest(&self) -> Vec<Server> {
        let mut all: Vec<(i64, Server)> = self
            .servers()
            .into_iter()
            .filter_map(|v| v.delay_ms.filter(|ms| *ms > 0).map(|ms| (ms, v.server)))
            .collect();
        all.sort_by_key(|(ms, _)| *ms);
        all.into_iter().map(|(_, s)| s).collect()
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
        assert_eq!(s.fastest().iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["B", "A"]);
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
    }
}
