//! The connection: geekcore running a server's config, and the system proxy
//! pointing at it.
//!
//! State changes are emitted as `tunnel://state`; the UI never polls. The
//! system proxy's previous settings are written to disk before they are
//! changed, and put back at the next start if the app died connected.

use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use geek_config::{client_config, LocalPorts, Route, Server};
use geek_core::{CoreError, CoreProcess};
use geek_netplat::{apply_system_proxy, restore_system_proxy, ProxySpec, Snapshot};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

pub const STATE_EVENT: &str = "tunnel://state";
pub const STATS_EVENT: &str = "tunnel://stats";

/// Probe for «متصل شد»: a connection only counts once traffic got through.
const PROBE_URL: &str = "https://www.gstatic.com/generate_204";

/// The probe, overridable in debug builds only (`GEEK_PROBE_URL`) so the
/// end-to-end run can use a local origin where the internet is walled off.
pub fn probe_url() -> String {
    #[cfg(debug_assertions)]
    if let Ok(u) = std::env::var("GEEK_PROBE_URL") {
        return u;
    }
    PROBE_URL.to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "status")]
pub enum TunnelState {
    Off,
    Connecting { server_id: String, attempt: u32, of: u32 },
    On {
        server_id: String,
        server_name: String,
        since_ms: u64,
        http_port: u16,
        socks_port: u16,
        delay_ms: i64,
        /// Set when the system proxy could not be changed on this desktop.
        note: Option<String>,
    },
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    down_bps: f64,
    up_bps: f64,
    down_total: i64,
    up_total: i64,
}

pub struct Tunnel {
    binary: PathBuf,
    geo: PathBuf,
    snapshot_file: PathBuf,
    core: Mutex<Option<Arc<CoreProcess>>>,
    pub state: Mutex<TunnelState>,
    proxy: Mutex<Option<Snapshot>>,
}

impl Tunnel {
    pub fn new(binary: PathBuf, geo: PathBuf, snapshot_file: PathBuf) -> Self {
        Self {
            binary,
            geo,
            snapshot_file,
            core: Mutex::new(None),
            state: Mutex::new(TunnelState::Off),
            proxy: Mutex::new(None),
        }
    }

    /// At start-up: if the last run died with the system proxy set, put the
    /// user's own settings back.
    pub fn recover(&self) {
        if let Ok(bytes) = std::fs::read(&self.snapshot_file) {
            if let Ok(snap) = serde_json::from_slice::<Snapshot>(&bytes) {
                let _ = restore_system_proxy(&snap);
            }
            let _ = std::fs::remove_file(&self.snapshot_file);
        }
    }

    /// The running core, started on first use. A dead one is replaced.
    pub async fn core(&self) -> Result<Arc<CoreProcess>, String> {
        let mut slot = self.core.lock().await;
        if let Some(c) = slot.as_ref().filter(|c| c.is_alive()) {
            return Ok(c.clone());
        }
        let c = Arc::new(CoreProcess::spawn(&self.binary, &self.geo).map_err(|e| core_message(&e))?);
        *slot = Some(c.clone());
        Ok(c)
    }

    async fn set(&self, app: &AppHandle, s: TunnelState) {
        *self.state.lock().await = s.clone();
        let _ = app.emit(STATE_EVENT, s);
    }

    /// Tries `candidates` in order until one carries traffic; at most three,
    /// as the Android app does before telling the customer.
    pub async fn connect(self: &Arc<Self>, app: &AppHandle, candidates: Vec<Server>, route: Route) -> Result<(), String> {
        let tries: Vec<Server> = candidates.into_iter().take(3).collect();
        if tries.is_empty() {
            let msg = "سروری برای اتصال نیست. اول یک سرویس یا لینک اضافه کن.".to_string();
            self.set(app, TunnelState::Failed { message: msg.clone() }).await;
            return Err(msg);
        }
        let core = match self.core().await {
            Ok(c) => c,
            Err(e) => {
                self.set(app, TunnelState::Failed { message: e.clone() }).await;
                return Err(e);
            }
        };
        let of = tries.len() as u32;
        let mut last = String::new();
        for (i, server) in tries.iter().enumerate() {
            self.set(app, TunnelState::Connecting { server_id: server.id.clone(), attempt: i as u32 + 1, of }).await;
            match self.try_one(&core, server, route).await {
                Ok((ports, delay)) => {
                    let note = match self.point_system_proxy(ports).await {
                        Ok(note) => note,
                        Err(e) => {
                            let _ = core.call("core.stop", json!({}), Duration::from_secs(5)).await;
                            self.set(app, TunnelState::Failed { message: e.clone() }).await;
                            return Err(e);
                        }
                    };
                    let since_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or_default();
                    self.set(
                        app,
                        TunnelState::On {
                            server_id: server.id.clone(),
                            server_name: server.name.clone(),
                            since_ms,
                            http_port: ports.http,
                            socks_port: ports.socks,
                            delay_ms: delay,
                            note,
                        },
                    )
                    .await;
                    self.clone().watch(app.clone(), core.clone());
                    return Ok(());
                }
                Err(e) => last = e,
            }
        }
        let _ = core.call("core.stop", json!({}), Duration::from_secs(5)).await;
        let msg = if of > 1 {
            format!("با {of} سرور امتحان شد و هیچ‌کدام وصل نشد. {last}")
        } else {
            format!("اتصال برقرار نشد. {last}")
        };
        self.set(app, TunnelState::Failed { message: msg.clone() }).await;
        Err(msg)
    }

    async fn try_one(&self, core: &CoreProcess, server: &Server, route: Route) -> Result<(LocalPorts, i64), String> {
        let ports = local_ports();
        let config = client_config(server, None, route, ports);
        core.call("core.start", json!({ "config": config }), Duration::from_secs(15))
            .await
            .map_err(|e| core_message(&e))?;
        // Started is not connected: only a request that got an answer is.
        match core.call("core.delay", json!({ "url": probe_url() }), Duration::from_secs(15)).await {
            Ok(v) => Ok((ports, v["ms"].as_i64().unwrap_or(0))),
            Err(e) => {
                // The customer sees a plain sentence; the log keeps the cause
                // for problem reports.
                eprintln!("geekvpn: probe through {} failed: {e}", server.name);
                let _ = core.call("core.stop", json!({}), Duration::from_secs(5)).await;
                Err(match e {
                    CoreError::Refused(_) => "سرور جواب نداد.".into(),
                    other => core_message(&other),
                })
            }
        }
    }

    /// Ok(Some(note)) when this desktop has no proxy setting to change: the
    /// tunnel still runs, and the note says which port to use by hand.
    async fn point_system_proxy(&self, ports: LocalPorts) -> Result<Option<String>, String> {
        let mut slot = self.proxy.lock().await;
        let spec = ProxySpec { http_port: ports.http, socks_port: ports.socks };
        // Reconnecting: the saved snapshot is still the user's, keep it.
        if let Some(prev) = slot.as_ref() {
            let _ = restore_system_proxy(prev);
        }
        let snap = match apply_system_proxy(&spec) {
            Ok(s) => s,
            Err(geek_netplat::NetError::Unsupported) => {
                return Ok(Some(format!(
                    "این محیط دسکتاپ پروکسی سیستمی ندارد؛ برنامه‌ها را روی 127.0.0.1:{} (HTTP) یا {} (SOCKS) تنظیم کن.",
                    ports.http, ports.socks
                )))
            }
            Err(e) => return Err(e.user_message()),
        };
        if let Ok(bytes) = serde_json::to_vec(&snap) {
            let _ = std::fs::write(&self.snapshot_file, bytes);
        }
        *slot = Some(snap);
        Ok(None)
    }

    async fn release_system_proxy(&self) {
        if let Some(snap) = self.proxy.lock().await.take() {
            let _ = restore_system_proxy(&snap);
        }
        let _ = std::fs::remove_file(&self.snapshot_file);
    }

    pub async fn disconnect(&self, app: &AppHandle) {
        self.release_system_proxy().await;
        if let Some(core) = self.core.lock().await.as_ref() {
            let _ = core.call("core.stop", json!({}), Duration::from_secs(5)).await;
        }
        self.set(app, TunnelState::Off).await;
    }

    /// While connected: live speed once a second, and a crash of the core
    /// turned into a plain message instead of a silent dead proxy.
    fn watch(self: Arc<Self>, app: AppHandle, core: Arc<CoreProcess>) {
        let mut events = core.events();
        tauri::async_runtime::spawn(async move {
            let mut last: Option<(Instant, i64, i64)> = None;
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            loop {
                tokio::select! {
                    ev = events.recv() => {
                        if matches!(ev, Ok(ref e) if e.event == "core.exited") {
                            self.release_system_proxy().await;
                            let tail = core.log_tail();
                            let reason = tail.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").to_string();
                            self.set(&app, TunnelState::Failed {
                                message: format!("هسته‌ی اتصال بسته شد و اتصال قطع شد. پروکسی سیستم به حالت قبل برگشت. {reason}"),
                            }).await;
                            return;
                        }
                    }
                    _ = tick.tick() => {
                        if !matches!(*self.state.lock().await, TunnelState::On { .. }) {
                            return;
                        }
                        let Ok(t) = core.call("core.traffic", json!({}), Duration::from_secs(3)).await else { continue };
                        let (up, down) = (t["up"].as_i64().unwrap_or(0), t["down"].as_i64().unwrap_or(0));
                        let now = Instant::now();
                        if let Some((at, up0, down0)) = last {
                            let secs = now.duration_since(at).as_secs_f64().max(0.001);
                            let _ = app.emit(STATS_EVENT, Stats {
                                down_bps: (down - down0).max(0) as f64 / secs,
                                up_bps: (up - up0).max(0) as f64 / secs,
                                down_total: down,
                                up_total: up,
                            });
                        }
                        last = Some((now, up, down));
                    }
                }
            }
        });
    }

    /// Quitting the app: leave the machine as it was.
    pub async fn shutdown(&self) {
        self.release_system_proxy().await;
        if let Some(core) = self.core.lock().await.take() {
            core.kill().await;
        }
    }
}

/// 10808/10809 when free (people point other apps at them by hand, as with
/// v2rayN), otherwise whatever the OS gives.
fn local_ports() -> LocalPorts {
    let pick = |want: u16| {
        TcpListener::bind(("127.0.0.1", want))
            .or_else(|_| TcpListener::bind(("127.0.0.1", 0)))
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .unwrap_or(want)
    };
    let socks = pick(10808);
    let mut http = pick(10809);
    if http == socks {
        http = pick(0);
    }
    LocalPorts { socks, http }
}

pub fn core_message(e: &CoreError) -> String {
    match e {
        CoreError::Spawn(_) => "هسته‌ی اتصال (geekcore) اجرا نشد. برنامه را دوباره نصب کن.".into(),
        CoreError::Exited { tail } => {
            let last = tail.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
            format!("هسته‌ی اتصال بسته شد. {last}")
        }
        CoreError::Timeout(_) => "هسته‌ی اتصال جواب نداد.".into(),
        CoreError::Refused(r) => format!("کانفیگ این سرور اجرا نشد: {r}"),
    }
}
