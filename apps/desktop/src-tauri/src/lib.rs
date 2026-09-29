use tauri::Manager;

/// Builds and runs the app. Everything the UI may call is declared in
/// `capabilities/`; the webview gets nothing else.
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
        .run(tauri::generate_context!())
        .expect("GeekVPN failed to start");
}
