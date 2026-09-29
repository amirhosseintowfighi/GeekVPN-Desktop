mod auth;

use std::sync::Arc;

use geek_api::{ApiClient, Session, TokenStore};
use geek_secrets::KeychainStore;
use tauri::Manager;
use url::Url;

/// A store that keeps nothing, for the rare machine with no keychain at all:
/// the session then lasts until the app quits, and `store_error` says why.
struct NoStore(String);

impl TokenStore for NoStore {
    fn load(&self) -> Result<Option<geek_api::SignedIn>, String> {
        Err(self.0.clone())
    }
    fn save(&self, _: &geek_api::SignedIn) -> Result<(), String> {
        Err(self.0.clone())
    }
    fn clear(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Builds and runs the app. Everything the UI may call is declared in
/// `capabilities/` and the `invoke_handler` below; the webview gets nothing else.
pub fn run() {
    tauri::Builder::default()
        // A second launch (autostart plus a click on the shortcut, or a
        // `geekvpn://` link later) must bring the running window forward, not
        // start a second core that fights the first over the TUN device.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let version = app.package_info().version.to_string();
            let device = geek_secrets::device_info(&version, || {
                KeychainStore::fallback_device_id(|| uuid::Uuid::new_v4().to_string())
            });
            let base = Url::parse(env!("GEEK_API_BASE")).expect("build.rs validated GEEK_API_BASE");
            let api = ApiClient::new(base, &format!("GeekVPN/{version} ({})", device.platform))?;
            let store: Arc<dyn TokenStore> = match KeychainStore::new() {
                Ok(s) => Arc::new(s),
                Err(e) => Arc::new(NoStore(e)),
            };
            let session = Arc::new(Session::restore(api, store));
            tauri::async_runtime::spawn(auth::revalidate(app.handle().clone(), session.clone()));
            app.manage(auth::AuthState::new(session, device));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth::auth_state,
            auth::login_telegram_start,
            auth::login_open_telegram,
            auth::login_cancel,
            auth::login_password,
            auth::logout,
        ])
        .run(tauri::generate_context!())
        .expect("GeekVPN failed to start");
}
