//! The sing-box config for TUN mode.
//!
//! sing-box owns the TUN device and the routing decision (Iran direct, apps
//! in or out, the local network); whatever should go through the tunnel it
//! hands to geekcore's SOCKS inbound, where Xray talks to the server.
//! geekcore's own connections come back in through the TUN, and are sent
//! straight out by process path, or the tunnel would feed itself.
//!
//! The app describes what it wants as a [`TunSpec`]; only the privileged
//! helper turns that into a config, together with the [`TunHost`] facts it
//! alone decides (interface, API port, firewall mark). A caller can never
//! hand the root process a config of its own, with its file paths and
//! listeners.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::geo::RuleLists;
use crate::rules::{singbox_rules, CustomRule};
use crate::xray::Route;

/// Firewall mark on everything sing-box sends itself (Linux): the kill
/// switch lets marked packets out, and nothing else but the TUN.
pub const LINUX_MARK: u32 = 0x2024;

/// geekcore's SOCKS inbound, locked with a per-connection user and password
/// so other programs on the machine cannot use it as an open proxy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Upstream {
    pub port: u16,
    pub username: String,
    pub password: String,
}

/// «تونل برنامه‌ای».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    /// Every program follows the route.
    #[default]
    Off,
    /// The listed programs go direct; the rest follow the route.
    Bypass,
    /// Only the listed programs go through the tunnel.
    Only,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRouting {
    pub mode: AppMode,
    /// Full paths of the programs' executables.
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunSpec {
    pub upstream: Upstream,
    pub route: Route,
    /// What Smart sends direct (see [`crate::iran_rules`]); unused otherwise.
    pub direct: RuleLists,
    pub apps: AppRouting,
    /// The executables of the app's own engine: always direct.
    pub core_paths: Vec<String>,
    /// The customer's own domain and address rules, after the apps'.
    #[serde(default)]
    pub rules: Vec<CustomRule>,
    /// Server IPs that must bypass the TUN (Windows: route via gateway,
    /// otherwise Xray's own connection loops through hev).
    #[serde(default)]
    pub bypass: Vec<String>,
}

/// What the helper decides on its own about the machine.
#[derive(Debug, Clone)]
pub struct TunHost {
    /// Linux and Windows take a name; macOS insists on picking `utunN`.
    pub interface: Option<String>,
    /// Without IPv6 in the kernel, an IPv6 address or `strict_route`
    /// (which adds IPv6 policy rules) makes sing-box refuse to start.
    pub ipv6: bool,
    pub clash_port: u16,
    pub clash_secret: String,
    pub mark: Option<u32>,
}

#[cfg(test)]
mod _tun_spec_bypass {
    use super::*;
    #[test]
    fn bypass_defaults_empty() {
        let s = TunSpec {
            upstream: Upstream { port: 10808, username: "u".into(), password: "p".into() },
            route: crate::xray::Route::Global,
            direct: Default::default(),
            apps: Default::default(),
            core_paths: vec![],
            rules: vec![],
            bypass: vec![],
        };
        assert!(s.bypass.is_empty());
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["bypass"], serde_json::json!([]));
    }
}

pub fn tun_config(spec: &TunSpec, host: &TunHost) -> Value {
    let mut address = vec![json!("172.19.0.1/30")];
    if host.ipv6 {
        address.push(json!("fdfe:dcba:9876::1/126"));
    }
    let mut tun = json!({
        "type": "tun", "tag": "tun",
        "address": address,
        "mtu": 9000,
        "auto_route": true,
        "strict_route": host.ipv6,
        // The kernel's TCP stack, gVisor's for UDP: sing-box's own advice.
        "stack": "mixed",
    });
    if let Some(name) = &host.interface {
        tun["interface_name"] = json!(name);
    }

    let apps: Vec<&String> = spec.apps.paths.iter().filter(|p| !p.is_empty()).collect();
    let app_mode = if apps.is_empty() { AppMode::Off } else { spec.apps.mode };
    let smart = spec.route == Route::Smart && !spec.direct.is_empty();

    let mut rules = vec![
        json!({ "action": "sniff" }),
        json!({ "protocol": "dns", "action": "hijack-dns" }),
        json!({ "process_path": spec.core_paths, "outbound": "direct" }),
        json!({ "ip_is_private": true, "outbound": "direct" }),
    ];
    let mut dns_rules = vec![json!({ "process_path": spec.core_paths, "server": "local" })];
    match app_mode {
        AppMode::Off => {}
        AppMode::Bypass => {
            rules.push(json!({ "process_path": apps, "outbound": "direct" }));
            dns_rules.push(json!({ "process_path": apps, "server": "local" }));
        }
        AppMode::Only => {
            rules.push(json!({ "process_path": apps, "invert": true, "outbound": "direct" }));
            dns_rules.push(json!({ "process_path": apps, "invert": true, "server": "local" }));
        }
    }
    let (custom, custom_dns) = singbox_rules(&spec.rules);
    rules.extend(custom);
    dns_rules.extend(custom_dns);
    let mut rule_set = vec![];
    if smart {
        // Same as Xray's Smart: QUIC blocked so browsers fall back to TCP,
        // which the servers carry better; Iran direct.
        rules.push(json!({ "network": "udp", "port": 443, "action": "reject" }));
        rules.push(json!({ "rule_set": "iran", "outbound": "direct" }));
        dns_rules.push(json!({ "rule_set": "iran", "server": "local" }));
        let d = &spec.direct;
        let mut domains = json!({});
        for (key, list) in [
            ("domain", &d.domain),
            ("domain_suffix", &d.domain_suffix),
            ("domain_keyword", &d.domain_keyword),
            ("domain_regex", &d.domain_regex),
        ] {
            if !list.is_empty() {
                domains[key] = json!(list);
            }
        }
        let mut set_rules = vec![];
        if domains.as_object().is_some_and(|o| !o.is_empty()) {
            set_rules.push(domains);
        }
        if !d.ip_cidr.is_empty() {
            set_rules.push(json!({ "ip_cidr": d.ip_cidr }));
        }
        rule_set.push(json!({ "type": "inline", "tag": "iran", "rules": set_rules }));
    }
    let through = spec.route != Route::Direct;

    let mut route = json!({
        "rules": rules,
        "rule_set": rule_set,
        "final": if through { "proxy" } else { "direct" },
        "auto_detect_interface": true,
        "find_process": true,
        "default_domain_resolver": "local",
    });
    if let Some(mark) = host.mark {
        route["default_mark"] = json!(mark);
    }

    json!({
        "log": { "level": "warn", "timestamp": true },
        "dns": {
            "servers": [
                // Names are resolved on the far side of the tunnel, so the
                // ISP sees no lookups for what goes through it.
                { "type": "https", "tag": "remote", "server": "1.1.1.1", "detour": "proxy" },
                { "type": "local", "tag": "local" },
            ],
            "rules": dns_rules,
            "final": if through { "remote" } else { "local" },
        },
        "inbounds": [tun],
        "outbounds": [
            { "type": "socks", "tag": "proxy", "server": "127.0.0.1", "server_port": spec.upstream.port,
              "version": "5", "username": spec.upstream.username, "password": spec.upstream.password },
            { "type": "direct", "tag": "direct" },
        ],
        "route": route,
        "experimental": {
            "clash_api": {
                "external_controller": format!("127.0.0.1:{}", host.clash_port),
                "secret": host.clash_secret,
            },
        },
    })
}
