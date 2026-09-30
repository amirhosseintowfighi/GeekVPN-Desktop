//! The machine around the tunnel: geekvpn-helper (is it there, install
//! it), the live connection list in TUN mode, and the programs the user can
//! route one by one.

use std::path::PathBuf;
use std::time::Duration;

use geek_ipc::PROTOCOL;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::servers::ServersState;
use crate::tunnel::helper_message;

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum HelperStatus {
    /// Nothing answers on the helper's socket.
    Missing { reason: String },
    /// Answers, but speaks another protocol version: reinstall.
    Outdated { version: String },
    Ready { version: String, sing_box: String },
}

#[tauri::command]
pub async fn helper_status(state: State<'_, ServersState>) -> Result<HelperStatus, String> {
    Ok(match state.tunnel.helper().await {
        Ok((_, hello)) if hello.protocol == PROTOCOL => HelperStatus::Ready { version: hello.version, sing_box: hello.sing_box },
        Ok((_, hello)) => HelperStatus::Outdated { version: hello.version },
        Err(e) => HelperStatus::Missing { reason: helper_message(&e) },
    })
}

/// The helper binary shipped beside the app (Tauri's `externalBin`).
fn shipped_helper() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let p = exe.parent().ok_or("no app directory")?.join(if cfg!(windows) { "geekvpn-helper.exe" } else { "geekvpn-helper" });
    if p.exists() {
        Ok(p)
    } else {
        Err("فایل سرویس کنار برنامه نیست؛ برنامه را دوباره نصب کن.".into())
    }
}

/// «نصب سرویس»: `geekvpn-helper install`, with the system's own
/// administrator prompt (polkit, macOS's password sheet, Windows UAC).
#[tauri::command]
pub async fn helper_install(state: State<'_, ServersState>) -> Result<HelperStatus, String> {
    let helper = shipped_helper()?;
    let ok = tauri::async_runtime::spawn_blocking(move || elevated_install(&helper))
        .await
        .map_err(|e| e.to_string())??;
    if !ok {
        return Err("نصب سرویس انجام نشد (اجازه‌ی مدیر داده نشد یا نصب خطا داد).".into());
    }
    // The service starts right after install; give it a moment to listen.
    for _ in 0..40 {
        if let Ok(s @ HelperStatus::Ready { .. }) = helper_status(state.clone()).await {
            return Ok(s);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    helper_status(state).await
}

#[cfg(target_os = "linux")]
fn elevated_install(helper: &std::path::Path) -> Result<bool, String> {
    let status = std::process::Command::new("pkexec")
        .arg(helper)
        .arg("install")
        .status()
        .map_err(|_| "pkexec پیدا نشد. در ترمینال بزن: sudo <مسیر برنامه>/geekvpn-helper install".to_string())?;
    Ok(status.success())
}

#[cfg(target_os = "macos")]
fn elevated_install(helper: &std::path::Path) -> Result<bool, String> {
    // AppleScript string: backslashes and double quotes escaped; the shell
    // word single-quoted.
    let shell = format!("'{}' install", helper.to_string_lossy().replace('\'', r"'\''"));
    let script = format!("do shell script \"{}\" with administrator privileges", shell.replace('\\', "\\\\").replace('"', "\\\""));
    let status = std::process::Command::new("osascript").arg("-e").arg(script).status().map_err(|e| e.to_string())?;
    Ok(status.success())
}

#[cfg(windows)]
fn elevated_install(helper: &std::path::Path) -> Result<bool, String> {
    use std::os::windows::process::CommandExt;
    let path = helper.to_string_lossy().replace('\'', "''");
    let command = format!(
        "$p = Start-Process -FilePath '{path}' -ArgumentList 'install' -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode"
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .creation_flags(0x0800_0000)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(status.success())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn elevated_install(_: &std::path::Path) -> Result<bool, String> {
    Err("این سیستم‌عامل پشتیبانی نمی‌شود.".into())
}

// -- Connections (sing-box's Clash API, TUN mode) ----------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClashList {
    #[serde(default)]
    connections: Vec<ClashConn>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClashConn {
    id: String,
    metadata: ClashMeta,
    #[serde(default)]
    upload: u64,
    #[serde(default)]
    download: u64,
    #[serde(default)]
    start: String,
    #[serde(default)]
    chains: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ClashMeta {
    network: String,
    host: String,
    destination_ip: String,
    destination_port: String,
    process_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    id: String,
    /// The name when sniffed or resolved through sing-box, else the address.
    host: String,
    port: String,
    network: String,
    /// The program's file name, and its full path.
    process: String,
    process_path: String,
    upload: u64,
    download: u64,
    start: String,
    /// "proxy" (through the tunnel) or "direct".
    route: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionsView {
    /// False when there is nothing to list: not connected in TUN mode.
    available: bool,
    connections: Vec<ConnectionView>,
}

fn clash() -> reqwest::Client {
    // Straight to loopback: never through a proxy from the environment.
    reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(3)).build().unwrap_or_default()
}

#[tauri::command]
pub async fn connections_list(state: State<'_, ServersState>) -> Result<ConnectionsView, String> {
    let Some((info, _)) = state.tunnel.tun.lock().await.clone() else {
        return Ok(ConnectionsView { available: false, connections: vec![] });
    };
    let list: ClashList = clash()
        .get(format!("http://127.0.0.1:{}/connections", info.clash_port))
        .bearer_auth(&info.clash_secret)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|_| "فهرست اتصال‌ها از موتور TUN خوانده نشد.".to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let mut connections: Vec<ConnectionView> = list
        .connections
        .into_iter()
        .map(|c| {
            let m = c.metadata;
            let process = m.process_path.rsplit(['/', '\\']).next().unwrap_or("").to_string();
            ConnectionView {
                id: c.id,
                host: if m.host.is_empty() { m.destination_ip } else { m.host },
                port: m.destination_port,
                network: m.network,
                process,
                process_path: m.process_path,
                upload: c.upload,
                download: c.download,
                start: c.start,
                // Clash lists the chain last hop first; ours is one hop.
                route: c.chains.first().cloned().unwrap_or_default(),
            }
        })
        .collect();
    // Newest first, as the Android list.
    connections.sort_by(|a, b| b.start.cmp(&a.start));
    Ok(ConnectionsView { available: true, connections })
}

/// Closes one connection, or all of them without `id`.
#[tauri::command]
pub async fn connections_close(state: State<'_, ServersState>, id: Option<String>) -> Result<(), String> {
    let Some((info, _)) = state.tunnel.tun.lock().await.clone() else { return Ok(()) };
    let url = match id {
        Some(id) => format!("http://127.0.0.1:{}/connections/{id}", info.clash_port),
        None => format!("http://127.0.0.1:{}/connections", info.clash_port),
    };
    clash()
        .delete(url)
        .bearer_auth(&info.clash_secret)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map(|_| ())
        .map_err(|_| "اتصال بسته نشد.".to_string())
}

// -- Programs, for «تونل برنامه‌ای» ---------------------------------------------------

#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ProgramView {
    name: String,
    path: String,
}

/// Running programs with an executable on disk, one entry per executable.
#[tauri::command]
pub async fn programs_running() -> Result<Vec<ProgramView>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_exe(UpdateKind::Always));
        let own = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let mut out: Vec<ProgramView> = sys
            .processes()
            .values()
            .filter_map(|p| p.exe().map(|e| e.to_path_buf()))
            .filter(|e| own.as_deref().is_none_or(|d| !e.starts_with(d)))
            .map(|e| ProgramView {
                name: e.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                path: e.to_string_lossy().into_owned(),
            })
            .filter(|p| !p.name.is_empty())
            .collect();
        out.sort();
        out.dedup();
        out
    })
    .await
    .map_err(|e| e.to_string())
}
