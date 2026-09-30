mod auth;
mod servers;
mod store;
mod system;
mod tunnel;

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

/// geekcore sits next to the app's executable: Tauri's `externalBin` puts
/// it there in every bundle, and `tauri dev` copies it there too.
fn core_binary() -> std::io::Result<std::path::PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().ok_or_else(|| std::io::Error::other("no exe dir"))?;
    Ok(dir.join(if cfg!(windows) { "geekcore.exe" } else { "geekcore" }))
}

fn geo_dir(app: &tauri::App) -> tauri::Result<std::path::PathBuf> {
    Ok(app.path().resource_dir()?.join("resources").join("geo"))
}

/// A logout, shutdown or `kill` ends the process without Tauri's Exit event;
/// the system proxy still has to go back before we are gone.
#[cfg(unix)]
fn restore_on_signals(app: tauri::AppHandle, tunnel: Arc<tunnel::Tunnel>) {
    use tokio::signal::unix::{signal, SignalKind};
    tauri::async_runtime::spawn(async move {
        let (Ok(mut term), Ok(mut int), Ok(mut hup)) =
            (signal(SignalKind::terminate()), signal(SignalKind::interrupt()), signal(SignalKind::hangup()))
        else {
            return;
        };
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv() => {}
            _ = hup.recv() => {}
        }
        tunnel.shutdown().await;
        app.exit(0);
    });
}

/// Windows ends sessions through window messages, which Tauri turns into the
/// Exit event handled in `run`; a hard kill is covered by `Tunnel::recover`.
#[cfg(not(unix))]
fn restore_on_signals(_app: tauri::AppHandle, _tunnel: Arc<tunnel::Tunnel>) {}

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
        .plugin(tauri_plugin_dialog::init())
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

            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let tunnel = Arc::new(tunnel::Tunnel::new(core_binary()?, geo_dir(app)?, data.join("proxy-snapshot.json")));
            // A previous run that died connected left the system proxy on.
            tunnel.recover();
            app.manage(servers::ServersState {
                store: tokio::sync::Mutex::new(store::Store::load(data.join("servers.json"))),
                tunnel: tunnel.clone(),
            });
            restore_on_signals(app.handle().clone(), tunnel);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth::auth_state,
            auth::login_telegram_start,
            auth::login_open_telegram,
            auth::login_cancel,
            auth::login_password,
            auth::logout,
            servers::servers_state,
            servers::servers_refresh,
            servers::servers_add,
            servers::servers_remove_source,
            servers::servers_set,
            servers::servers_test,
            servers::tunnel_state,
            servers::tunnel_connect,
            servers::tunnel_disconnect,
            system::helper_status,
            system::helper_install,
            system::connections_list,
            system::connections_close,
            system::programs_running,
        ])
        .build(tauri::generate_context!())
        .expect("GeekVPN failed to start")
        .run(|app, event| {
            // Whatever the way out, the system proxy goes back to the user's.
            if let tauri::RunEvent::Exit = event {
                if let Some(s) = app.try_state::<servers::ServersState>() {
                    tauri::async_runtime::block_on(s.tunnel.shutdown());
                }
            }
        });
}
