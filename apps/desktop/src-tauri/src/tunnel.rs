//! The connection: geekcore running a server's config, and either the
//! system proxy pointing at it or, in TUN mode, geekvpn-helper's sing-box
//! feeding it every program's traffic.
//!
//! State changes are emitted as `tunnel://state`; the UI never polls. The
//! system proxy's previous settings are written to disk before they are
//! changed, and put back at the next start if the app died connected. In
//! TUN mode the helper owns the device and the kill switch; if the app dies,
//! the helper sees the connection close and cleans up itself.

use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use geek_config::{client_config, iran_rules, tun_upstream_config, AppRouting, CustomRule, LocalPorts, Route, RuleLists, Server, TunSpec, Upstream};
use geek_core::{CoreError, CoreProcess};
use geek_ipc::{ErrorCode, Event, Hello, HelperClient, IpcError, KillSwitch, Request, TunInfo, PROTOCOL};
use geek_netplat::{apply_system_proxy, restore_system_proxy, ProxySpec, Snapshot};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, OnceCell};

use crate::store::Mode;

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

/// Everything a connect needs besides the servers.
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    pub route: Route,
    pub mode: Mode,
    /// TUN mode only.
    pub kill_switch: Option<KillSwitch>,
    pub apps: AppRouting,
    pub rules: Vec<CustomRule>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "status")]
pub enum TunnelState {
    Off,
    Connecting { server_id: String, attempt: u32, of: u32, stage: Stage },
    On {
        server_id: String,
        server_name: String,
        since_ms: u64,
        mode: Mode,
        /// The local proxy ports (system-proxy mode; 0 in TUN mode).
        http_port: u16,
        socks_port: u16,
        delay_ms: i64,
        kill_switch: bool,
        /// Set when the system proxy could not be changed on this desktop.
        note: Option<String>,
    },
    Failed {
        message: String,
        /// The kill switch still blocks the internet: the UI says so, and
        /// «قطع» is what opens it.
        blocking: bool,
    },
}

/// What a connect is doing, for Home's progress (smart connect's stages).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    /// A short clean-IP scan: this network has no fresh results.
    FindingIp,
    /// The real-delay test over the servers (and clean addresses).
    Testing,
    Connecting,
}

/// The running connection, for failover to swap its server in place: the
/// same local ports (or TUN upstream), so the system proxy and sing-box
/// never notice.
#[derive(Clone)]
struct Live {
    mode: Mode,
    route: Route,
    rules: Vec<CustomRule>,
    ports: LocalPorts,
    upstream: Upstream,
    config: serde_json::Value,
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
    helper: Mutex<Option<Arc<HelperClient>>>,
    /// The running TUN (for the Connections page), and whether its kill
    /// switch is strict (a clean quit then leaves it to the helper).
    pub tun: Mutex<Option<(TunInfo, bool)>>,
    iran: OnceCell<RuleLists>,
    /// Bytes up and down of the running connection, for the summary after it.
    totals: std::sync::Mutex<(i64, i64)>,
    live: Mutex<Option<Live>>,
    /// «مصرف روزانه» of this computer.
    pub usage: crate::usage::Usage,
    /// The last failure and when (ms), for «گزارش مشکل».
    pub last_failure: std::sync::Mutex<Option<(String, u64)>>,
}

impl Tunnel {
    pub fn new(binary: PathBuf, geo: PathBuf, snapshot_file: PathBuf, usage_file: PathBuf) -> Self {
        Self {
            binary,
            geo,
            snapshot_file,
            core: Mutex::new(None),
            state: Mutex::new(TunnelState::Off),
            proxy: Mutex::new(None),
            helper: Mutex::new(None),
            tun: Mutex::new(None),
            iran: OnceCell::new(),
            totals: std::sync::Mutex::new((0, 0)),
            live: Mutex::new(None),
            usage: crate::usage::Usage::load(usage_file),
            last_failure: std::sync::Mutex::new(None),
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

    /// The helper, connected on first use, with a matching protocol.
    pub async fn helper(&self) -> Result<(Arc<HelperClient>, Hello), IpcError> {
        let mut slot = self.helper.lock().await;
        if let Some(h) = slot.as_ref().filter(|h| h.is_alive()) {
            let hello = h.call(Request::Hello, Duration::from_secs(5)).await?;
            return Ok((h.clone(), hello));
        }
        let h = Arc::new(HelperClient::connect(geek_ipc::ENDPOINT).await?);
        let hello: Hello = h.call(Request::Hello, Duration::from_secs(5)).await?;
        *slot = Some(h.clone());
        Ok((h, hello))
    }

    async fn set(&self, app: &AppHandle, s: TunnelState) {
        let before = std::mem::replace(&mut *self.state.lock().await, s.clone());
        let totals = *self.totals.lock().unwrap();
        if matches!(s, TunnelState::Connecting { .. }) {
            *self.totals.lock().unwrap() = (0, 0);
        }
        crate::desktop::on_state(app, &before, &s, totals);
        let _ = app.emit(STATE_EVENT, s);
    }

    async fn fail(&self, app: &AppHandle, message: String, blocking: bool) -> String {
        *self.last_failure.lock().unwrap() = Some((message.clone(), now_ms()));
        self.set(app, TunnelState::Failed { message: message.clone(), blocking }).await;
        message
    }

    /// Tries `candidates` in order until one carries traffic; at most three,
    /// as the Android app does before telling the customer.
    /// Smart connect's stages before a server is tried.
    pub async fn stage(&self, app: &AppHandle, stage: Stage) {
        self.set(app, TunnelState::Connecting { server_id: String::new(), attempt: 0, of: 0, stage }).await;
    }

    /// `candidates` are servers with the clean address each uses on this
    /// network, if any.
    pub async fn connect(self: &Arc<Self>, app: &AppHandle, candidates: Vec<(Server, Option<String>)>, opts: ConnectOptions) -> Result<(), String> {
        let tries: Vec<(Server, Option<String>)> = candidates.into_iter().take(3).collect();
        if tries.is_empty() {
            return Err(self.fail(app, "سروری برای اتصال نیست. اول یک سرویس یا لینک اضافه کن.".into(), false).await);
        }
        // Switching modes: the other mode's pieces go first.
        match opts.mode {
            Mode::Proxy => self.stop_tun(false).await,
            Mode::Tun => self.release_system_proxy().await,
        }
        let core = match self.core().await {
            Ok(c) => c,
            Err(e) => return Err(self.fail(app, e, false).await),
        };
        // TUN mode needs the helper; better to say so before trying servers.
        let helper = match opts.mode {
            Mode::Proxy => None,
            Mode::Tun => match self.helper().await {
                Ok((h, hello)) if hello.protocol == PROTOCOL => Some(h),
                Ok(_) => return Err(self.fail(app, HELPER_OUTDATED.into(), false).await),
                Err(e) => return Err(self.fail(app, helper_message(&e), false).await),
            },
        };
        let of = tries.len() as u32;
        let mut last = String::new();
        for (i, (server, over)) in tries.iter().enumerate() {
            self.set(app, TunnelState::Connecting { server_id: server.id.clone(), attempt: i as u32 + 1, of, stage: Stage::Connecting }).await;
            let upstream = Upstream { port: free_port(), username: token(), password: token() };
            let ports = local_ports();
            let config = match opts.mode {
                Mode::Proxy => client_config(server, over.as_deref(), opts.route, &opts.rules, ports),
                Mode::Tun => tun_upstream_config(server, over.as_deref(), &upstream),
            };
            match self.try_one(&core, server, &config).await {
                Ok(delay) => {
                    let brought_up = match &helper {
                        None => self.point_system_proxy(&config).await.map(|note| (note, false)),
                        Some(h) => self.tun_up(h, &upstream, &opts, server, over.as_deref()).await.map(|()| (None, opts.kill_switch.is_some())),
                    };
                    let (note, kill_switch) = match brought_up {
                        Ok(x) => x,
                        Err(e) => {
                            let _ = core.call("core.stop", json!({}), Duration::from_secs(5)).await;
                            // A tunnel that never came up protects nothing: the
                            // kill switch goes too, so retrying (or fetching a
                            // subscription) works.
                            if let Some(h) = &helper {
                                let _ = h.call::<serde_json::Value>(Request::TunStop, Duration::from_secs(10)).await;
                            }
                            return Err(self.fail(app, e, false).await);
                        }
                    };
                    let since_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or_default();
                    *self.live.lock().await = Some(Live { mode: opts.mode, route: opts.route, rules: opts.rules.clone(), ports, upstream: upstream.clone(), config: config.clone() });
                    let port = |name: &str| config["inbounds"].as_array().into_iter().flatten().find(|i| i["tag"] == name).and_then(|i| i["port"].as_u64());
                    self.set(
                        app,
                        TunnelState::On {
                            server_id: server.id.clone(),
                            server_name: server.name.clone(),
                            since_ms,
                            mode: opts.mode,
                            http_port: if helper.is_none() { port("http").unwrap_or(0) as u16 } else { 0 },
                            socks_port: if helper.is_none() { port("socks").unwrap_or(0) as u16 } else { 0 },
                            delay_ms: delay,
                            kill_switch,
                            note,
                        },
                    )
                    .await;
                    self.clone().watch(app.clone(), core.clone(), helper.clone(), kill_switch);
                    self.clone().monitor(app.clone(), core.clone(), since_ms);
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
        Err(self.fail(app, msg, false).await)
    }

    /// Starts `config` and probes through it; the delay in ms.
    async fn try_one(&self, core: &CoreProcess, server: &Server, config: &serde_json::Value) -> Result<i64, String> {
        core.call("core.start", json!({ "config": config }), Duration::from_secs(15))
            .await
            .map_err(|e| core_message(&e))?;
        // Started is not connected: only a request that got an answer is.
        match core.call("core.delay", json!({ "url": probe_url() }), Duration::from_secs(15)).await {
            Ok(v) => Ok(v["ms"].as_i64().unwrap_or(0)),
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

    /// Hands the helper the TUN spec for this connection.
    /// `server` + `over` give the actual IP the engine will dial, so the
    /// helper can add a bypass route for it (Windows hev has no per-process
    /// routing: without this its own TCP loops through the TUN and the link
    /// goes dead — the "internet cuts on TUN" bug).
    async fn tun_up(
        &self,
        helper: &HelperClient,
        upstream: &Upstream,
        opts: &ConnectOptions,
        server: &Server,
        over: Option<&str>,
    ) -> Result<(), String> {
        let direct = if opts.route == Route::Smart { self.iran().await? } else { RuleLists::default() };
        let core_path = self.binary.canonicalize().unwrap_or_else(|_| self.binary.clone());
        let bypass = Self::tun_bypass(server, over).await;
        let spec = TunSpec {
            upstream: upstream.clone(),
            route: opts.route,
            direct,
            apps: opts.apps.clone(),
            core_paths: vec![core_path.to_string_lossy().into_owned()],
            rules: opts.rules.clone(),
            bypass,
        };
        let info: TunInfo = helper
            .call(Request::TunStart { spec: Box::new(spec), kill_switch: opts.kill_switch }, Duration::from_secs(30))
            .await
            .map_err(|e| helper_message(&e))?;
        *self.tun.lock().await = Some((info, opts.kill_switch.is_some_and(|k| k.strict)));
        Ok(())
    }

    async fn tun_bypass(server: &Server, over: Option<&str>) -> Vec<String> {
        // `over` is the clean-IP chosen by the scanner (numeric). Prefer it.
        if let Some(ip) = over {
            let s = ip.trim();
            if s.parse::<std::net::IpAddr>().is_ok() {
                return vec![s.to_string()];
            }
        }
        let addr = server.address.trim();
        if addr.is_empty() { return vec![]; }
        if addr.parse::<std::net::IpAddr>().is_ok() {
            return vec![addr.to_string()];
        }
        // Domain (CDN-fronted WS/XHTTP etc.): hev's default routes (0.0.0.0/1)
        // would otherwise send geekcore's own TCP to this domain back through
        // the TUN, looping forever and cutting the internet. WFP App bypass
        // does NOT change routing, so we must add a concrete /32 via the real
        // gateway. Resolve the domain now (before TUN is up) and return those IPs.
        let host = addr.trim_matches(|c| c == '[' || c == ']');
        let port = server.port;
        // tokio DNS; 3s cap so a bad DNS doesn't stall connect.
        let addrs = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            tokio::net::lookup_host((host, port)),
        )
        .await;
        if let Ok(Ok(iter)) = addrs {
            let mut out: Vec<String> = iter.map(|sa| sa.ip().to_string()).collect();
            out.sort_unstable();
            out.dedup();
            if !out.is_empty() {
                // Prefer IPv4 first (most servers are v4); keep both.
                return out;
            }
        }
        // Fallback: could not resolve — let WFP App filter carry it, but log
        // so the helper's `add_bypass` warning is visible.
        eprintln!("geekvpn: tun_bypass could not resolve {host}, no bypass route");
        vec![]
    }

    /// Iran's lists from the shipped geo files, read once.
    async fn iran(&self) -> Result<RuleLists, String> {
        let geo = self.geo.clone();
        self.iran
            .get_or_try_init(|| async move {
                tokio::task::spawn_blocking(move || {
                    let ip = std::fs::read(geo.join("geoip.dat")).map_err(|e| e.to_string())?;
                    let site = std::fs::read(geo.join("geosite.dat")).map_err(|e| e.to_string())?;
                    iran_rules(&ip, &site).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())?
            })
            .await
            .cloned()
            .map_err(|e: String| format!("فهرست سایت‌های ایران خوانده نشد؛ برنامه را دوباره نصب کن. ({e})"))
    }

    /// Ok(Some(note)) when this desktop has no proxy setting to change: the
    /// tunnel still runs, and the note says which port to use by hand.
    async fn point_system_proxy(&self, config: &serde_json::Value) -> Result<Option<String>, String> {
        let port = |name: &str| {
            config["inbounds"].as_array().into_iter().flatten().find(|i| i["tag"] == name).and_then(|i| i["port"].as_u64()).unwrap_or(0) as u16
        };
        let spec = ProxySpec { http_port: port("http"), socks_port: port("socks") };
        let mut slot = self.proxy.lock().await;
        // Reconnecting: the saved snapshot is still the user's, keep it.
        if let Some(prev) = slot.as_ref() {
            if let Err(e) = restore_system_proxy(prev) {
                eprintln!("geekvpn: restore_system_proxy (reconnect) failed: {e}");
            }
        }
        let snap = match apply_system_proxy(&spec) {
            Ok(s) => s,
            Err(geek_netplat::NetError::Unsupported) => {
                return Ok(Some(format!(
                    "این محیط دسکتاپ پروکسی سیستمی ندارد؛ برنامه‌ها را روی 127.0.0.1:{} (HTTP) یا {} (SOCKS) تنظیم کن، یا به حالت TUN برو.",
                    spec.http_port, spec.socks_port
                )))
            }
            Err(e) => return Err(e.user_message()),
        };
        if let Ok(bytes) = serde_json::to_vec(&snap) {
            let _ = std::fs::write(&self.snapshot_file, bytes);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&self.snapshot_file, std::fs::Permissions::from_mode(0o600));
            }
        }
        *slot = Some(snap);
        Ok(None)
    }

    async fn release_system_proxy(&self) {
        if let Some(snap) = self.proxy.lock().await.take() {
            if let Err(e) = restore_system_proxy(&snap) {
                eprintln!("geekvpn: restore_system_proxy failed: {e}");
            }
        }
        let _ = std::fs::remove_file(&self.snapshot_file);
    }

    /// Stops the TUN and releases the kill switch. With `leave_strict`, a
    /// strict kill switch is left to the helper instead (quitting the app).
    async fn stop_tun(&self, leave_strict: bool) {
        let strict = self.tun.lock().await.take().is_some_and(|(_, strict)| strict);
        let helper = self.helper.lock().await.clone();
        let Some(h) = helper.filter(|h| h.is_alive()) else { return };
        if leave_strict && strict {
            // Closing our connection stops the tunnel; the helper keeps a
            // strict kill switch engaged.
            self.helper.lock().await.take();
            return;
        }
        if let Err(e) = h.call::<serde_json::Value>(Request::TunStop, Duration::from_secs(10)).await {
            eprintln!("geekvpn: TunStop failed: {e}");
        }
    }

    pub async fn disconnect(&self, app: &AppHandle) {
        let was_live = self.live.lock().await.take().is_some();
        self.release_system_proxy().await;
        self.stop_tun(false).await;
        if let Some(core) = self.core.lock().await.as_ref() {
            // The last second's bytes, before the counters go with the engine.
            if was_live {
                if let Ok(t) = core.call("core.traffic", json!({}), Duration::from_secs(2)).await {
                    self.usage.sample(t["up"].as_i64().unwrap_or(0) + t["down"].as_i64().unwrap_or(0));
                }
            }
            let _ = core.call("core.stop", json!({}), Duration::from_secs(5)).await;
        }
        self.usage.flush();
        self.set(app, TunnelState::Off).await;
    }

    /// While connected: live speed once a second, and a crash of the core,
    /// of sing-box or of the helper turned into a plain message instead of
    /// a silent dead tunnel.
    fn watch(self: Arc<Self>, app: AppHandle, core: Arc<CoreProcess>, helper: Option<Arc<HelperClient>>, kill_switch: bool) {
        let mut events = core.events();
        let mut helper_events = helper.as_ref().map(|h| h.events());
        self.usage.begin();
        tauri::async_runtime::spawn(async move {
            let mut last: Option<(Instant, i64, i64)> = None;
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            loop {
                let helper_event = async {
                    match helper_events.as_mut() {
                        Some(rx) => rx.recv().await.ok(),
                        None => std::future::pending().await,
                    }
                };
                tokio::select! {
                    ev = events.recv() => {
                        if matches!(ev, Ok(ref e) if e.event == "core.exited") {
                            let tail = core.log_tail();
                            let reason = tail.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").to_string();
                            self.release_system_proxy().await;
                            let message = if kill_switch {
                                // sing-box now feeds a dead core: nothing
                                // leaves, which is what the kill switch is for.
                                format!("هسته‌ی اتصال بسته شد. Kill Switch اینترنت را بسته نگه داشته تا دوباره وصل شوی یا «قطع» را بزنی. {reason}")
                            } else {
                                self.stop_tun(false).await;
                                format!("هسته‌ی اتصال بسته شد و اتصال قطع شد. تنظیمات شبکه به حالت قبل برگشت. {reason}")
                            };
                            self.usage.flush();
                            self.fail(&app, message, kill_switch).await;
                            return;
                        }
                    }
                    ev = helper_event => {
                        // sing-box died: the helper keeps the kill switch.
                        // The helper itself went away: with it, its firewall
                        // (Linux's nftables table is re-checked at its restart).
                        let tun_died = match ev {
                            Some(Event::TunExited { detail }) => {
                                eprintln!("geekvpn: {detail}");
                                true
                            }
                            None => false,
                        };
                        self.tun.lock().await.take();
                        let _ = core.call("core.stop", json!({}), Duration::from_secs(5)).await;
                        let blocking = kill_switch && tun_died;
                        let text = match (tun_died, blocking) {
                            (true, true) => "حالت TUN از کار افتاد. Kill Switch اینترنت را بسته نگه داشته تا دوباره وصل شوی یا «قطع» را بزنی.",
                            (true, false) => "حالت TUN از کار افتاد و اتصال قطع شد.",
                            _ => "سرویس GeekVPN بسته شد و اتصال قطع شد.",
                        };
                        self.usage.flush();
                        self.fail(&app, text.into(), blocking).await;
                        return;
                    }
                    _ = tick.tick() => {
                        if !matches!(*self.state.lock().await, TunnelState::On { .. }) {
                            self.usage.flush();
                            return;
                        }
                        let Ok(t) = core.call("core.traffic", json!({}), Duration::from_secs(3)).await else { continue };
                        let (up, down) = (t["up"].as_i64().unwrap_or(0), t["down"].as_i64().unwrap_or(0));
                        *self.totals.lock().unwrap() = (up, down);
                        self.usage.sample(up + down);
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

    /// Failover: the running connection moves to `server` in place, on the
    /// same local ports (or TUN upstream), so neither the system proxy nor
    /// sing-box sees a change. If the new server does not carry traffic,
    /// the old config goes back.
    pub async fn switch(&self, app: &AppHandle, server: &Server, over: Option<&str>) -> Result<i64, String> {
        let Some(live) = self.live.lock().await.clone() else { return Err("not connected".into()) };
        let core = self.core().await?;
        let config = match live.mode {
            Mode::Proxy => client_config(server, over, live.route, &live.rules, live.ports),
            Mode::Tun => tun_upstream_config(server, over, &live.upstream),
        };
        match self.try_one(&core, server, &config).await {
            Ok(delay) => {
                *self.live.lock().await = Some(Live { config, ..live });
                let mut state = self.state.lock().await;
                // The connection goes on (and its timer with it); only the
                // server changes.
                if let TunnelState::On { server_id, server_name, delay_ms, .. } = &mut *state {
                    *server_id = server.id.clone();
                    server_name.clone_from(&server.name);
                    *delay_ms = delay;
                    let s = state.clone();
                    drop(state);
                    let _ = app.emit(STATE_EVENT, s);
                }
                Ok(delay)
            }
            Err(e) => {
                let _ = core.call("core.start", json!({ "config": live.config }), Duration::from_secs(15)).await;
                Err(e)
            }
        }
    }

    /// While connected: the running connection's real delay every half
    /// minute (after twenty seconds to settle); when it turns bad by the
    /// failover threshold, the servers are retested and the connection
    /// moves to the best one (the Android app's `FailoverMonitor`).
    fn monitor(self: Arc<Self>, app: AppHandle, core: Arc<CoreProcess>, since: u64) {
        tauri::async_runtime::spawn(async move {
            let mut threshold = geek_config::FailoverThreshold::Off;
            let mut policy = geek_config::FailoverPolicy::new(threshold);
            tokio::time::sleep(Duration::from_secs(20)).await;
            loop {
                let still = matches!(&*self.state.lock().await, TunnelState::On { .. }) && self.since().await == Some(since);
                if !still || !core.is_alive() {
                    return;
                }
                let wanted = {
                    let s = app.state::<crate::servers::ServersState>();
                    let store = s.store.lock().await;
                    if store.data.auto_select { store.data.failover } else { geek_config::FailoverThreshold::Off }
                };
                if wanted != threshold {
                    threshold = wanted;
                    policy = geek_config::FailoverPolicy::new(threshold);
                }
                if threshold != geek_config::FailoverThreshold::Off {
                    let ms = match core.call("core.delay", json!({ "url": probe_url() }), Duration::from_secs(15)).await {
                        Ok(v) => v["ms"].as_i64().unwrap_or(-1),
                        Err(_) => -1,
                    };
                    if policy.on_check(ms, now_ms() as i64) {
                        crate::servers::failover(&app).await;
                        policy.on_failover(now_ms() as i64);
                    }
                }
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }

    async fn since(&self) -> Option<u64> {
        match &*self.state.lock().await {
            TunnelState::On { since_ms, .. } => Some(*since_ms),
            _ => None,
        }
    }

    /// Quitting the app: leave the machine as it was (a strict kill switch
    /// excepted: holding is its whole point).
    pub async fn shutdown(&self) {
        self.usage.flush();
        self.release_system_proxy().await;
        self.stop_tun(true).await;
        if let Some(core) = self.core.lock().await.take() {
            core.kill().await;
        }
    }
}

const HELPER_OUTDATED: &str = "سرویس GeekVPN با این نسخه‌ی برنامه جور نیست. از تنظیمات «نصب سرویس» را دوباره بزن.";

pub fn helper_message(e: &IpcError) -> String {
    match e {
        IpcError::NotRunning(_) => "برای حالت TUN، سرویس GeekVPN باید نصب و روشن باشد. از تنظیمات «نصب سرویس» را بزن.".into(),
        IpcError::Closed | IpcError::Timeout => "سرویس GeekVPN جواب نداد. اگر تکرار شد، از تنظیمات دوباره نصبش کن.".into(),
        IpcError::Protocol(_) => HELPER_OUTDATED.into(),
        IpcError::Helper(h) => match h.code {
            ErrorCode::BadRequest => HELPER_OUTDATED.into(),
            ErrorCode::NoEngine => "موتور TUN کنار سرویس GeekVPN پیدا نشد. سرویس را دوباره نصب کن.".into(),
            ErrorCode::EngineFailed => format!("حالت TUN راه نیفتاد. {}", h.detail.lines().last().unwrap_or("")),
            ErrorCode::Firewall => format!("Kill Switch فعال نشد، پس اتصال برقرار نشد. {}", h.detail),
            ErrorCode::Internal => format!("سرویس GeekVPN خطا داد: {}", h.detail),
        },
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default()
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0)).and_then(|l| l.local_addr()).map(|a| a.port()).unwrap_or(10810)
}

/// A random credential for the core's locked SOCKS inbound.
fn token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
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
