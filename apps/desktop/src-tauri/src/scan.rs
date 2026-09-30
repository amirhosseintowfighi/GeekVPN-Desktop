//! «بهینه‌ساز کلادفلر»: the clean-IP scanner, as on Android. A scan runs in
//! geekcore (the same Go `cfscan` module), its best address is put in place
//! for the network it was found on, and every connect and delay test on that
//! network uses it instead of the server's own address.

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::Duration;

use geek_config::{rank, scan_config, verdict, CdnTarget, CfRanges, CleanIp, KEEP};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::OnceCell;

use crate::servers::ServersState;
use crate::store::{CdnVerdict, IpOverride, ScanRecord, ServerView, Store};

pub const PROGRESS: &str = "scan://progress";
const VERDICT_MAX_AGE_MS: i64 = 24 * 60 * 60 * 1000;

/// The network a clean address was found on. Which Cloudflare addresses get
/// through depends on the ISP, so results and overrides are kept per network:
/// told apart by the gateway (its MAC and address) and the DNS servers the
/// network handed out, never by anything that needs a permission.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    pub key: String,
    /// "Wi-Fi", "Ethernet" or the adapter's own name, for the UI.
    pub label: String,
}

/// The network underneath: our own TUN device is skipped, it is not one.
pub fn current_network() -> Network {
    let tun = |name: &str| {
        let n = name.to_ascii_lowercase();
        n.starts_with("geekvpn") || n.starts_with("utun") || n.starts_with("tun") || n.starts_with("wg") || n.starts_with("wintun")
    };
    let default = netdev::get_default_interface().ok().filter(|i| !tun(&i.name) && i.gateway.is_some());
    let iface = default.or_else(|| netdev::get_interfaces().into_iter().find(|i| i.gateway.is_some() && !tun(&i.name) && !i.is_loopback()));
    let Some(i) = iface else {
        return Network { key: "other".into(), label: "نامشخص".into() };
    };
    let gw = i.gateway.as_ref().map(|g| format!("{}|{:?}", g.mac_addr, g.ipv4)).unwrap_or_default();
    let mut dns: Vec<String> = i.dns_servers.iter().map(|d| d.to_string()).collect();
    dns.sort();
    let digest = Sha256::digest(format!("{gw}|{}", dns.join(",")).as_bytes());
    // Windows names adapters for people ("Wi-Fi 2"); elsewhere the type says more than "wlp2s0".
    let label = match (i.friendly_name.clone().filter(|n| !n.is_empty() && cfg!(windows)), i.if_type) {
        (Some(n), _) => n,
        (None, netdev::interface::types::InterfaceType::Wireless80211) => "Wi-Fi".into(),
        (None, netdev::interface::types::InterfaceType::Ethernet) => "کابل شبکه".into(),
        _ => i.name.clone(),
    };
    Network { key: format!("net:{}", digest[..6].iter().map(|b| format!("{b:02x}")).collect::<String>()), label }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

/// The bundled cf-scanner list: the scan's ranges, and what "on Cloudflare"
/// means for a domain.
pub struct Ranges {
    path: PathBuf,
    text: OnceCell<(String, std::sync::Arc<CfRanges>)>,
}

impl Ranges {
    pub fn new(path: PathBuf) -> Self {
        Self { path, text: OnceCell::new() }
    }

    async fn get(&self) -> Result<&(String, std::sync::Arc<CfRanges>), String> {
        self.text
            .get_or_try_init(|| async {
                let text = tokio::fs::read_to_string(&self.path).await.map_err(|e| format!("{}: {e}", self.path.display()))?;
                let parsed = std::sync::Arc::new(CfRanges::parse(&text));
                Ok::<_, String>((text, parsed))
            })
            .await
    }
}

/// Whether the domain's SNI and Host resolve to Cloudflare, rechecked after
/// a day. False from either is false; no public address is no verdict.
async fn verify(app: &AppHandle, target: &CdnTarget) -> Result<Option<bool>, String> {
    let state = app.state::<ServersState>();
    if let Some(v) = state.store.lock().await.data.scan.verdicts.get(&target.verdict_key()) {
        if now_ms() - v.checked_at < VERDICT_MAX_AGE_MS {
            return Ok(Some(v.behind));
        }
    }
    let ranges = app.state::<Ranges>();
    let (_, table) = ranges.get().await?;
    let mut answers = vec![];
    for name in [&target.sni, &target.host] {
        // A filtered name can hang the resolver; no answer in time is no verdict.
        let addrs: Vec<Ipv4Addr> = match tokio::time::timeout(Duration::from_secs(5), tokio::net::lookup_host((name.as_str(), 443))).await {
            Ok(Ok(it)) => it.filter_map(|a| if let std::net::IpAddr::V4(v4) = a.ip() { Some(v4) } else { None }).collect(),
            _ => vec![],
        };
        answers.push(verdict(table, &addrs));
    }
    let result = if answers.contains(&Some(false)) {
        Some(false)
    } else if answers.iter().all(|a| *a == Some(true)) {
        Some(true)
    } else {
        None
    };
    if let Some(behind) = result {
        state.store.lock().await.data.scan.verdicts.insert(target.verdict_key(), CdnVerdict { behind, checked_at: now_ms() });
    }
    Ok(result)
}

// -- State for the Tools page -----------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    id: String,
    name: String,
    sni: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanView {
    network: Network,
    /// Servers the scanner applies to.
    candidates: Vec<Candidate>,
    /// The one shown: the selected server if it is a candidate.
    server_id: Option<String>,
    results: Vec<CleanIp>,
    scanned_at: Option<i64>,
    /// The address in use on this network instead of the server's own.
    in_use: Option<String>,
    /// Whether the domain is known to be on Cloudflare (null: not checked).
    behind_cloudflare: Option<bool>,
    download_test: bool,
    running: bool,
}

pub struct ScanRun {
    pub running: tokio::sync::Mutex<Option<String>>,
}

fn view_of(store: &Store, network: Network, want: Option<String>, running: Option<String>) -> ScanView {
    let candidates: Vec<(ServerView, CdnTarget)> =
        store.servers().into_iter().filter_map(|v| store.scan_target(&v).map(|t| (v, t))).collect();
    let pick = want.or_else(|| store.data.selected.clone()).filter(|id| candidates.iter().any(|(v, _)| &v.server.id == id));
    let pick = pick.or_else(|| candidates.first().map(|(v, _)| v.server.id.clone()));
    let chosen = pick.as_ref().and_then(|id| candidates.iter().find(|(v, _)| &v.server.id == id));
    let record = chosen.and_then(|(_, t)| store.data.scan.results.get(&t.result_key(&network.key)));
    ScanView {
        candidates: candidates.iter().map(|(v, t)| Candidate { id: v.server.id.clone(), name: v.server.name.clone(), sni: t.sni.clone() }).collect(),
        server_id: pick.clone(),
        results: record.map(|r| r.results.clone()).unwrap_or_default(),
        scanned_at: record.map(|r| r.scanned_at),
        in_use: chosen.and_then(|(v, _)| store.data.scan.overrides.get(&Store::override_key(v, &network.key)).map(|o| o.ip.clone())),
        behind_cloudflare: chosen.and_then(|(_, t)| store.data.scan.verdicts.get(&t.verdict_key()).map(|d| d.behind)),
        download_test: store.data.scan.download_test,
        running: running.is_some_and(|r| Some(&r) == pick.as_ref()),
        network,
    }
}

#[tauri::command]
pub async fn scan_state(state: State<'_, ServersState>, run: State<'_, ScanRun>, server_id: Option<String>) -> Result<ScanView, String> {
    let network = tauri::async_runtime::spawn_blocking(current_network).await.map_err(|e| e.to_string())?;
    let running = run.running.lock().await.clone();
    Ok(view_of(&*state.store.lock().await, network, server_id, running))
}

/// «شروع اسکن» for one server. Runs to the end (five clean addresses, or
/// 300 tried); progress and results arrive as `scan://progress`.
/// The address put in place, or none when nothing clean answered.
#[tauri::command]
pub async fn scan_start(app: AppHandle, server_id: String, download: bool) -> Result<Option<String>, String> {
    run_scan(&app, &server_id, download, 5, None).await
}

/// The scan itself; also smart connect's short scan (`stop_after`,
/// `timeout`). Returns the best address it put in place.
pub async fn run_scan(app: &AppHandle, server_id: &str, download: bool, stop_after: u32, timeout: Option<Duration>) -> Result<Option<String>, String> {
    let state = app.state::<ServersState>();
    let run = app.state::<ScanRun>();
    {
        let mut r = run.running.lock().await;
        if r.is_some() {
            return Err("یک اسکن در حال اجراست.".into());
        }
        *r = Some(server_id.to_string());
    }
    let result = scan_inner(app, &state, server_id, download, stop_after, timeout).await;
    *run.running.lock().await = None;
    let _ = app.emit(PROGRESS, json!({ "kind": "finish", "error": result.as_ref().err() }));
    result
}

async fn scan_inner(
    app: &AppHandle,
    state: &ServersState,
    server_id: &str,
    download: bool,
    stop_after: u32,
    timeout: Option<Duration>,
) -> Result<Option<String>, String> {
    let (view, target) = {
        let store = state.store.lock().await;
        let v = store.find_view(server_id).ok_or("این سرور دیگر در فهرست نیست.")?;
        let t = store.scan_target(&v).ok_or("اسکنر فقط برای سرویس‌های مستقیم پشت کلادفلر است.")?;
        (v, t)
    };
    // /cdn-cgi/trace answers on any Cloudflare address whatever the domain,
    // so the scan itself cannot tell whether this domain is on Cloudflare.
    if verify(app, &target).await? == Some(false) {
        return Err("دامنه‌ی این سرور پشت کلادفلر نیست؛ IP تمیز به کارش نمی‌آید.".into());
    }
    let network = tauri::async_runtime::spawn_blocking(current_network).await.map_err(|e| e.to_string())?;
    let known: Vec<String> = state
        .store
        .lock()
        .await
        .data
        .scan
        .results
        .get(&target.result_key(&network.key))
        .map(|r| r.results.iter().map(|c| c.ip.clone()).collect())
        .unwrap_or_default();
    let ranges = app.state::<Ranges>();
    let (text, _) = ranges.get().await?;
    let core = state.tunnel.core().await?;
    let mut events = core.events();
    let found = std::sync::Arc::new(std::sync::Mutex::new(Vec::<CleanIp>::new()));
    let (done_tx, done_rx) = tokio::sync::oneshot::channel::<Option<String>>();
    {
        let (app, found) = (app.clone(), found.clone());
        tauri::async_runtime::spawn(async move {
            let mut done_tx = Some(done_tx);
            loop {
                let ev = match events.recv().await {
                    Ok(ev) => ev,
                    // Behind a burst of progress: skip ahead, keep listening.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                };
                match ev.event.as_str() {
                    "scan.result" => {
                        if let Ok(ip) = serde_json::from_value::<CleanIp>(ev.data.clone()) {
                            found.lock().unwrap().push(ip.clone());
                            let _ = app.emit(PROGRESS, json!({ "kind": "result", "result": ip }));
                        }
                    }
                    "scan.progress" => {
                        let _ = app.emit(PROGRESS, json!({ "kind": "progress", "tested": ev.data["tested"], "total": ev.data["total"], "found": ev.data["found"] }));
                    }
                    "scan.finish" | "core.exited" => {
                        let err = ev.data["error"].as_str().filter(|e| !e.is_empty()).map(String::from);
                        if let Some(tx) = done_tx.take() {
                            let _ = tx.send(err);
                        }
                        return;
                    }
                    _ => {}
                }
            }
        });
    }
    core.call("scan.start", json!({ "config": scan_config(&target, &known, download, stop_after), "ranges": text }), Duration::from_secs(10))
        .await
        .map_err(|e| crate::tunnel::core_message(&e))?;
    let finished = match timeout {
        Some(t) => match tokio::time::timeout(t, done_rx).await {
            Ok(r) => r,
            Err(_) => {
                // Smart connect's short scan: stop where it is, keep what it found.
                let _ = core.call("scan.stop", json!({}), Duration::from_secs(5)).await;
                Ok(None)
            }
        },
        None => done_rx.await,
    };
    if let Ok(Some(err)) = finished {
        eprintln!("geekvpn: scan finished with: {err}");
    }
    let found = found.lock().unwrap().clone();
    if found.is_empty() {
        return Ok(None);
    }
    let now = now_ms();
    let mut store = state.store.lock().await;
    let best = rank(&found).into_iter().take(KEEP).collect::<Vec<_>>();
    let first = best.first().cloned();
    store.data.scan.results.insert(target.result_key(&network.key), ScanRecord { results: best, scanned_at: now });
    // The customer ran this scan: where DNS gave no verdict, take the domain
    // as Cloudflare's.
    store.data.scan.verdicts.entry(target.verdict_key()).or_insert(CdnVerdict { behind: true, checked_at: now });
    if let Some(b) = &first {
        store.data.scan.overrides.insert(Store::override_key(&view, &network.key), IpOverride { ip: b.ip.clone(), applied_at: now, latency_ms: b.latency_ms });
    }
    store.save()?;
    Ok(first.map(|b| b.ip))
}

#[tauri::command]
pub async fn scan_stop(state: State<'_, ServersState>) -> Result<(), String> {
    let core = state.tunnel.core().await?;
    let _ = core.call("scan.stop", json!({}), Duration::from_secs(5)).await;
    Ok(())
}

/// «استفاده» on a result, or «بازگشت به IP اصلی» with no `ip`: from the next
/// connect (or at once, when connected to this server).
#[tauri::command]
pub async fn scan_use(app: AppHandle, state: State<'_, ServersState>, server_id: String, ip: Option<String>) -> Result<(), String> {
    let network = tauri::async_runtime::spawn_blocking(current_network).await.map_err(|e| e.to_string())?;
    {
        let mut store = state.store.lock().await;
        let v = store.find_view(&server_id).ok_or("این سرور دیگر در فهرست نیست.")?;
        let key = Store::override_key(&v, &network.key);
        match ip {
            Some(ip) => {
                let latency = v.delay_ms.unwrap_or(0);
                store.data.scan.overrides.insert(key, IpOverride { ip, applied_at: now_ms(), latency_ms: latency });
                if let Some(t) = store.scan_target(&v) {
                    store.data.scan.verdicts.entry(t.verdict_key()).or_insert(CdnVerdict { behind: true, checked_at: now_ms() });
                }
            }
            None => {
                store.data.scan.overrides.remove(&key);
            }
        }
        store.save()?;
    }
    // Connected to this server: reconnect so the change takes effect.
    let connected = matches!(&*state.tunnel.state.lock().await, crate::tunnel::TunnelState::On { server_id: s, .. } if *s == server_id);
    if connected {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = crate::servers::connect(&app).await;
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn scan_set_download(state: State<'_, ServersState>, enabled: bool) -> Result<(), String> {
    let mut store = state.store.lock().await;
    store.data.scan.download_test = enabled;
    store.save()
}
