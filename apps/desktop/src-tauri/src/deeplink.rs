//! `geekvpn://` links: the bank's return page hands the customer back to the
//! app (`geekvpn://payment/result?payment=<id>&result=ok`), as on Android.
//!
//! Only that one link means anything; the rest are ignored. The result in it
//! is a hint from a page anybody can open, never proof of payment: the shop
//! re-reads the payment and the services from the server either way.

use tauri::{AppHandle, Emitter};
use tauri_plugin_deep_link::DeepLinkExt;
use url::Url;

/// What a link asks the app to show, as a route of the UI.
pub fn route(link: &Url) -> Option<String> {
    if link.scheme() != "geekvpn" || link.host_str() != Some("payment") || link.path() != "/result" {
        return None;
    }
    let result = link.query_pairs().find(|(k, _)| k == "result").map(|(_, v)| v.into_owned()).unwrap_or_default();
    let result = match result.as_str() {
        "ok" | "pending" | "failed" => result,
        _ => "unknown".into(),
    };
    Some(format!("/shop?payment={result}"))
}

fn open(app: &AppHandle, urls: &[Url]) {
    if let Some(path) = urls.iter().find_map(route) {
        crate::desktop::show_main(app);
        let _ = app.emit(crate::desktop::NAVIGATE, path);
    }
}

pub fn setup(app: &tauri::App) {
    let handle = app.handle().clone();
    // An AppImage has no installer to register the scheme, and a debug build
    // runs from the source tree.
    #[cfg(any(target_os = "linux", windows))]
    if cfg!(debug_assertions) || std::env::var_os("APPIMAGE").is_some() {
        if let Err(e) = app.deep_link().register_all() {
            eprintln!("geekvpn: geekvpn:// not registered: {e}");
        }
    }
    let on_open = handle.clone();
    app.deep_link().on_open_url(move |event| open(&on_open, &event.urls()));
    // Started by the link itself.
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        let later = handle.clone();
        tauri::async_runtime::spawn(async move {
            // The UI listens for navigation once it has loaded.
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            open(&later, &urls);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::route;
    use url::Url;

    #[test]
    fn only_the_payment_result_link_leads_anywhere() {
        let r = |s: &str| route(&Url::parse(s).unwrap());
        assert_eq!(r("geekvpn://payment/result?payment=abc&result=ok").as_deref(), Some("/shop?payment=ok"));
        assert_eq!(r("geekvpn://payment/result?payment=abc&result=pending").as_deref(), Some("/shop?payment=pending"));
        assert_eq!(r("geekvpn://payment/result?result=<script>").as_deref(), Some("/shop?payment=unknown"));
        assert_eq!(r("geekvpn://payment/result").as_deref(), Some("/shop?payment=unknown"));
        assert_eq!(r("geekvpn://settings/split"), None);
        assert_eq!(r("https://payment/result?result=ok"), None);
    }
}
