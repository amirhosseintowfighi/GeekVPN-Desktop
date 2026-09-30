//! Sign-in and the session, as commands the UI may call.
//!
//! Tokens never cross into the webview: the UI sees who is signed in and
//! what happened, never a token. Every backend call goes through `Session`.

use std::sync::Arc;
use std::time::Duration;

use geek_api::{telegram_app_link, wait_for_approval, ApiClient, ApiError, AppUser, DeviceInfo, LinkOutcome, Session, SessionError};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

pub const AUTH_CHANGED: &str = "auth://changed";
pub const LOGIN_STATUS: &str = "login://status";

pub struct AuthState {
    pub session: Arc<Session>,
    pub device: DeviceInfo,
    login: Mutex<Option<PendingLogin>>,
}

struct PendingLogin {
    deep_link: String,
    cancel: CancellationToken,
}

impl AuthState {
    pub fn new(session: Arc<Session>, device: DeviceInfo) -> Self {
        Self { session, device, login: Mutex::new(None) }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthView {
    user: Option<AppUser>,
    /// Why the session will not survive a restart, when the keychain refused it.
    store_error: Option<String>,
}

async fn view(session: &Session) -> AuthView {
    AuthView { user: session.user().await, store_error: session.store_error() }
}

pub async fn emit_changed(app: &AppHandle, session: &Session) {
    let _ = app.emit(AUTH_CHANGED, view(session).await);
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LoginStatus {
    /// approved | denied | expired | error
    status: &'static str,
    message: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginView {
    deep_link: String,
    /// The same link as a QR code, for approving from Telegram on a phone.
    qr_svg: String,
    expires_in: u64,
}

#[tauri::command]
pub async fn auth_state(state: State<'_, AuthState>) -> Result<AuthView, String> {
    Ok(view(&state.session).await)
}

/// Asks the backend for a sign-in link, opens Telegram on it and starts
/// waiting for the customer's decision in the background.
#[tauri::command]
pub async fn login_telegram_start(app: AppHandle, state: State<'_, AuthState>) -> Result<LoginView, String> {
    let start = state.session.api().link_start(&state.device).await.map_err(|e| e.user_message())?;
    let qr_svg = qr_svg(&start.deep_link)?;
    let cancel = CancellationToken::new();
    if let Some(previous) = state.login.lock().await.replace(PendingLogin {
        deep_link: start.deep_link.clone(),
        cancel: cancel.clone(),
    }) {
        previous.cancel.cancel();
    }
    open_telegram(&app, &start.deep_link);

    let session = state.session.clone();
    let api = session.api().clone();
    let poll_token = start.poll_token.clone();
    let deadline = Instant::now() + Duration::from_secs(start.expires_in);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let status = match wait_for_approval(&api, &poll_token, deadline, &cancel).await {
            Ok(LinkOutcome::Approved(signed_in)) => {
                session.sign_in(*signed_in).await;
                emit_changed(&handle, &session).await;
                LoginStatus { status: "approved", message: None }
            }
            Ok(LinkOutcome::Denied) => LoginStatus {
                status: "denied",
                message: Some("درخواست ورود در ربات لغو شد.".into()),
            },
            Ok(LinkOutcome::Expired) => LoginStatus {
                status: "expired",
                message: Some("مهلت پنج‌دقیقه‌ای ورود تمام شد. دوباره «ورود با تلگرام» را بزن.".into()),
            },
            Ok(LinkOutcome::Cancelled) => return,
            Err(e) => LoginStatus { status: "error", message: Some(e.user_message()) },
        };
        let _ = handle.emit(LOGIN_STATUS, status);
    });

    Ok(LoginView { deep_link: start.deep_link, qr_svg, expires_in: start.expires_in })
}

/// Opens Telegram again on the pending link. Only that link: the UI cannot
/// make the app open an arbitrary address through this.
#[tauri::command]
pub async fn login_open_telegram(app: AppHandle, state: State<'_, AuthState>) -> Result<(), String> {
    let link = state.login.lock().await.as_ref().map(|p| p.deep_link.clone());
    match link {
        Some(link) => {
            open_telegram(&app, &link);
            Ok(())
        }
        None => Err("درخواست ورودی در جریان نیست.".into()),
    }
}

#[tauri::command]
pub async fn login_cancel(state: State<'_, AuthState>) -> Result<(), String> {
    if let Some(pending) = state.login.lock().await.take() {
        pending.cancel.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn login_password(
    app: AppHandle,
    state: State<'_, AuthState>,
    username: String,
    password: String,
) -> Result<AuthView, String> {
    let username = username.trim();
    if username.is_empty() || password.is_empty() {
        return Err("نام کاربری و رمز را وارد کن.".into());
    }
    let signed_in = state
        .session
        .api()
        .password_login(username, &password, &state.device)
        .await
        .map_err(|e| e.user_message())?;
    state.session.sign_in(signed_in).await;
    emit_changed(&app, &state.session).await;
    Ok(view(&state.session).await)
}

#[tauri::command]
pub async fn logout(app: AppHandle, state: State<'_, AuthState>) -> Result<(), String> {
    state.session.sign_out().await;
    emit_changed(&app, &state.session).await;
    Ok(())
}

/// One backend call with the session's token, its error in Persian. A
/// session the server has ended also tells the UI, which goes back to the
/// sign-in screen.
pub async fn call<T, F, Fut>(app: &AppHandle, state: &AuthState, f: F) -> Result<T, String>
where
    F: Fn(ApiClient, String) -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    let api = state.session.api().clone();
    match state.session.authorized(|t| f(api.clone(), t)).await {
        Ok(v) => Ok(v),
        Err(e) => {
            if matches!(e, SessionError::Api(ApiError::Unauthorized)) {
                emit_changed(app, &state.session).await;
            }
            Err(e.user_message())
        }
    }
}

/// At start-up: re-read the profile, which also refreshes an old token. A
/// session the server no longer accepts ends here, not at the first click.
pub async fn revalidate(app: AppHandle, session: Arc<Session>) {
    if session.user().await.is_none() {
        return;
    }
    match session.refresh_user().await {
        Ok(_) | Err(SessionError::Api(geek_api::ApiError::Unauthorized)) => emit_changed(&app, &session).await,
        // Offline at start-up: keep the session; the next call retries.
        Err(_) => {}
    }
}

/// Telegram Desktop through `tg://` when it is installed; otherwise the
/// browser, whose t.me page offers to open Telegram (or Telegram Web).
///
/// The opener hands the URL to the OS and returns before the OS knows
/// whether anything handles it, so "no Telegram installed" never comes back
/// as an error. Hence the explicit check for a `tg:` handler first.
pub fn open_telegram(app: &AppHandle, deep_link: &str) {
    let opener = app.opener();
    let opened = match telegram_app_link(deep_link) {
        Some(tg) if has_tg_handler() => opener.open_url(tg, None::<&str>).is_ok(),
        _ => false,
    };
    if !opened {
        let _ = opener.open_url(deep_link, None::<&str>);
    }
}

#[cfg(target_os = "linux")]
fn has_tg_handler() -> bool {
    std::process::Command::new("xdg-mime")
        .args(["query", "default", "x-scheme-handler/tg"])
        .output()
        .is_ok_and(|o| o.status.success() && !o.stdout.trim_ascii().is_empty())
}

#[cfg(target_os = "windows")]
fn has_tg_handler() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("reg")
        .args(["query", r"HKCR\tg", "/v", "URL Protocol"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(target_os = "macos")]
fn has_tg_handler() -> bool {
    // Launch Services knows the handler; Telegram and Telegram Lite both
    // register tg:. Checking the bundle ids avoids spawning a URL open.
    ["ru.keepcoder.Telegram", "org.telegram.desktop"].iter().any(|id| {
        std::process::Command::new("mdfind")
            .arg(format!("kMDItemCFBundleIdentifier == '{id}'"))
            .output()
            .is_ok_and(|o| o.status.success() && !o.stdout.trim_ascii().is_empty())
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn has_tg_handler() -> bool {
    false
}

pub fn qr_svg(link: &str) -> Result<String, String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::new(link.as_bytes()).map_err(|e| e.to_string())?;
    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(196, 196)
        .dark_color(svg::Color("#062845"))
        .light_color(svg::Color("#FFFFFF"))
        .build())
}
