//! Servers and the connection, as commands the UI may call.

use std::sync::Arc;
use std::time::Duration;

use geek_config::{delay_config, parse_subscription, AppRouting, Route, Server};
use geek_ipc::KillSwitch;
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::auth::AuthState;
use crate::store::{Mode, ServerView, Source, SourceKind, Store};
use crate::tunnel::{probe_url, ConnectOptions, Tunnel, TunnelState};

pub const CHANGED: &str = "servers://changed";
pub const DELAY: &str = "servers://delay";

pub struct ServersState {
    pub store: Mutex<Store>,
    pub tunnel: Arc<Tunnel>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServersView {
    sources: Vec<Source>,
    servers: Vec<ServerView>,
    selected: Option<String>,
    auto_select: bool,
    route: Route,
    sort_by_ping: bool,
    mode: Mode,
    kill_switch: bool,
    strict: bool,
    allow_lan: bool,
    apps: AppRouting,
}

fn view(store: &Store) -> ServersView {
    ServersView {
        sources: store.data.sources.clone(),
        servers: store.servers(),
        selected: store.data.selected.clone(),
        auto_select: store.data.auto_select,
        route: store.data.route,
        sort_by_ping: store.data.sort_by_ping,
        mode: store.data.mode,
        kill_switch: store.data.kill_switch,
        strict: store.data.strict,
        allow_lan: store.data.allow_lan,
        apps: store.data.apps.clone(),
    }
}

async fn changed(app: &AppHandle, state: &ServersState) -> Result<ServersView, String> {
    let store = state.store.lock().await;
    store.save()?;
    let v = view(&store);
    let _ = app.emit(CHANGED, ());
    Ok(v)
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or_default()
}

#[tauri::command]
pub async fn servers_state(state: State<'_, ServersState>) -> Result<ServersView, String> {
    Ok(view(&*state.store.lock().await))
}

/// Pulls the account's services from the backend and every subscription
/// URL, as the Android app's `AccountSync` does: account services follow the
/// account (a removed service disappears), manual links are never touched,
/// and a subscription that cannot be fetched keeps its last servers.
#[tauri::command]
pub async fn servers_refresh(
    app: AppHandle,
    state: State<'_, ServersState>,
    auth: State<'_, AuthState>,
) -> Result<ServersView, String> {
    let api = auth.session.api().clone();
    let mut problems = Vec::new();

    if auth.session.user().await.is_some() {
        let cards = auth
            .session
            .authorized(|t| {
                let api = api.clone();
                async move { api.subscriptions(&t).await }
            })
            .await
            .map_err(|e| e.user_message())?;
        let previous: Vec<Source> = state.store.lock().await.data.sources.clone();
        let mut account = Vec::new();
        for card in cards {
            let Some(url) = card.subscription_url.clone().filter(|u| !u.trim().is_empty()) else { continue };
            let id = format!("geek-{}", card.subscription_id);
            let old_links = previous.iter().find(|s| s.id == id).map(|s| s.links.clone()).unwrap_or_default();
            // Expired or suspended services keep their card but offer no servers.
            let links = if card.state == "active" {
                match api.fetch_subscription(&url).await {
                    Ok(body) => parse_subscription(&body).into_iter().map(|s| s.link).collect(),
                    Err(e) => {
                        problems.push(e.user_message());
                        old_links
                    }
                }
            } else {
                Vec::new()
            };
            account.push(Source {
                id,
                name: format!("{} · {}", card.product_name_fa, card.plan_name_fa),
                kind: SourceKind::Account {
                    subscription_id: card.subscription_id,
                    tier: card.tier,
                    state: card.state,
                    expires_at: card.expires_at.map(|d| d.to_rfc3339()),
                    quota_gib: card.quota_gib,
                    used_gib: card.used_gib,
                },
                url: Some(url),
                links,
                updated_at: Some(now()),
            });
        }
        let mut store = state.store.lock().await;
        store.data.sources.retain(|s| !matches!(s.kind, SourceKind::Account { .. }));
        let manual = std::mem::take(&mut store.data.sources);
        store.data.sources = account.into_iter().chain(manual).collect();
    }

    let links: Vec<(String, String)> = state
        .store
        .lock()
        .await
        .data
        .sources
        .iter()
        .filter(|s| matches!(s.kind, SourceKind::Link))
        .filter_map(|s| s.url.clone().map(|u| (s.id.clone(), u)))
        .collect();
    for (id, url) in links {
        match api.fetch_subscription(&url).await {
            Ok(body) => {
                let fresh: Vec<String> = parse_subscription(&body).into_iter().map(|s| s.link).collect();
                if let Some(src) = state.store.lock().await.data.sources.iter_mut().find(|s| s.id == id) {
                    src.links = fresh;
                    src.updated_at = Some(now());
                }
            }
            Err(e) => problems.push(e.user_message()),
        }
    }
    let v = changed(&app, &state).await?;
    match problems.first() {
        // Partial success still returns the list; the UI shows the problem.
        Some(p) if v.servers.is_empty() => Err(p.clone()),
        _ => Ok(v),
    }
}

/// «افزودن لینک»: a subscription URL, or share links (one or many lines).
#[tauri::command]
pub async fn servers_add(
    app: AppHandle,
    state: State<'_, ServersState>,
    auth: State<'_, AuthState>,
    text: String,
) -> Result<ServersView, String> {
    let text = text.trim();
    if text.starts_with("http://") || text.starts_with("https://") {
        let body = auth.session.api().fetch_subscription(text).await.map_err(|e| e.user_message())?;
        let servers = parse_subscription(&body);
        if servers.is_empty() {
            return Err("این لینک اشتراک سروری که برنامه بشناسد نداشت.".into());
        }
        let host = url::Url::parse(text).ok().and_then(|u| u.host_str().map(String::from)).unwrap_or_default();
        let mut store = state.store.lock().await;
        store.data.sources.retain(|s| s.url.as_deref() != Some(text));
        store.data.sources.push(Source {
            id: format!("link-{}", servers[0].id),
            name: host,
            kind: SourceKind::Link,
            url: Some(text.to_string()),
            links: servers.into_iter().map(|s| s.link).collect(),
            updated_at: Some(now()),
        });
    } else {
        let parsed: Vec<Server> = parse_subscription(text);
        if parsed.is_empty() {
            return Err("لینکی که برنامه بشناسد پیدا نشد (vless، vmess، trojan، ss یا لینک اشتراک).".into());
        }
        let mut store = state.store.lock().await;
        let manual = match store.data.sources.iter().position(|s| matches!(s.kind, SourceKind::Manual)) {
            Some(i) => i,
            None => {
                store.data.sources.push(Source {
                    id: "manual".into(),
                    name: "لینک‌های دستی".into(),
                    kind: SourceKind::Manual,
                    url: None,
                    links: Vec::new(),
                    updated_at: None,
                });
                store.data.sources.len() - 1
            }
        };
        let src = &mut store.data.sources[manual];
        for s in parsed {
            if !src.links.contains(&s.link) {
                src.links.push(s.link);
            }
        }
    }
    changed(&app, &state).await
}

#[tauri::command]
pub async fn servers_remove_source(app: AppHandle, state: State<'_, ServersState>, id: String) -> Result<ServersView, String> {
    state.store.lock().await.data.sources.retain(|s| s.id != id || matches!(s.kind, SourceKind::Account { .. }));
    changed(&app, &state).await
}

/// Every connection setting the UI changes; each is optional.
#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    favorite: Option<(String, bool)>,
    selected: Option<String>,
    auto_select: Option<bool>,
    route: Option<Route>,
    sort_by_ping: Option<bool>,
    mode: Option<Mode>,
    kill_switch: Option<bool>,
    strict: Option<bool>,
    allow_lan: Option<bool>,
    apps: Option<AppRouting>,
}

#[tauri::command]
pub async fn servers_set(app: AppHandle, state: State<'_, ServersState>, settings: Settings) -> Result<ServersView, String> {
    let Settings { favorite, selected, auto_select, route, sort_by_ping, mode, kill_switch, strict, allow_lan, apps } = settings;
    {
        let mut store = state.store.lock().await;
        if let Some((id, on)) = favorite {
            if on {
                store.data.favorites.insert(id);
            } else {
                store.data.favorites.remove(&id);
            }
        }
        if let Some(id) = selected {
            store.data.selected = Some(id);
            // Picking a server by hand is turning «خودکار» off, as on Android.
            store.data.auto_select = false;
        }
        if let Some(on) = auto_select {
            store.data.auto_select = on;
        }
        if let Some(r) = route {
            store.data.route = r;
        }
        if let Some(p) = sort_by_ping {
            store.data.sort_by_ping = p;
        }
        if let Some(m) = mode {
            store.data.mode = m;
        }
        if let Some(k) = kill_switch {
            store.data.kill_switch = k;
        }
        if let Some(s) = strict {
            store.data.strict = s;
        }
        if let Some(l) = allow_lan {
            store.data.allow_lan = l;
        }
        if let Some(mut a) = apps {
            a.paths.retain(|p| !p.trim().is_empty());
            a.paths.dedup();
            store.data.apps = a;
        }
    }
    changed(&app, &state).await
}

/// The real-delay test over every server, eight at a time. Each result is
/// emitted as it lands, so the list fills in instead of freezing.
#[tauri::command]
pub async fn servers_test(app: AppHandle, state: State<'_, ServersState>) -> Result<ServersView, String> {
    let servers: Vec<Server> = state.store.lock().await.servers().into_iter().map(|v| v.server).collect();
    if servers.is_empty() {
        return changed(&app, &state).await;
    }
    let core = state.tunnel.core().await?;
    let mut events = core.events();
    let relay = app.clone();
    let forward = tauri::async_runtime::spawn(async move {
        while let Ok(ev) = events.recv().await {
            if ev.event == "test.result" {
                let _ = relay.emit(DELAY, ev.data);
            }
        }
    });
    let items: Vec<_> = servers.iter().map(|s| json!({ "id": s.id, "config": delay_config(s, None) })).collect();
    let result = core
        .call("test.delay", json!({ "items": items, "url": probe_url(), "concurrency": 8 }), Duration::from_secs(120))
        .await;
    forward.abort();
    let result = result.map_err(|e| crate::tunnel::core_message(&e))?;
    {
        let mut store = state.store.lock().await;
        for r in result.as_array().into_iter().flatten() {
            if let (Some(id), Some(ms)) = (r["id"].as_str(), r["ms"].as_i64()) {
                store.data.delays.insert(id.to_string(), ms);
            }
        }
    }
    changed(&app, &state).await
}

#[tauri::command]
pub async fn tunnel_state(state: State<'_, ServersState>) -> Result<TunnelState, String> {
    Ok(state.tunnel.state.lock().await.clone())
}

/// «اتصال»: the selected server, or with «خودکار» the fastest that answered
/// (testing first if nothing has been tested yet), then the next ones.
#[tauri::command]
pub async fn tunnel_connect(app: AppHandle, state: State<'_, ServersState>) -> Result<(), String> {
    let (auto, needs_test) = {
        let store = state.store.lock().await;
        (store.data.auto_select, store.data.auto_select && store.fastest().is_empty())
    };
    if needs_test {
        servers_test(app.clone(), state.clone()).await?;
    }
    let (candidates, opts) = {
        let store = state.store.lock().await;
        let chosen = store.data.selected.as_deref().and_then(|id| store.find(id));
        let list = match (auto, chosen) {
            (false, Some(s)) => vec![s],
            (false, None) | (true, _) => {
                let fast = store.fastest();
                if fast.is_empty() {
                    // Nothing answered the test: try the list as it stands,
                    // the probe URL itself may be what is blocked.
                    store.servers().into_iter().map(|v| v.server).collect()
                } else {
                    fast
                }
            }
        };
        let d = &store.data;
        let opts = ConnectOptions {
            route: d.route,
            mode: d.mode,
            // The kill switch lives in the helper, so it comes with TUN mode.
            kill_switch: (d.mode == Mode::Tun && d.kill_switch).then_some(KillSwitch { allow_lan: d.allow_lan, strict: d.strict }),
            apps: d.apps.clone(),
        };
        (list, opts)
    };
    state.tunnel.connect(&app, candidates, opts).await?;
    // Remember what worked, so the next manual connect starts there.
    if let TunnelState::On { server_id, .. } = &*state.tunnel.state.lock().await {
        state.store.lock().await.data.selected = Some(server_id.clone());
    }
    let _ = state.store.lock().await.save();
    Ok(())
}

#[tauri::command]
pub async fn tunnel_disconnect(app: AppHandle, state: State<'_, ServersState>) -> Result<(), String> {
    state.tunnel.disconnect(&app).await;
    Ok(())
}
