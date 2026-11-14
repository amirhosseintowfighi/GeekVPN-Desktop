//! «شبکه‌ها»: what to do when the computer joins a network. A network is
//! told apart as the clean-IP scanner does (`scan::current_network`: the
//! gateway and the DNS servers it hands out, no Location permission). Each
//! remembered network either connects the VPN («وصل شو») or is trusted and
//! takes it down («شبکه‌ی امن»); a network with no rule can connect too
//! («شبکه‌های جدید»). Only a change of network acts, so a manual connect or
//! disconnect on the same network is never undone.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::desktop::notify;
use crate::scan::{current_network, Network};
use crate::servers::ServersState;
use crate::tunnel::TunnelState;

const EVERY: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetAction {
    /// Connect on joining this network.
    Connect,
    /// A trusted network (home, work): disconnect on joining it.
    Trusted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetRule {
    /// The customer's name for it («Wi-Fi خانه»).
    pub name: String,
    pub action: NetAction,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NetRules {
    /// Network key → rule.
    pub rules: BTreeMap<String, NetRule>,
    /// Connect on a network that has no rule.
    pub connect_on_new: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Nothing,
    Connect,
    Disconnect,
}

impl NetRules {
    /// What joining `key` should do, given whether the tunnel is up (or coming up).
    pub fn step(&self, key: &str, busy: bool) -> Step {
        if key == OFFLINE {
            return Step::Nothing;
        }
        match self.rules.get(key).map(|r| r.action) {
            Some(NetAction::Trusted) if busy => Step::Disconnect,
            Some(NetAction::Connect) if !busy => Step::Connect,
            None if self.connect_on_new && !busy => Step::Connect,
            _ => Step::Nothing,
        }
    }
}

/// `current_network`'s key when there is no network at all.
const OFFLINE: &str = "other";

/// What the watcher remembers between looks.
#[derive(Debug, Default)]
pub struct Watch {
    last: Option<String>,
    was_busy: bool,
}

impl Watch {
    /// The network seen now (and whether the tunnel is up): `Some(key)` when
    /// it is a new one to act on. The first look only remembers (start-up has
    /// its own auto-connect), and so does the moment the tunnel goes down,
    /// so a manual disconnect is not undone. While the tunnel is up, the TUN
    /// can change what the network looks like, so only the trusted check runs
    /// and the network it was joined on stays the reference.
    pub fn observe(&mut self, key: &str, busy: bool) -> Option<String> {
        let went_down = self.was_busy && !busy;
        self.was_busy = busy;
        if key == OFFLINE {
            return None;
        }
        let Some(last) = &self.last else {
            self.last = Some(key.to_string());
            return None;
        };
        if went_down {
            self.last = Some(key.to_string());
            return None;
        }
        if last == key {
            return None;
        }
        if !busy {
            self.last = Some(key.to_string());
        }
        Some(key.to_string())
    }
}

fn busy(state: &TunnelState) -> bool {
    matches!(state, TunnelState::On { .. } | TunnelState::Connecting { .. })
}

/// Watches the network for the life of the app.
pub fn watch(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut seen = Watch::default();
        loop {
            tokio::time::sleep(EVERY).await;
            let Ok(net) = tauri::async_runtime::spawn_blocking(current_network).await else { continue };
            let state = app.state::<ServersState>();
            let up = busy(&*state.tunnel.state.lock().await);
            let Some(key) = seen.observe(&net.key, up) else { continue };
            let (step, name, has_servers) = {
                let store = state.store.lock().await;
                let rules = &store.data.networks;
                let name = rules.rules.get(&key).map(|r| r.name.clone()).unwrap_or_else(|| net.label.clone());
                (rules.step(&key, up), name, !store.pool().is_empty())
            };
            match step {
                Step::Connect if has_servers => {
                    notify(&app, "شبکه عوض شد", &format!("به «{name}» وصل شدی؛ VPN خودکار روشن می‌شود."));
                    let _ = crate::servers::connect(&app).await;
                }
                Step::Disconnect => {
                    state.tunnel.disconnect(&app).await;
                    notify(&app, "شبکه‌ی امن", &format!("روی «{name}» هستی؛ VPN خودکار قطع شد."));
                }
                _ => {}
            }
        }
    });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworksView {
    pub current: Network,
    pub rules: Vec<RuleView>,
    pub connect_on_new: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleView {
    pub key: String,
    #[serde(flatten)]
    pub rule: NetRule,
}

async fn view(state: &ServersState) -> Result<NetworksView, String> {
    let current = tauri::async_runtime::spawn_blocking(current_network).await.map_err(|e| e.to_string())?;
    let store = state.store.lock().await;
    let n = &store.data.networks;
    Ok(NetworksView {
        current,
        rules: n.rules.iter().map(|(k, r)| RuleView { key: k.clone(), rule: r.clone() }).collect(),
        connect_on_new: n.connect_on_new,
    })
}

#[tauri::command]
pub async fn networks_view(state: State<'_, ServersState>) -> Result<NetworksView, String> {
    view(&state).await
}

/// Sets (or, with no `action`, forgets) the rule for `key`.
#[tauri::command]
pub async fn network_rule_set(state: State<'_, ServersState>, key: String, name: String, action: Option<NetAction>) -> Result<NetworksView, String> {
    if key.is_empty() || key == OFFLINE {
        return Err("به شبکه‌ای وصل نیستی.".into());
    }
    {
        let mut store = state.store.lock().await;
        match action {
            Some(action) => {
                let name: String = name.trim().chars().take(40).collect();
                let name = if name.is_empty() { "شبکه".to_string() } else { name };
                store.data.networks.rules.insert(key, NetRule { name, action });
            }
            None => {
                store.data.networks.rules.remove(&key);
            }
        }
        store.save()?;
    }
    view(&state).await
}

#[tauri::command]
pub async fn network_connect_on_new(state: State<'_, ServersState>, on: bool) -> Result<NetworksView, String> {
    {
        let mut store = state.store.lock().await;
        store.data.networks.connect_on_new = on;
        store.save()?;
    }
    view(&state).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> NetRules {
        let mut r = NetRules::default();
        r.rules.insert("net:home".into(), NetRule { name: "خانه".into(), action: NetAction::Trusted });
        r.rules.insert("net:cafe".into(), NetRule { name: "کافه".into(), action: NetAction::Connect });
        r
    }

    #[test]
    fn rules_decide_by_network_and_tunnel() {
        let r = rules();
        assert_eq!(r.step("net:home", true), Step::Disconnect);
        assert_eq!(r.step("net:home", false), Step::Nothing);
        assert_eq!(r.step("net:cafe", false), Step::Connect);
        assert_eq!(r.step("net:cafe", true), Step::Nothing);
        assert_eq!(r.step("net:new", false), Step::Nothing);
        let open = NetRules { connect_on_new: true, ..r };
        assert_eq!(open.step("net:new", false), Step::Connect);
        assert_eq!(open.step(OFFLINE, false), Step::Nothing);
    }

    #[test]
    fn only_a_change_of_network_acts() {
        let mut w = Watch::default();
        assert_eq!(w.observe("net:a", false), None, "the first look only remembers");
        assert_eq!(w.observe("net:a", false), None);
        assert_eq!(w.observe(OFFLINE, false), None, "dropping off the network is not a change");
        assert_eq!(w.observe("net:a", false), None, "nor is coming back to the same one");
        assert_eq!(w.observe("net:b", false).as_deref(), Some("net:b"));
        assert_eq!(w.observe("net:b", false), None);
    }

    #[test]
    fn the_tunnel_going_down_is_not_a_new_network() {
        let mut w = Watch::default();
        w.observe("net:a", false);
        // The TUN changes what the network looks like: seen, not adopted.
        assert_eq!(w.observe("net:a-under-tun", true).as_deref(), Some("net:a-under-tun"));
        // The customer disconnects: whatever is seen now is the reference.
        assert_eq!(w.observe("net:b", false), None);
        assert_eq!(w.observe("net:b", false), None);
        assert_eq!(w.observe("net:c", false).as_deref(), Some("net:c"));
    }
}
