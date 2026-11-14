//! «تست سرعت» and «لاگ هسته».
//!
//! The speed test is the Android app's: Cloudflare's speed endpoints, five
//! pings (the median counts), then download and upload, each stopped at ten
//! seconds or its byte budget. Connected in proxy mode it goes through the
//! local HTTP proxy; in TUN mode the app's own traffic is in the tunnel
//! anyway; disconnected it measures the network itself.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

use crate::servers::ServersState;
use crate::tunnel::TunnelState;

pub const PROGRESS: &str = "speed://progress";
const BASE: &str = "https://speed.cloudflare.com";
const PINGS: usize = 5;
const DOWNLOAD_BYTES: u64 = 50_000_000;
const UPLOAD_BYTES: u64 = 20_000_000;
const PHASE: Duration = Duration::from_secs(10);
const REPORT: Duration = Duration::from_millis(250);

#[derive(Default)]
pub struct SpeedRun {
    running: AtomicBool,
    cancel: AtomicBool,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpeedResult {
    ping_ms: Option<u64>,
    jitter_ms: Option<u64>,
    download_mbps: Option<f64>,
    upload_mbps: Option<f64>,
    /// Through the tunnel, or the network itself.
    through_vpn: bool,
}

fn mbps(bytes: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if bytes == 0 || secs <= 0.0 {
        0.0
    } else {
        bytes as f64 * 8.0 / secs / 1_000_000.0
    }
}

/// The middle value, so one slow handshake does not set the number.
fn median(mut v: Vec<u64>) -> Option<u64> {
    v.sort_unstable();
    v.get(v.len() / 2).copied()
}

#[tauri::command]
pub async fn speed_test(app: AppHandle, state: State<'_, ServersState>, run: State<'_, SpeedRun>) -> Result<SpeedResult, String> {
    if run.running.swap(true, Ordering::SeqCst) {
        return Err("تست سرعت در حال اجراست.".into());
    }
    run.cancel.store(false, Ordering::SeqCst);
    let proxy = match &*state.tunnel.state.lock().await {
        TunnelState::On { http_port, .. } if *http_port > 0 => Some(*http_port),
        _ => None,
    };
    let on = matches!(&*state.tunnel.state.lock().await, TunnelState::On { .. });
    let result = measure(&app, &run, proxy).await;
    run.running.store(false, Ordering::SeqCst);
    result.map(|mut r| {
        r.through_vpn = on;
        r
    })
}

#[tauri::command]
pub fn speed_cancel(run: State<'_, SpeedRun>) {
    run.cancel.store(true, Ordering::SeqCst);
}

async fn measure(app: &AppHandle, run: &SpeedRun, proxy: Option<u16>) -> Result<SpeedResult, String> {
    let mut builder = reqwest::Client::builder().connect_timeout(Duration::from_secs(8)).timeout(Duration::from_secs(30));
    builder = match proxy {
        Some(port) => builder.proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?),
        None => builder.no_proxy(),
    };
    let client = builder.build().map_err(|e| e.to_string())?;
    let cancelled = || run.cancel.load(Ordering::SeqCst);
    let emit = |phase: &str, mbps: f64| {
        let _ = app.emit(PROGRESS, json!({ "phase": phase, "mbps": mbps }));
    };

    // Ping: time to first byte of an empty download.
    let mut pings = vec![];
    for _ in 0..PINGS {
        if cancelled() {
            break;
        }
        let start = Instant::now();
        if let Ok(r) = client.get(format!("{BASE}/__down?bytes=0")).send().await {
            if r.status().is_success() {
                let _ = r.bytes().await;
                pings.push(start.elapsed().as_millis() as u64);
            }
        }
    }
    let ping = median(pings.clone());
    if ping.is_none() {
        return Err("به سرور تست سرعت (Cloudflare) نرسید.".into());
    }
    let jitter = (pings.len() > 1).then(|| {
        let diffs: Vec<u64> = pings.windows(2).map(|w| w[0].abs_diff(w[1])).collect();
        diffs.iter().sum::<u64>() / diffs.len() as u64
    });
    emit("ping", 0.0);

    // Download, time-boxed.
    let mut down = None;
    if !cancelled() {
        let start = Instant::now();
        let mut bytes = 0u64;
        if let Ok(mut r) = client.get(format!("{BASE}/__down?bytes={DOWNLOAD_BYTES}")).timeout(PHASE + Duration::from_secs(5)).send().await {
            let mut last = start;
            while let Ok(Some(chunk)) = r.chunk().await {
                bytes += chunk.len() as u64;
                if last.elapsed() > REPORT {
                    emit("download", mbps(bytes, start.elapsed()));
                    last = Instant::now();
                }
                if start.elapsed() > PHASE || cancelled() {
                    break;
                }
            }
        }
        down = (bytes > 0).then(|| mbps(bytes, start.elapsed()));
        emit("download", down.unwrap_or(0.0));
    }

    // Upload: a body that ends when the time box or the budget does.
    let mut up = None;
    if !cancelled() {
        let sent = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let start = Instant::now();
        let chunk = bytes::Bytes::from(vec![0u8; 64 * 1024]);
        let counter = sent.clone();
        let stream = futures_util::stream::unfold((), move |()| {
            let (chunk, counter) = (chunk.clone(), counter.clone());
            async move {
                let so_far = counter.load(Ordering::SeqCst);
                if so_far >= UPLOAD_BYTES || start.elapsed() > PHASE {
                    return None;
                }
                counter.fetch_add(chunk.len() as u64, Ordering::SeqCst);
                Some((Ok::<_, std::io::Error>(chunk), ()))
            }
        });
        let progress = {
            let (sent, app) = (sent.clone(), app.clone());
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(REPORT).await;
                    let _ = app.emit(PROGRESS, json!({ "phase": "upload", "mbps": mbps(sent.load(Ordering::SeqCst), start.elapsed()) }));
                }
            })
        };
        let r = client
            .post(format!("{BASE}/__up"))
            .header("content-type", "application/octet-stream")
            .body(reqwest::Body::wrap_stream(stream))
            .timeout(PHASE + Duration::from_secs(10))
            .send()
            .await;
        progress.abort();
        let total = sent.load(Ordering::SeqCst);
        if r.is_ok_and(|r| r.status().is_success()) && total > 0 {
            up = Some(mbps(total, start.elapsed()));
        }
        emit("upload", up.unwrap_or(0.0));
    }
    Ok(SpeedResult { ping_ms: ping, jitter_ms: jitter, download_mbps: down, upload_mbps: up, through_vpn: false })
}

/// «لاگ هسته»: the last lines geekcore wrote (Xray's log), for problem
/// reports and for seeing why a server does not connect.
#[tauri::command]
pub async fn core_log(state: State<'_, ServersState>) -> Result<String, String> {
    Ok(state.tunnel.core().await?.log_tail())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    #[test]
    fn speed_math() {
        assert_eq!(super::mbps(1_250_000, Duration::from_secs(1)), 10.0);
        assert_eq!(super::mbps(0, Duration::from_secs(1)), 0.0);
        assert_eq!(super::median(vec![90, 30, 400, 50, 60]), Some(60));
        assert_eq!(super::median(vec![]), None);
    }
}

/// «IP خروجی»: the address and country the internet sees, asked through
/// the running connection (Cloudflare's trace, plain `key=value` lines).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExitIp {
    pub ip: String,
    /// ISO 3166 alpha-2, as Cloudflare reports it ("DE").
    pub country: String,
    /// The Cloudflare data centre that answered ("FRA").
    pub colo: String,
}

/// Reads Cloudflare's `/cdn-cgi/trace`; `None` without an address.
pub fn parse_trace(body: &str) -> Option<ExitIp> {
    let get = |k: &str| body.lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('='))).map(|v| v.trim().to_string());
    let ip = get("ip").filter(|ip| ip.parse::<std::net::IpAddr>().is_ok())?;
    let country = get("loc").filter(|c| c.len() == 2 && c.bytes().all(|b| b.is_ascii_uppercase())).unwrap_or_default();
    let colo = get("colo").filter(|c| c.len() <= 4 && c.bytes().all(|b| b.is_ascii_alphanumeric())).unwrap_or_default();
    Some(ExitIp { ip, country, colo })
}

#[tauri::command]
pub async fn exit_ip(state: State<'_, ServersState>) -> Result<ExitIp, String> {
    let proxy = match &*state.tunnel.state.lock().await {
        TunnelState::On { http_port, .. } if *http_port > 0 => Some(*http_port),
        TunnelState::On { .. } => None,
        _ => return Err("وصل نیستی.".into()),
    };
    let mut builder = reqwest::Client::builder().connect_timeout(Duration::from_secs(8)).timeout(Duration::from_secs(12));
    builder = match proxy {
        Some(port) => builder.proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?),
        None => builder.no_proxy(),
    };
    let client = builder.build().map_err(|e| e.to_string())?;
    let body = client
        .get("https://www.cloudflare.com/cdn-cgi/trace")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|_| "IP خروجی پیدا نشد؛ اتصال جواب نداد.".to_string())?
        .text()
        .await
        .map_err(|_| "IP خروجی پیدا نشد.".to_string())?;
    parse_trace(&body).ok_or_else(|| "IP خروجی پیدا نشد.".into())
}

#[cfg(test)]
mod trace_tests {
    #[test]
    fn reads_the_trace() {
        let t = super::parse_trace("fl=1f\nh=www.cloudflare.com\nip=203.0.113.7\nts=1\nvisit_scheme=https\nloc=DE\ncolo=FRA\n").unwrap();
        assert_eq!((t.ip.as_str(), t.country.as_str(), t.colo.as_str()), ("203.0.113.7", "DE", "FRA"));
        assert!(super::parse_trace("loc=DE\n").is_none());
        assert_eq!(super::parse_trace("ip=2001:db8::1\nloc=<script>\n").unwrap().country, "");
    }
}
