//! Servers and the connection, as commands the UI may call.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use geek_config::{delay_config, parse_rule, parse_subscription, AppRouting, CustomRule, FailoverThreshold, Route, Server, MAX_RULES};
use geek_ipc::KillSwitch;
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::auth::AuthState;
use crate::store::{IpOverride, Mode, ServerView, Source, SourceKind, Store};
use crate::tunnel::{probe_url, ConnectOptions, Stage, Tunnel, TunnelState};

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
    auto_connect: bool,
    close_to_tray: bool,
    shortcut: bool,
    expiry_alert: bool,
    failover: FailoverThreshold,
    mode: Mode,
    kill_switch: bool,
    strict: bool,
    allow_lan: bool,
    apps: AppRouting,
    rules: Vec<CustomRule>,
}

fn view(store: &Store) -> ServersView {
    ServersView {
        sources: store.data.sources.clone(),
        servers: store.servers(),
        selected: store.data.selected.clone(),
        auto_select: store.data.auto_select,
        route: store.data.route,
        sort_by_ping: store.data.sort_by_ping,
        auto_connect: store.data.auto_connect,
        close_to_tray: store.data.close_to_tray,
        shortcut: store.data.shortcut,
        expiry_alert: store.data.expiry_alert,
        failover: store.data.failover,
        mode: store.data.mode,
        kill_switch: store.data.kill_switch,
        strict: store.data.strict,
        allow_lan: store.data.allow_lan,
        apps: store.data.apps.clone(),
        rules: store.data.rules.clone(),
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
    {
        let mut store = state.store.lock().await;
        crate::desktop::check_expiry(&app, &mut store.data);
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
    auto_connect: Option<bool>,
    close_to_tray: Option<bool>,
    shortcut: Option<bool>,
    expiry_alert: Option<bool>,
    failover: Option<FailoverThreshold>,
    rules: Option<Vec<CustomRule>>,
}

/// Normalises the customer's rules and refuses a broken one with its reason,
/// so what is saved is what both engines will read. A later rule for the same
/// site replaces the earlier one.
fn clean_rules(rules: Vec<CustomRule>) -> Result<Vec<CustomRule>, String> {
    if rules.len() > MAX_RULES {
        return Err(format!("حداکثر {MAX_RULES} قانون."));
    }
    let mut out: Vec<CustomRule> = vec![];
    for r in rules {
        let value = geek_config::rule_display(&parse_rule(&r.value).map_err(|e| format!("{}: {e}", r.value.trim()))?);
        out.retain(|o| o.value != value);
        out.push(CustomRule { value, action: r.action });
    }
    Ok(out)
}

#[tauri::command]
pub async fn servers_set(app: AppHandle, state: State<'_, ServersState>, settings: Settings) -> Result<ServersView, String> {
    let Settings {
        favorite,
        selected,
        auto_select,
        route,
        sort_by_ping,
        mode,
        kill_switch,
        strict,
        allow_lan,
        apps,
        auto_connect,
        close_to_tray,
        shortcut,
        expiry_alert,
        failover,
        rules,
    } = settings;
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
        let d = &mut store.data;
        for (slot, v) in [
            (&mut d.auto_connect, auto_connect),
            (&mut d.close_to_tray, close_to_tray),
            (&mut d.shortcut, shortcut),
            (&mut d.expiry_alert, expiry_alert),
        ] {
            if let Some(v) = v {
                *slot = v;
            }
        }
        if let Some(f) = failover {
            store.data.failover = f;
        }
        if let Some(r) = rules {
            store.data.rules = clean_rules(r)?;
        }
        crate::desktop::apply(&app, &store.data);
    }
    changed(&app, &state).await
}

/// The real-delay test over every server, eight at a time. Each result is
/// emitted as it lands, so the list fills in instead of freezing.
///
/// Servers the clean-IP scanner applies to are also tested on each of the
/// freshest clean addresses found on this network (three at most), and are
/// left on whichever answered fastest, their own address included: the
/// Android app's smart-connect ranking.
#[tauri::command]
pub async fn servers_test(app: AppHandle, state: State<'_, ServersState>) -> Result<ServersView, String> {
    let network = tauri::async_runtime::spawn_blocking(crate::scan::current_network).await.map_err(|e| e.to_string())?.key;
    let now = now() * 1000;
    let (views, scannable, ips) = {
        let store = state.store.lock().await;
        let views = store.servers();
        let scannable: Vec<ServerView> = views
            .iter()
            .filter(|v| {
                store.scan_target(v).is_some_and(|t| store.data.scan.verdicts.get(&t.verdict_key()).is_some_and(|d| d.behind))
            })
            .cloned()
            .collect();
        let mut ips: Vec<String> = scannable.iter().flat_map(|v| store.fresh_ips(v, &network, now)).collect();
        ips.dedup();
        ips.truncate(3);
        (views, scannable, ips)
    };
    if views.is_empty() {
        return changed(&app, &state).await;
    }
    let core = state.tunnel.core().await?;
    let mut events = core.events();
    let relay = app.clone();
    let forward = tauri::async_runtime::spawn(async move {
        loop {
            let ev = match events.recv().await {
                Ok(ev) => ev,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            };
            // Clean-address rounds ("id@ip") are not a server's own result.
            if ev.event == "test.result" && !ev.data["id"].as_str().unwrap_or("").contains('@') {
                let _ = relay.emit(DELAY, ev.data);
            }
        }
    });
    // Round 0: every server on its own address.
    let mut items: Vec<_> = views.iter().map(|v| json!({ "id": v.server.id, "config": delay_config(&v.server, None) })).collect();
    for ip in &ips {
        for v in &scannable {
            items.push(json!({ "id": format!("{}@{ip}", v.server.id), "config": delay_config(&v.server, Some(ip)) }));
        }
    }
    let result = core
        .call("test.delay", json!({ "items": items, "url": probe_url(), "concurrency": 8 }), Duration::from_secs(180))
        .await;
    forward.abort();
    let result = result.map_err(|e| crate::tunnel::core_message(&e))?;
    let measured: HashMap<String, i64> = result
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| Some((r["id"].as_str()?.to_string(), r["ms"].as_i64()?)))
        .collect();
    {
        let mut store = state.store.lock().await;
        for v in &views {
            let own = measured.get(&v.server.id).copied().unwrap_or(-1);
            let mut best = (own, None::<&String>);
            if scannable.iter().any(|s| s.server.id == v.server.id) {
                for ip in &ips {
                    let ms = measured.get(&format!("{}@{ip}", v.server.id)).copied().unwrap_or(-1);
                    if ms > 0 && (best.0 <= 0 || ms < best.0) {
                        best = (ms, Some(ip));
                    }
                }
                let key = Store::override_key(v, &network);
                match best.1 {
                    Some(ip) => {
                        store.data.scan.overrides.insert(key, IpOverride { ip: ip.clone(), applied_at: now, latency_ms: best.0 });
                    }
                    None if own > 0 => {
                        store.data.scan.overrides.remove(&key);
                    }
                    // Nothing answered at all: leave things as they were.
                    None => {}
                }
            }
            store.data.delays.insert(v.server.id.clone(), best.0);
            let _ = app.emit(DELAY, json!({ "id": v.server.id, "ms": best.0 }));
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
pub async fn tunnel_connect(app: AppHandle) -> Result<(), String> {
    connect(&app).await
}

/// The same from the tray, the shortcut and auto-connect.
pub async fn connect(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<ServersState>();
    let network = tauri::async_runtime::spawn_blocking(crate::scan::current_network).await.map_err(|e| e.to_string())?.key;
    let (auto, needs_test, scan_first) = {
        let store = state.store.lock().await;
        let auto = store.data.auto_select;
        // Smart connect: a CDN-fronted service with no fresh clean addresses
        // on this network gets a short scan first.
        let scannable: Vec<ServerView> = store
            .servers()
            .into_iter()
            .filter(|v| store.scan_target(v).is_some_and(|t| store.data.scan.verdicts.get(&t.verdict_key()).is_some_and(|d| d.behind)))
            .collect();
        let fresh = scannable.iter().any(|v| !store.fresh_ips(v, &network, now() * 1000).is_empty());
        let scan_first = (auto && !scannable.is_empty() && !fresh).then(|| scannable[0].server.id.clone());
        (auto, auto && (store.fastest().is_empty() || scan_first.is_some()), scan_first)
    };
    if let Some(id) = scan_first {
        state.tunnel.stage(app, Stage::FindingIp).await;
        let _ = crate::scan::run_scan(app, &id, false, 3, Some(Duration::from_secs(15))).await;
    }
    if needs_test {
        state.tunnel.stage(app, Stage::Testing).await;
        servers_test(app.clone(), state.clone()).await?;
    }
    let (candidates, opts) = {
        let store = state.store.lock().await;
        let chosen = store.data.selected.as_deref().and_then(|id| store.find_view(id));
        let list: Vec<ServerView> = match (auto, chosen) {
            (false, Some(v)) => vec![v],
            (false, None) | (true, _) => {
                let fast = store.fastest();
                if fast.is_empty() {
                    // Nothing answered the test: try the list as it stands,
                    // the probe URL itself may be what is blocked.
                    store.servers()
                } else {
                    fast
                }
            }
        };
        let list = list
            .into_iter()
            .map(|v| {
                let over = store.address_override(&v, &network);
                (v.server, over)
            })
            .collect::<Vec<_>>();
        let d = &store.data;
        let opts = ConnectOptions {
            route: d.route,
            mode: d.mode,
            // The kill switch lives in the helper, so it comes with TUN mode.
            kill_switch: (d.mode == Mode::Tun && d.kill_switch).then_some(KillSwitch { allow_lan: d.allow_lan, strict: d.strict }),
            apps: d.apps.clone(),
            rules: d.rules.clone(),
        };
        (list, opts)
    };
    state.tunnel.connect(app, candidates, opts).await?;
    // Remember what worked, so the next manual connect starts there.
    if let TunnelState::On { server_id, .. } = &*state.tunnel.state.lock().await {
        state.store.lock().await.data.selected = Some(server_id.clone());
    }
    let _ = state.store.lock().await.save();
    Ok(())
}

/// The failover monitor found the running connection bad: retest, and move
/// to the fastest server if it is not the one in use. Nothing answering
/// means the network itself is the likely problem: stay put.
pub async fn failover(app: &AppHandle) {
    let state = app.state::<ServersState>();
    let current = match &*state.tunnel.state.lock().await {
        TunnelState::On { server_id, .. } => server_id.clone(),
        _ => return,
    };
    if servers_test(app.clone(), state.clone()).await.is_err() {
        return;
    }
    let network = tauri::async_runtime::spawn_blocking(crate::scan::current_network).await.map(|n| n.key).unwrap_or_default();
    let best = {
        let store = state.store.lock().await;
        store.fastest().into_iter().next().map(|v| {
            let over = store.address_override(&v, &network);
            (v.server, over)
        })
    };
    let Some((server, over)) = best else { return };
    if server.id == current {
        return;
    }
    if state.tunnel.switch(app, &server, over.as_deref()).await.is_ok() {
        let mut store = state.store.lock().await;
        store.data.selected = Some(server.id.clone());
        let _ = store.save();
        drop(store);
        crate::desktop::notify(app, "سرور عوض شد", &format!("اتصال کند شده بود؛ به {} منتقل شد.", crate::desktop::plain(&server.name)));
    }
}

#[tauri::command]
pub async fn tunnel_disconnect(app: AppHandle, state: State<'_, ServersState>) -> Result<(), String> {
    state.tunnel.disconnect(&app).await;
    Ok(())
}
