//! «به‌روزرسانی برنامه»: Tauri's updater over a signed manifest.
//!
//! The manifest comes from the backend (`/api/app/desktop/update/...`, which
//! points the downloads at the mirror Iran can reach) and, failing that, from
//! the GitHub release itself. Neither is trusted for the content: every
//! package carries a minisign signature checked against the public key built
//! into the app, and the signed version must match the announced one, so a
//! mirror can neither swap a file nor push an old one back.
//!
//! The VPN stays up while the package downloads. Only for the install itself
//! is the connection taken down (the system proxy restored, the TUN stopped),
//! and if it was up, the new version connects again when it starts.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;
use url::Url;

use crate::servers::ServersState;
use crate::tunnel::TunnelState;

pub const AVAILABLE_EVENT: &str = "update://available";
pub const PROGRESS_EVENT: &str = "update://progress";
const FIRST_CHECK: Duration = Duration::from_secs(20);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 3600);
/// Left in the data folder by an install that interrupted a connection.
pub const RESUME_FILE: &str = "resume-after-update";

pub fn pubkey() -> Option<&'static str> {
    Some(env!("GEEK_UPDATER_PUBKEY")).filter(|k| !k.is_empty())
}

fn endpoints() -> Vec<Url> {
    env!("GEEK_UPDATE_ENDPOINTS").split_whitespace().filter_map(|u| Url::parse(u).ok()).collect()
}

pub struct UpdateState {
    found: Mutex<Option<Update>>,
    installing: AtomicBool,
    /// The version the customer was already told about.
    announced: std::sync::Mutex<Option<String>>,
    resume_file: PathBuf,
}

impl UpdateState {
    pub fn new(data_dir: &std::path::Path) -> Self {
        Self {
            found: Mutex::new(None),
            installing: AtomicBool::new(false),
            announced: std::sync::Mutex::new(None),
            resume_file: data_dir.join(RESUME_FILE),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateView {
    /// False in a build without the signing key: nothing can be verified.
    configured: bool,
    current: String,
    available: Option<Available>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Available {
    version: String,
    notes: String,
    date: Option<String>,
    /// The backend says this version is too old to keep using.
    required: bool,
}

fn describe(u: &Update) -> Available {
    Available {
        version: u.version.clone(),
        notes: u.body.clone().unwrap_or_default(),
        date: u.date.map(|d| d.to_string()),
        required: u.raw_json["required"].as_bool().unwrap_or(false),
    }
}

async fn check(app: &AppHandle) -> Result<Option<Update>, String> {
    if pubkey().is_none() {
        return Ok(None);
    }
    let updater = app
        .updater_builder()
        .endpoints(endpoints())
        .map_err(|e| e.to_string())?
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    updater.check().await.map_err(|e| e.to_string())
}

fn current(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub async fn update_check(app: AppHandle, state: State<'_, UpdateState>) -> Result<UpdateView, String> {
    let found = check(&app).await.map_err(|e| {
        eprintln!("geekvpn: update check: {e}");
        "سرور به‌روزرسانی در دسترس نبود. کمی بعد دوباره امتحان کن.".to_string()
    })?;
    let available = found.as_ref().map(describe);
    *state.found.lock().await = found;
    Ok(UpdateView { configured: pubkey().is_some(), current: current(&app), available })
}

/// What the last check found, without asking again.
#[tauri::command]
pub async fn update_state(app: AppHandle, state: State<'_, UpdateState>) -> Result<UpdateView, String> {
    let available = state.found.lock().await.as_ref().map(describe);
    Ok(UpdateView { configured: pubkey().is_some(), current: current(&app), available })
}

#[tauri::command]
pub async fn update_install(app: AppHandle, state: State<'_, UpdateState>) -> Result<(), String> {
    let Some(update) = state.found.lock().await.clone() else {
        return Err("به‌روزرسانی‌ای پیدا نشده. دوباره بررسی کن.".into());
    };
    if state.installing.swap(true, Ordering::SeqCst) {
        return Err("به‌روزرسانی در حال انجام است.".into());
    }
    let result = install(&app, &state, &update).await;
    state.installing.store(false, Ordering::SeqCst);
    result
}

async fn install(app: &AppHandle, state: &UpdateState, update: &Update) -> Result<(), String> {
    let mut done: u64 = 0;
    let progress = app.clone();
    let bytes = update
        .download(
            move |chunk, total| {
                done += chunk as u64;
                let _ = progress.emit(PROGRESS_EVENT, json!({ "downloaded": done, "total": total }));
            },
            || {},
        )
        .await
        .map_err(|e| {
            eprintln!("geekvpn: update download: {e}");
            // A signature that does not verify fails here too.
            "دانلود یا بررسی امضای به‌روزرسانی ناموفق بود. دوباره امتحان کن.".to_string()
        })?;

    // The installer replaces the running program: the network goes back to
    // the customer's own settings first, and a live connection is resumed by
    // the new version.
    let servers = app.state::<ServersState>();
    let was_on = matches!(*servers.tunnel.state.lock().await, TunnelState::On { .. });
    if was_on {
        let _ = std::fs::write(&state.resume_file, b"1");
    }
    servers.tunnel.shutdown().await;
    if let Err(e) = update.install(bytes) {
        eprintln!("geekvpn: update install: {e}");
        let _ = std::fs::remove_file(&state.resume_file);
        return Err("نصب به‌روزرسانی انجام نشد. برنامه همان نسخه‌ی قبلی ماند.".into());
    }
    // Windows has already left for the installer; macOS and Linux (an
    // AppImage replaced, or a .deb or .rpm through pkexec) start the new
    // version here.
    app.restart();
}

/// Whether the last run stopped a connection to update; cleared on read.
pub fn take_resume(data_dir: &std::path::Path) -> bool {
    std::fs::remove_file(data_dir.join(RESUME_FILE)).is_ok()
}

/// Shortly after start and then every six hours: a new version shows on the
/// account page, and once per version as a notification.
pub fn watch(app: AppHandle) {
    if pubkey().is_none() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if let Ok(Some(update)) = check(&app).await {
                let view = describe(&update);
                let state = app.state::<UpdateState>();
                *state.found.lock().await = Some(update);
                let fresh = {
                    let mut announced = state.announced.lock().unwrap();
                    let fresh = announced.as_deref() != Some(view.version.as_str());
                    *announced = Some(view.version.clone());
                    fresh
                };
                let _ = app.emit(AVAILABLE_EVENT, &view);
                if fresh {
                    crate::desktop::notify(
                        &app,
                        "نسخه‌ی جدید GeekVPN",
                        &format!("نسخه‌ی {} آماده است. از «حساب ← به‌روزرسانی برنامه» نصب کن.", view.version),
                    );
                }
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn endpoints_are_the_backend_then_github() {
        let e = super::endpoints();
        assert!(!e.is_empty());
        assert!(e[0].as_str().contains("api/app/desktop/update/%7B%7Btarget%7D%7D") || e[0].as_str().contains("{{target}}"), "{}", e[0]);
    }
}
