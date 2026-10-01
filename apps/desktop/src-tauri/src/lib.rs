mod auth;
mod deeplink;
mod desktop;
mod report;
mod scan;
mod servers;
mod shop;
mod store;
mod support;
mod system;
mod tools;
mod tunnel;
mod update;
mod usage;

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
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| desktop::show_main(app)))
        // Size and place, not visibility: whether the window shows at launch
        // is autostart's (--minimized) and close-to-tray's decision.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(tauri_plugin_window_state::StateFlags::all() & !tauri_plugin_window_state::StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec![desktop::MINIMIZED])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        desktop::toggle_connection(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin({
            // The key the release workflow signs with, from the build; none
            // means updates stay off (`update::pubkey`).
            let mut updater = tauri_plugin_updater::Builder::new();
            if let Some(key) = update::pubkey() {
                updater = updater.pubkey(key);
            }
            updater.build()
        })
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
            app.manage(shop::ShopState::default());
            app.manage(auth::AuthState::new(session, device));

            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let tunnel = Arc::new(tunnel::Tunnel::new(core_binary()?, geo_dir(app)?, data.join("proxy-snapshot.json"), data.join("usage.json")));
            // A previous run that died connected left the system proxy on.
            tunnel.recover();
            let saved = store::Store::load(data.join("servers.json"));
            let settings = saved.data.clone();
            app.manage(servers::ServersState { store: tokio::sync::Mutex::new(saved), tunnel: tunnel.clone() });
            app.manage(scan::Ranges::new(app.path().resource_dir()?.join("resources").join("cfscan").join("ipv4.txt")));
            app.manage(tools::SpeedRun::default());
            app.manage(scan::ScanRun { running: tokio::sync::Mutex::new(None) });
            app.manage(support::SupportState::new(data.join("support-seen.json")));
            support::watch(app.handle().clone(), app.state::<auth::AuthState>().session.clone());
            app.manage(update::UpdateState::new(&data));
            update::watch(app.handle().clone());
            let resume = update::take_resume(&data);
            restore_on_signals(app.handle().clone(), tunnel);
            desktop::setup(app, &settings, resume)?;
            deeplink::setup(app);
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
            scan::scan_state,
            scan::scan_start,
            scan::scan_stop,
            scan::scan_use,
            scan::scan_set_download,
            tools::speed_test,
            tools::speed_cancel,
            tools::core_log,
            usage::usage_local,
            shop::shop_load,
            shop::shop_quote,
            shop::shop_coupon,
            shop::shop_checkout,
            shop::shop_open_gateway,
            shop::shop_pending,
            shop::shop_receipt,
            shop::shop_txid,
            shop::trial_claim,
            shop::wallet_load,
            shop::wallet_transactions,
            shop::wallet_topup,
            shop::referral_load,
            shop::usage_service,
            support::tickets_list,
            support::support_unread,
            support::ticket_thread,
            support::ticket_reply,
            support::ticket_open,
            support::report_preview,
            support::report_send,
            support::support_open_bot,
            update::update_check,
            update::update_state,
            update::update_install,
            desktop::desktop_autostart,
            desktop::desktop_open,
            desktop::desktop_quit,
        ])
        .on_window_event(|window, event| {
            // Closing the main window keeps GeekVPN beside the clock.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && desktop::closes_to_tray(window.app_handle()) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("GeekVPN failed to start")
        .run(|app, event| {
            match event {
                // Whatever the way out, the system proxy goes back to the user's.
                tauri::RunEvent::Exit => {
                    if let Some(s) = app.try_state::<servers::ServersState>() {
                        tauri::async_runtime::block_on(s.tunnel.shutdown());
                    }
                }
                // The Dock icon clicked while the window is hidden (macOS).
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen { .. } => desktop::show_main(app),
                _ => {}
            }
        });
}
