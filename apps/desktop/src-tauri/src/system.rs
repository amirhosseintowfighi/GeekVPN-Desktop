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
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "state")]
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
    // تشخیص ادمین بودن بدون winapi اضافه: یک SCM بازِ آزمایشی.
    // اگر ادمین نیستیم، اصلا تلاشِ مستقیم نکن — مستقیم برو سراغ UAC تا پیامِ
    // winapiِ خام به کاربر نرسد.
    let already_admin = {
        use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
        ServiceManager::local_computer(
            None::<&str>,
            ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
        )
        .is_ok()
    };
    if already_admin {
        let out = std::process::Command::new(helper)
            .arg("install")
            .creation_flags(0x0800_0000)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            return Ok(true);
        }
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let raw = if !stderr.is_empty() { stderr } else { stdout };
        let detail = translate_helper_error(&raw);
        return Err(detail);
    }

    let log = std::env::temp_dir().join("geekvpn-helper-install.log");
    let _ = std::fs::remove_file(&log);
    // PowerShell 5.1: -Verb RunAs cannot be combined with -RedirectStandard*
    // (→ "parameter set cannot be resolved…"). To avoid any quoting hell,
    // write a tiny .cmd that does the redirection and elevate that instead.
    let bat = std::env::temp_dir().join("geekvpn-helper-install.cmd");
    let bat_content = format!("\"{}\" install > \"{}\" 2>&1\r\n", helper.display(), log.display());
    let _ = std::fs::write(&bat, bat_content);
    let log_str = log.to_string_lossy().replace('\'', "''");
    let bat_str = bat.to_string_lossy().replace('\'', "''");
    // Only call Write-Host EXIT:$c when we actually have a process handle;
    // PowerShell's catch path may not have $c.
    let command = format!(
        "$log='{log_str}'; $bat='{bat_str}'; try {{ $p = Start-Process -FilePath 'cmd.exe' -ArgumentList \"/c `\"$bat`\"\" -Verb RunAs -Wait -PassThru -WindowStyle Hidden; if ($p) {{ $c=$p.ExitCode; if (Test-Path $log) {{ Get-Content $log | Write-Host }}; Write-Host \"EXIT:$c\"; exit $c }} else {{ Write-Host \"NO_HANDLE\"; exit 1 }} }} catch {{ $m=$_.Exception.Message; $m | Out-File -Append $log; Write-Host $m; Write-Host \"CATCH\"; exit 1 }}; if (Test-Path $log) {{ Get-Content $log | Write-Host }}"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &command])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(true);
    }
    let mut detail = String::new();
    if let Ok(t) = std::fs::read_to_string(&log) {
        if !t.trim().is_empty() {
            detail = t.trim().to_string();
        }
    }
    if detail.is_empty() {
        detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
    }
    if detail.is_empty() {
        detail = String::from_utf8_lossy(&out.stdout).trim().to_string();
    }
    if detail.to_lowercase().contains("operation was canceled") || detail.contains("1223") || detail.to_lowercase().contains("canceled") {
        return Err(
            "نصب ادامه پیدا نکرد — به نظر پنجره‌ی تاییدِ ویندوز بسته یا رد شد.\n\
             ویندوز هنگام «نصب دوباره» یک پنجره‌ی آبی/زرد می‌آورد با عنوان «آیا می‌خواهید به این برنامه اجازه دهید…» — \
             باید Yes را بزنی. دوباره «نصب دوباره» را بزن و وقتی آن پنجره آمد Yes را بزن."
                .into(),
        );
    }
    if detail.is_empty() || detail.starts_with("EXIT:") {
        // ریشه‌ی باگِ بیلدِ قبلی: log خالی می‌ماند و فقط EXIT برمی‌گشت.
        // علت واقعی معمولاً در bat/log است، نه در stderrِ powershell.
        let bat_hint = if bat.exists() { format!(" (bat: {})", bat.display()) } else { "".to_string() };
        return Err(format!(
            "نصبِ سرویس بدونِ پیامِ خطایِ واضح تمام شد.\n\
             برای اینکه دقیق بفهمم مشکل کجاست، لطفاً این‌ها را برایم بفرست:\n\
             1) محتوای فایل: {}{bat_hint}\n\
             2) همین متنِ خطا را کامل کپی کن (کد: {})\n\
             3) نسخه‌ی برنامه (تنظیمات → درباره، یا نامِ فایلِ نصاب)\n\
             \n\
             توضیحِ UAC برای کاربرِ عادی: ویندوز برای نصبِ سرویس یک پنجره‌ی جداگانه می‌آورد \
             («آیا اجازه می‌دهید این برنامه تغییراتی در دستگاه ایجاد کند؟») — باید Yes بزنی. \
             اگر آن پنجره را اصلاً ندیدی، یک‌بار برنامه را ببند و دوباره باز کن، بعد «نصب دوباره» را بزن.\n\
             اگر Windows Defender چیزی را قرنطینه کرده باشد، معمولاً داخلِ log بالا نوشته می‌شود.",
            log.display(),
            if detail.is_empty() { "(خالی)" } else { &detail }
        ));
    }
    Err(translate_helper_error(&detail))
}

#[cfg(windows)]
fn translate_helper_error(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return "نصب سرویس بدون پیام خطا تمام شد.".into();
    }
    // PowerShell 5.1: ترکیب -Verb RunAs با -RedirectStandard* خطای parameter set می‌دهد.
    // اگر هنوز این خطا دیده شد (بیلد قدیمی نصب است)، مستقیم بگو بیلد را عوض کند.
    if t.contains("parameter set cannot be resolved") || t.contains("A positional parameter cannot be found") {
        return format!(
            "نصبِ UAC به‌خاطر باگِ بیلدِ قبلی انجام نشد (parameter set). \
              آخرین geekvpn-windows را از Actions همین برنچ (35bb240 به بعد) دوباره دانلود و نصب کن، \
              بعد «نصب دوباره» را بزن. (جزئیات: {t})"
        );
    }
    // پیام‌های فنیِ ویندوز/helper را به فارسیِ قابلِ اقدام ترجمه کن.
    if t.contains("wintun.dll is missing") || t.contains("msys-2.0.dll is missing") {
        // ریشه‌ی مشکلِ دسته‌ی دومِ کاربر — DLLها کنارِ helper نیستند چون بیلدِ قدیمی نصب است.
        let dll = if t.contains("wintun.dll") { "wintun.dll" } else { "msys-2.0.dll" };
        return format!(
            "فایل {dll} کنارِ سرویس پیدا نشد. این بیلدی که نصب کرده‌ای قدیمی‌ست — \
             آخرین geekvpn-windows را از همین برنچ (f3eb9ff به بعد) دانلود و نصب کن، \
             بعد دوباره «نصب دوباره» را بزن. (جزئیات: {t})"
        );
    }
    if t.to_lowercase().contains("service manager") {
        return format!(
            "برای نصبِ سرویس دسترسیِ مدیر لازم است. برنامه را معمولی باز کن و «نصب دوباره» را بزن و در پنجره‌ی ویندوز Yes را بزن \
             — یا یک PowerShell را Run as administrator کن و بزن: geekvpn-helper install  (جزئیات: {t})"
        );
    }
    t.to_string()
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
    // "IP", not "Ip": Clash's own spelling.
    #[serde(rename = "destinationIP")]
    destination_ip: String,
    destination_port: String,
    process_path: String,
}

/// sing-box writes the path as "/usr/bin/curl (alice)", user appended.
fn strip_user(path: &str) -> &str {
    match path.rfind(" (") {
        Some(i) if path.ends_with(')') => &path[..i],
        _ => path,
    }
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
            let path = strip_user(&m.process_path).to_string();
            let process = path.rsplit(['/', '\\']).next().unwrap_or("").to_string();
            ConnectionView {
                id: c.id,
                host: if m.host.is_empty() { m.destination_ip } else { m.host },
                port: m.destination_port,
                network: m.network,
                process,
                process_path: path,
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

#[cfg(test)]
mod tests {
    #[test]
    fn clash_connection_fields() {
        let c: super::ClashConn = serde_json::from_value(serde_json::json!({
            "id": "1", "upload": 5, "download": 7, "start": "2026-09-30T06:00:00Z", "chains": ["proxy"],
            "metadata": { "network": "tcp", "host": "", "destinationIP": "45.90.28.30", "destinationPort": "80",
                          "processPath": "/usr/bin/curl (root)" }
        }))
        .unwrap();
        assert_eq!(c.metadata.destination_ip, "45.90.28.30");
        assert_eq!(super::strip_user(&c.metadata.process_path), "/usr/bin/curl");
        assert_eq!(super::strip_user(r"C:\Program Files (x86)\Steam\steam.exe"), r"C:\Program Files (x86)\Steam\steam.exe");
    }
}
