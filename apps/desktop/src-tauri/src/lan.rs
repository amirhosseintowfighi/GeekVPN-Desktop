//! «اشتراک در شبکه‌ی محلی»: the settings, this computer's addresses on the
//! local network, and the link and QR a phone imports.

use geek_config::LanShare;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::servers::ServersState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanView {
    pub settings: LanShare,
    /// Whether the running connection carries the share; `None` when not connected.
    pub running: Option<bool>,
    /// This computer's private IPv4 addresses, the network's first.
    pub addresses: Vec<String>,
    /// For the first address; `None` while the share is off.
    pub link: Option<String>,
    pub qr: Option<String>,
}

/// Private IPv4 addresses of real network cards (not our TUN, not loopback).
fn addresses() -> Vec<String> {
    let tun = |n: &str| {
        let n = n.to_ascii_lowercase();
        n.starts_with("geekvpn") || n.starts_with("utun") || n.starts_with("tun") || n.starts_with("wg") || n.starts_with("wintun")
    };
    let default = netdev::get_default_interface().ok().map(|i| i.index);
    let mut found: Vec<(bool, String)> = netdev::get_interfaces()
        .into_iter()
        .filter(|i| !i.is_loopback() && !tun(&i.name) && i.is_up())
        .flat_map(|i| {
            let first = Some(i.index) == default || i.gateway.is_some();
            i.ipv4.iter().filter(|n| n.addr().is_private()).map(move |n| (first, n.addr().to_string())).collect::<Vec<_>>()
        })
        .collect();
    found.sort_by_key(|(first, _)| !first);
    let mut out: Vec<String> = Vec::new();
    for (_, a) in found {
        if !out.contains(&a) {
            out.push(a);
        }
    }
    out
}

fn token(n: usize) -> String {
    uuid::Uuid::new_v4().simple().to_string().chars().take(n).collect()
}

async fn view(state: &ServersState) -> Result<LanView, String> {
    let settings = state.store.lock().await.data.lan_share.clone();
    let connected = matches!(*state.tunnel.state.lock().await, crate::tunnel::TunnelState::On { .. });
    let running = if connected { Some(state.tunnel.lan_running().await.is_some()) } else { None };
    let addresses = tauri::async_runtime::spawn_blocking(addresses).await.map_err(|e| e.to_string())?;
    let link = (settings.on && settings.valid()).then(|| addresses.first().map(|ip| settings.link(ip))).flatten();
    let qr = link.as_deref().map(crate::auth::qr_svg).transpose()?;
    Ok(LanView { settings, running, addresses, link, qr })
}

#[tauri::command]
pub async fn lan_view(state: State<'_, ServersState>) -> Result<LanView, String> {
    view(&state).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanPatch {
    on: Option<bool>,
    port: Option<u16>,
    auth: Option<bool>,
    /// A new random password.
    #[serde(default)]
    renew: bool,
}

/// Changes take effect from the next connection.
#[tauri::command]
pub async fn lan_set(state: State<'_, ServersState>, patch: LanPatch) -> Result<LanView, String> {
    {
        let mut store = state.store.lock().await;
        let mut s = store.data.lan_share.clone();
        if let Some(on) = patch.on {
            s.on = on;
        }
        if let Some(port) = patch.port {
            if port < 1024 {
                return Err("پورت باید ۱۰۲۴ یا بیشتر باشد.".into());
            }
            s.port = port;
        }
        if let Some(auth) = patch.auth {
            s.auth = auth;
        }
        if s.user.is_empty() {
            s.user = "geek".into();
        }
        if s.pass.is_empty() || patch.renew {
            s.pass = token(10);
        }
        store.data.lan_share = s;
        store.save()?;
    }
    view(&state).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn addresses_are_private_v4() {
        for a in super::addresses() {
            let ip: std::net::Ipv4Addr = a.parse().unwrap();
            assert!(ip.is_private(), "{a}");
        }
    }
}
