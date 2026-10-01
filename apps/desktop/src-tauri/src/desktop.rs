//! Living beside the clock: the tray icon and its menu, the small panel
//! (flyout), closing to the tray, starting with the system, connecting on
//! launch, the global shortcut, and the notifications the Android app sends
//! (a summary after a disconnect, a service running out).

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Rect, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;

use crate::servers::ServersState;
use crate::store::{Persisted, SourceKind};
use crate::tunnel::TunnelState;

const TRAY: &str = "main";
const FLYOUT: &str = "flyout";
/// Sent to the main window to open a page («تنظیمات» from the flyout).
pub const NAVIGATE: &str = "desktop://navigate";
/// The argument autostart launches us with: start in the tray, no window.
pub const MINIMIZED: &str = "--minimized";
const SHORTCUT: &str = "CommandOrControl+Shift+K";

/// Settings the window and tray callbacks read without awaiting the store.
pub struct DesktopState {
    close_to_tray: AtomicBool,
    toggle: MenuItem<tauri::Wry>,
    tray: TrayIcon,
}

fn icon(name: &str) -> Option<Image<'static>> {
    let bytes: &'static [u8] = match name {
        "on" => include_bytes!("../icons/tray-on.png"),
        "busy" => include_bytes!("../icons/tray-busy.png"),
        _ => include_bytes!("../icons/tray-off.png"),
    };
    Image::from_bytes(bytes).ok()
}

/// `resume`: an update stopped a connection; the new version brings it back.
pub fn setup(app: &tauri::App, data: &Persisted, resume: bool) -> tauri::Result<()> {
    let handle = app.handle();
    let toggle = MenuItem::with_id(handle, "toggle", "اتصال", true, None::<&str>)?;
    let open = MenuItem::with_id(handle, "open", "باز کردن GeekVPN", true, None::<&str>)?;
    let quit = MenuItem::with_id(handle, "quit", "خروج از GeekVPN", true, None::<&str>)?;
    let menu = Menu::with_items(handle, &[&toggle, &open, &PredefinedMenuItem::separator(handle)?, &quit])?;

    let mut tray = TrayIconBuilder::with_id(TRAY)
        .tooltip("GeekVPN · قطع")
        .menu(&menu)
        // Left click: the small panel; right click: the system's own menu.
        // Linux trays (AppIndicator) report no clicks, only the menu.
        .show_menu_on_left_click(cfg!(target_os = "linux"))
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle_connection(app),
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, rect, .. } = event {
                toggle_flyout(tray.app_handle(), rect);
            }
        });
    if let Some(i) = icon("off") {
        tray = tray.icon(i);
    }
    let tray = tray.build(app)?;

    app.manage(DesktopState { close_to_tray: AtomicBool::new(data.close_to_tray), toggle, tray });
    apply(handle, data);

    // Started by autostart: stay in the tray.
    if !std::env::args().any(|a| a == MINIMIZED) {
        show_main(handle);
    }
    if data.auto_connect || resume {
        let app = handle.clone();
        tauri::async_runtime::spawn(async move {
            // Let the session revalidate and the store settle first.
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let _ = crate::servers::connect(&app).await;
        });
    }
    Ok(())
}

/// Settings that take effect at once: close-to-tray and the shortcut.
pub fn apply(app: &AppHandle, data: &Persisted) {
    if let Some(d) = app.try_state::<DesktopState>() {
        d.close_to_tray.store(data.close_to_tray, Ordering::SeqCst);
    }
    #[cfg(desktop)]
    {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        let gs = app.global_shortcut();
        let on = gs.is_registered(SHORTCUT);
        if data.shortcut && !on {
            // Another program may own it; the app works without.
            if let Err(e) = gs.register(SHORTCUT) {
                eprintln!("geekvpn: shortcut {SHORTCUT}: {e}");
            }
        } else if !data.shortcut && on {
            let _ = gs.unregister(SHORTCUT);
        }
    }
}

/// Whether closing the main window should only hide it.
pub fn closes_to_tray(app: &AppHandle) -> bool {
    app.try_state::<DesktopState>().is_some_and(|d| d.close_to_tray.load(Ordering::SeqCst))
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    if let Some(f) = app.get_webview_window(FLYOUT) {
        let _ = f.hide();
    }
}

/// Connects, or disconnects when connected (or connecting), from the tray
/// menu and the shortcut.
pub fn toggle_connection(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<ServersState>();
        let busy = matches!(*state.tunnel.state.lock().await, TunnelState::On { .. } | TunnelState::Connecting { .. });
        if busy {
            state.tunnel.disconnect(&app).await;
        } else {
            let _ = crate::servers::connect(&app).await;
        }
    });
}

/// The small panel, beside the tray icon: above it with the taskbar at the
/// bottom (Windows), below it with the menu bar at the top (macOS).
fn toggle_flyout(app: &AppHandle, icon: Rect) {
    let window = match app.get_webview_window(FLYOUT) {
        Some(w) => w,
        None => {
            let Ok(w) = WebviewWindowBuilder::new(app, FLYOUT, WebviewUrl::App("index.html#/flyout".into()))
                .title("GeekVPN")
                .inner_size(360.0, 560.0)
                .resizable(false)
                .decorations(false)
                .skip_taskbar(true)
                .always_on_top(true)
                .visible(false)
                .build()
            else {
                return;
            };
            let hide = w.clone();
            w.on_window_event(move |e| {
                if let tauri::WindowEvent::Focused(false) = e {
                    let _ = hide.hide();
                }
            });
            w
        }
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    let scale = window.scale_factor().unwrap_or(1.0);
    let pos: PhysicalPosition<f64> = icon.position.to_physical(scale);
    let size: tauri::PhysicalSize<f64> = icon.size.to_physical(scale);
    let (w, h) = (360.0 * scale, 560.0 * scale);
    let gap = 8.0 * scale;
    let mut x = pos.x + size.width / 2.0 - w / 2.0;
    let mut y = pos.y - h - gap;
    if let Ok(Some(m)) = app.monitor_from_point(pos.x, pos.y) {
        let (mx, my) = (m.position().x as f64, m.position().y as f64);
        let (mw, mh) = (m.size().width as f64, m.size().height as f64);
        // Icon in the top half: the menu bar is at the top.
        if pos.y < my + mh / 2.0 {
            y = pos.y + size.height + gap;
        }
        x = x.clamp(mx + gap, mx + mw - w - gap);
        y = y.clamp(my + gap, my + mh - h - gap);
    }
    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.show();
    let _ = window.set_focus();
}

/// Called on every tunnel state change: the tray follows, and the moments
/// the user may not be watching become notifications.
pub fn on_state(app: &AppHandle, before: &TunnelState, now: &TunnelState, totals: (i64, i64)) {
    if let Some(d) = app.try_state::<DesktopState>() {
        let (name, tip, label) = match now {
            TunnelState::On { server_name, .. } => ("on", format!("GeekVPN · متصل · {}", plain(server_name)), "قطع"),
            TunnelState::Connecting { .. } => ("busy", "GeekVPN · در حال اتصال…".to_string(), "لغو اتصال"),
            TunnelState::Failed { .. } => ("off", "GeekVPN · وصل نشد".to_string(), "اتصال"),
            TunnelState::Off => ("off", "GeekVPN · قطع".to_string(), "اتصال"),
        };
        let _ = d.tray.set_icon(icon(name));
        let _ = d.tray.set_tooltip(Some(&tip));
        let _ = d.toggle.set_text(label);
    }
    if let TunnelState::On { since_ms, server_name, .. } = before {
        let secs = (now_ms().saturating_sub(*since_ms) / 1000) as i64;
        let summary = format!("مدت {} · دانلود {} · آپلود {}", clock(secs), bytes(totals.1), bytes(totals.0));
        match now {
            TunnelState::Off => notify(app, &format!("اتصال به {} قطع شد", plain(server_name)), &summary),
            TunnelState::Failed { message, .. } => notify(app, "اتصال افتاد", message),
            _ => {}
        }
    }
}

/// «هشدار تمام شدن سرویس», once a day per service: 80% of the quota used,
/// or three days or fewer left. Returns true when the store changed.
pub fn check_expiry(app: &AppHandle, data: &mut Persisted) -> bool {
    if !data.expiry_alert {
        return false;
    }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut changed = false;
    for src in &data.sources {
        let SourceKind::Account { subscription_id, state, expires_at, quota_gib, used_gib, .. } = &src.kind else { continue };
        if state != "active" || data.expiry_warned.get(subscription_id) == Some(&today) {
            continue;
        }
        let days = expires_at
            .as_deref()
            .and_then(|e| chrono::DateTime::parse_from_rfc3339(e).ok())
            .map(|e| (e.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_hours().max(0) as f64 / 24.0);
        let used = quota_gib.filter(|q| *q > 0.0).map(|q| used_gib / q);
        let text = match (days, used) {
            (Some(d), _) if d <= 3.0 => format!("{} روز تا پایان «{}» مانده است.", fa_digits(&(d.ceil() as i64).to_string()), src.name),
            (_, Some(u)) if u >= 0.8 => format!("{}٪ حجم «{}» مصرف شده است.", fa_digits(&((u * 100.0).round() as i64).to_string()), src.name),
            _ => continue,
        };
        notify(app, "سرویس رو به پایان است", &text);
        data.expiry_warned.insert(subscription_id.clone(), today.clone());
        changed = true;
    }
    changed
}

pub fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default()
}

/// A server name without its flag emoji (notifications show them badly).
pub fn plain(name: &str) -> String {
    name.chars().filter(|c| !('\u{1F1E6}'..='\u{1F1FF}').contains(c)).collect::<String>().trim().to_string()
}

fn fa_digits(s: &str) -> String {
    s.chars().map(|c| c.to_digit(10).map(|d| char::from_u32(0x06F0 + d).unwrap_or(c)).unwrap_or(c)).collect()
}

fn clock(secs: i64) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, secs % 3600 / 60, secs % 60)
}

fn bytes(n: i64) -> String {
    let n = n.max(0) as f64;
    if n >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} GB", n / 1024f64.powi(3))
    } else if n >= 1024.0 * 1024.0 {
        format!("{:.1} MB", n / 1024f64.powi(2))
    } else {
        format!("{:.0} KB", n / 1024.0)
    }
}

// -- Commands --------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Autostart {
    enabled: bool,
}

/// «اجرا با روشن شدن سیستم»: read it, or set it with `enabled`.
#[tauri::command]
pub fn desktop_autostart(app: AppHandle, enabled: Option<bool>) -> Result<Autostart, String> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    match enabled {
        Some(true) => al.enable().map_err(|e| e.to_string())?,
        Some(false) => al.disable().map_err(|e| e.to_string())?,
        None => {}
    }
    Ok(Autostart { enabled: al.is_enabled().unwrap_or(false) })
}

/// From the flyout: the main window, optionally at a page.
#[tauri::command]
pub fn desktop_open(app: AppHandle, path: Option<String>) {
    show_main(&app);
    if let Some(p) = path {
        let _ = app.emit_to("main", NAVIGATE, p);
    }
}

#[tauri::command]
pub fn desktop_quit(app: AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    #[test]
    fn figures() {
        assert_eq!(super::clock(3725), "01:02:05");
        assert_eq!(super::bytes(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(super::fa_digits("80"), "۸۰");
        assert_eq!(super::plain("🇩🇪 Germany"), "Germany");
    }
}
