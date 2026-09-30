//! Full Xray client configs, and the minimal ones the delay test uses.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::link::Server;
use crate::rules::{xray_rules, CustomRule};
use crate::singbox::Upstream;

/// «مسیر ترافیک», with the Android app's meaning of each choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Route {
    /// v2rayNG's `WHITE_IRAN`: Iranian sites and addresses direct, the rest
    /// through the tunnel.
    #[default]
    Smart,
    /// Everything through the tunnel except the local network.
    Global,
    /// Nothing through the tunnel (connected, but bypassed).
    Direct,
}

/// The local ports the system proxy points at.
#[derive(Debug, Clone, Copy)]
pub struct LocalPorts {
    pub socks: u16,
    pub http: u16,
}

/// The config the tunnel runs: SOCKS and HTTP on loopback, the server as
/// outbound `proxy`, `direct` and `block` beside it, and `route`'s rules.
/// `address_override` is the scanner's clean IP for this server, if any;
/// `custom` are the customer's own rules, which win over the route's.
pub fn client_config(server: &Server, address_override: Option<&str>, route: Route, custom: &[CustomRule], ports: LocalPorts) -> Value {
    let mut rules = xray_rules(custom);
    rules.extend(route_rules(route).as_array().cloned().unwrap_or_default());
    let sniffing = json!({ "enabled": true, "destOverride": ["http", "tls", "quic"], "routeOnly": true });
    json!({
        "log": { "loglevel": "warning" },
        "inbounds": [
            { "tag": "socks", "listen": "127.0.0.1", "port": ports.socks, "protocol": "socks",
              "settings": { "udp": true, "auth": "noauth" }, "sniffing": sniffing },
            { "tag": "http", "listen": "127.0.0.1", "port": ports.http, "protocol": "http",
              "settings": {}, "sniffing": sniffing },
        ],
        "outbounds": [
            server.outbound("proxy", address_override),
            { "tag": "direct", "protocol": "freedom", "settings": {} },
            { "tag": "block", "protocol": "blackhole", "settings": {} },
        ],
        "routing": {
            // AsIs: domains are matched by name (sniffed), never resolved
            // locally, so choosing a route cannot leak a DNS query.
            "domainStrategy": "AsIs",
            "rules": rules,
        },
        // Byte counters for the live speed; read by geekcore's core.traffic.
        "stats": {},
        "policy": { "system": { "statsOutboundUplink": true, "statsOutboundDownlink": true } },
    })
}

/// The engine behind sing-box in TUN mode: one SOCKS inbound locked with
/// `upstream`'s credentials, everything it receives to the server. The
/// routing choice was already made in sing-box.
pub fn tun_upstream_config(server: &Server, address_override: Option<&str>, upstream: &Upstream) -> Value {
    json!({
        "log": { "loglevel": "warning" },
        "inbounds": [
            { "tag": "socks", "listen": "127.0.0.1", "port": upstream.port, "protocol": "socks",
              "settings": { "udp": true, "ip": "127.0.0.1", "auth": "password",
                            "accounts": [{ "user": upstream.username, "pass": upstream.password }] },
              "sniffing": { "enabled": true, "destOverride": ["http", "tls", "quic"], "routeOnly": true } },
        ],
        "outbounds": [server.outbound("proxy", address_override)],
        "routing": { "domainStrategy": "AsIs", "rules": [{ "type": "field", "outboundTag": "proxy", "network": "tcp,udp" }] },
        "stats": {},
        "policy": { "system": { "statsOutboundUplink": true, "statsOutboundDownlink": true } },
    })
}

/// Just the server, for the real-delay test: geekcore dials through the
/// first outbound, no inbounds and no routing needed.
pub fn delay_config(server: &Server, address_override: Option<&str>) -> Value {
    json!({
        "log": { "loglevel": "none" },
        "outbounds": [server.outbound("proxy", address_override)],
    })
}

fn route_rules(route: Route) -> Value {
    let lan = [
        json!({ "type": "field", "outboundTag": "direct", "ip": ["geoip:private"] }),
        json!({ "type": "field", "outboundTag": "direct", "domain": ["geosite:private"] }),
    ];
    match route {
        // assets/custom_routing_white_iran in v2rayNG, rule for rule.
        Route::Smart => json!([
            { "type": "field", "outboundTag": "block", "port": "443", "network": "udp" },
            lan[0], lan[1],
            { "type": "field", "outboundTag": "direct", "domain": ["domain:ir", "geosite:category-ir"] },
            { "type": "field", "outboundTag": "direct", "ip": ["geoip:ir"] },
        ]),
        Route::Global => json!([lan[0], lan[1], { "type": "field", "outboundTag": "proxy", "network": "tcp,udp" }]),
        Route::Direct => json!([{ "type": "field", "outboundTag": "direct", "network": "tcp,udp" }]),
    }
}
