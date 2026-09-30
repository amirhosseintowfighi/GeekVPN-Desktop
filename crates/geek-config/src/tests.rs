use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::json;

use crate::*;

const VLESS_WS_TLS: &str = "vless://b831381d-6324-4d53-ad4f-8cda48b30811@cdn.example.com:443?encryption=none&security=tls&sni=cdn.example.com&fp=chrome&type=ws&host=cdn.example.com&path=%2Fws%3Fed%3D2048#%F0%9F%87%A9%F0%9F%87%AA%20Germany";

#[test]
fn vless_ws_tls() {
    let s = Server::parse(VLESS_WS_TLS).unwrap();
    assert_eq!(s.protocol, Protocol::Vless);
    assert_eq!((s.address.as_str(), s.port), ("cdn.example.com", 443));
    assert_eq!((s.network.as_str(), s.security.as_str()), ("ws", "tls"));
    assert_eq!(s.path, "/ws?ed=2048");
    assert_eq!(s.name, "🇩🇪 Germany");
    assert!(s.cdn_fronted());

    let o = s.outbound("proxy", Some("104.16.1.1"));
    assert_eq!(o["settings"]["vnext"][0]["address"], "104.16.1.1");
    assert_eq!(o["settings"]["vnext"][0]["users"][0]["id"], "b831381d-6324-4d53-ad4f-8cda48b30811");
    // The clean IP replaces the address only; SNI and Host keep the domain.
    assert_eq!(o["streamSettings"]["tlsSettings"]["serverName"], "cdn.example.com");
    assert_eq!(o["streamSettings"]["wsSettings"]["host"], "cdn.example.com");
    assert_eq!(o["streamSettings"]["tlsSettings"]["fingerprint"], "chrome");
}

#[test]
fn vless_reality_and_plain_tunnel() {
    let r = Server::parse("vless://u@1.2.3.4:8443?security=reality&sni=www.microsoft.com&pbk=PUBKEY&sid=ab12&flow=xtls-rprx-vision&type=tcp#r").unwrap();
    let o = r.outbound("proxy", None);
    assert_eq!(o["streamSettings"]["realitySettings"]["publicKey"], "PUBKEY");
    assert_eq!(o["streamSettings"]["realitySettings"]["fingerprint"], "chrome");
    assert_eq!(o["settings"]["vnext"][0]["users"][0]["flow"], "xtls-rprx-vision");
    assert!(!r.cdn_fronted());

    // A tunnel service: plain VLESS over ws on a Cloudflare HTTP port.
    let t = Server::parse("vless://u@t.example.com:2086?type=ws&host=t.example.com&path=%2F#tunnel").unwrap();
    assert_eq!(t.security, "none");
    assert!(t.sni.is_empty());
    assert!(!t.cdn_fronted());
}

#[test]
fn vmess_v2_json_with_string_numbers() {
    let body = json!({"v":"2","ps":"vm","add":"v.example.com","port":"443","id":"uuid-1","aid":"0","scy":"auto",
                       "net":"grpc","type":"none","host":"","path":"svc","tls":"tls","sni":"","fp":"firefox"});
    let link = format!("vmess://{}", STANDARD.encode(body.to_string()));
    let s = Server::parse(&link).unwrap();
    assert_eq!((s.port, s.network.as_str(), s.sni.as_str()), (443, "grpc", "v.example.com"));
    let o = s.outbound("proxy", None);
    assert_eq!(o["streamSettings"]["grpcSettings"]["serviceName"], "svc");
    assert_eq!(o["settings"]["vnext"][0]["users"][0]["security"], "auto");
}

#[test]
fn trojan_defaults_to_tls() {
    let s = Server::parse("trojan://secret@t.example.com:443#t").unwrap();
    assert_eq!((s.security.as_str(), s.sni.as_str()), ("tls", "t.example.com"));
    assert_eq!(s.outbound("p", None)["settings"]["servers"][0]["password"], "secret");
}

#[test]
fn shadowsocks_both_formats() {
    let sip002 = format!("ss://{}@s.example.com:8388#ss", STANDARD.encode("aes-256-gcm:pw"));
    let legacy = format!("ss://{}#ss", STANDARD.encode("chacha20-ietf-poly1305:pw@s.example.com:8388"));
    for l in [sip002, legacy] {
        let s = Server::parse(&l).unwrap();
        assert_eq!((s.address.as_str(), s.port), ("s.example.com", 8388));
        assert_eq!(s.outbound("p", None)["settings"]["servers"][0]["password"], "pw");
    }
}

#[test]
fn rejects_what_it_cannot_run() {
    assert_eq!(Server::parse("hysteria2://x@h:1"), Err(LinkError::Unsupported));
    assert!(Server::parse("vless://@h:443").is_err());
    assert!(Server::parse("vmess://not-base64!").is_err());
}

#[test]
fn ids_are_stable_and_distinct() {
    let a = Server::parse(VLESS_WS_TLS).unwrap();
    assert_eq!(a.id, Server::parse(&format!("  {VLESS_WS_TLS}\n")).unwrap().id);
    assert_ne!(a.id, Server::parse("trojan://secret@t.example.com:443#t").unwrap().id);
}

#[test]
fn subscription_bodies_plain_or_base64_skip_unknown_lines() {
    let lines = format!("{VLESS_WS_TLS}\nhysteria2://x@h:1\ntrojan://secret@t.example.com:443#t\n");
    assert_eq!(parse_subscription(&lines).len(), 2);
    let wrapped = STANDARD.encode(&lines);
    // Some panels wrap base64 at 76 columns.
    let folded: String = wrapped.as_bytes().chunks(76).map(|c| format!("{}\n", std::str::from_utf8(c).unwrap())).collect();
    assert_eq!(parse_subscription(&folded).len(), 2);
    assert!(parse_subscription("garbage").is_empty());
}

#[test]
fn smart_route_is_v2rayngs_white_iran() {
    let s = Server::parse(VLESS_WS_TLS).unwrap();
    let c = client_config(&s, None, Route::Smart, &[], LocalPorts { socks: 10808, http: 10809 });
    let rules = c["routing"]["rules"].as_array().unwrap();
    assert!(rules.iter().any(|r| r["domain"] == json!(["domain:ir", "geosite:category-ir"]) && r["outboundTag"] == "direct"));
    assert!(rules.iter().any(|r| r["ip"] == json!(["geoip:ir"])));
    assert!(rules.iter().any(|r| r["network"] == "udp" && r["port"] == "443" && r["outboundTag"] == "block"));
    assert_eq!(c["inbounds"][0]["listen"], "127.0.0.1");
    assert_eq!(c["inbounds"][1]["port"], 10809);
    assert_eq!(c["outbounds"][0]["tag"], "proxy");

    let direct = client_config(&s, None, Route::Direct, &[], LocalPorts { socks: 1, http: 2 });
    assert_eq!(direct["routing"]["rules"][0]["outboundTag"], "direct");

    // The customer's own rules come first: first match wins in Xray.
    let mine = [CustomRule { value: "example.org".into(), action: RuleAction::Proxy }];
    let c = client_config(&s, None, Route::Direct, &mine, LocalPorts { socks: 1, http: 2 });
    assert_eq!(c["routing"]["rules"][0]["domain"], json!(["domain:example.org"]));
    assert_eq!(c["routing"]["rules"][0]["outboundTag"], "proxy");
}

#[test]
fn tun_puts_the_customers_rules_after_the_engine_and_the_apps() {
    let mut spec = tun_spec(Route::Smart, AppRouting::default());
    spec.rules = vec![CustomRule { value: "digikala.com".into(), action: RuleAction::Proxy }];
    let c = tun_config(&spec, &host(true));
    let rules = c["route"]["rules"].as_array().unwrap();
    let mine = rules.iter().position(|r| r["domain_suffix"] == json!(["digikala.com"])).unwrap();
    let engine = rules.iter().position(|r| r.get("process_path").is_some()).unwrap();
    let iran = rules.iter().position(|r| r["rule_set"] == "iran").unwrap();
    assert!(engine < mine && mine < iran, "before Smart's Iran rules, so a customer can send an Iranian site through the VPN");
    assert_eq!(rules[mine]["outbound"], "proxy");
    assert!(c["dns"]["rules"].as_array().unwrap().iter().any(|r| r["domain_suffix"] == json!(["digikala.com"]) && r["server"] == "remote"));
}

fn tun_spec(route: Route, apps: AppRouting) -> TunSpec {
    TunSpec {
        upstream: Upstream { port: 20808, username: "u".into(), password: "p".into() },
        route,
        direct: RuleLists { domain_suffix: vec!["ir".into()], ip_cidr: vec!["2.144.0.0/14".into()], ..Default::default() },
        apps,
        core_paths: vec!["/opt/GeekVPN/geekcore".into()],
        rules: vec![],
    }
}

fn host(ipv6: bool) -> TunHost {
    TunHost { interface: Some("geekvpn0".into()), ipv6, clash_port: 29090, clash_secret: "s".into(), mark: Some(LINUX_MARK) }
}

/// Where a rule sends matching traffic, found by what it matches on.
fn outbound_for<'a>(c: &'a serde_json::Value, key: &str) -> Vec<&'a serde_json::Value> {
    c["route"]["rules"].as_array().unwrap().iter().filter(|r| r.get(key).is_some()).collect()
}

#[test]
fn tun_sends_the_engine_itself_direct_before_anything_else() {
    let c = tun_config(&tun_spec(Route::Global, AppRouting::default()), &host(true));
    let rules = c["route"]["rules"].as_array().unwrap();
    // sniff, DNS hijack, then the engine: its connections to the server
    // must never re-enter the tunnel.
    assert_eq!(rules[1]["action"], "hijack-dns");
    assert_eq!(rules[2]["process_path"], json!(["/opt/GeekVPN/geekcore"]));
    assert_eq!(rules[2]["outbound"], "direct");
    assert_eq!(c["dns"]["rules"][0]["server"], "local");
    assert_eq!(c["route"]["final"], "proxy");
    assert_eq!(c["route"]["default_mark"], LINUX_MARK);
    assert_eq!(c["outbounds"][0]["username"], "u");
    assert_eq!(c["experimental"]["clash_api"]["external_controller"], "127.0.0.1:29090");
    // Global has no Iran list.
    assert!(outbound_for(&c, "rule_set").is_empty());
}

#[test]
fn tun_smart_routes_iran_direct_like_xray() {
    let c = tun_config(&tun_spec(Route::Smart, AppRouting::default()), &host(true));
    assert_eq!(outbound_for(&c, "rule_set")[0]["outbound"], "direct");
    let set = &c["route"]["rule_set"][0];
    assert_eq!(set["type"], "inline");
    assert_eq!(set["rules"][0]["domain_suffix"], json!(["ir"]));
    assert_eq!(set["rules"][1]["ip_cidr"], json!(["2.144.0.0/14"]));
    assert!(c["route"]["rules"].as_array().unwrap().iter().any(|r| r["network"] == "udp" && r["port"] == 443 && r["action"] == "reject"));
    assert!(c["dns"]["rules"].as_array().unwrap().iter().any(|r| r["rule_set"] == "iran" && r["server"] == "local"));
    assert_eq!(c["dns"]["final"], "remote");

    let direct = tun_config(&tun_spec(Route::Direct, AppRouting::default()), &host(true));
    assert_eq!(direct["route"]["final"], "direct");
    assert_eq!(direct["dns"]["final"], "local");
}

#[test]
fn tun_app_modes() {
    let apps = |mode| AppRouting { mode, paths: vec!["/usr/bin/telegram-desktop".into()] };
    let bypass = tun_config(&tun_spec(Route::Global, apps(AppMode::Bypass)), &host(true));
    let r = outbound_for(&bypass, "process_path");
    assert_eq!((r[1]["process_path"][0].as_str(), r[1]["outbound"].as_str(), r[1].get("invert")), (Some("/usr/bin/telegram-desktop"), Some("direct"), None));

    let only = tun_config(&tun_spec(Route::Global, apps(AppMode::Only)), &host(true));
    let r = outbound_for(&only, "process_path");
    assert_eq!((r[1]["invert"].as_bool(), r[1]["outbound"].as_str()), (Some(true), Some("direct")));

    // "Only these apps" with no apps would send everything direct.
    let empty = tun_config(&tun_spec(Route::Global, AppRouting { mode: AppMode::Only, paths: vec![] }), &host(true));
    assert_eq!(outbound_for(&empty, "process_path").len(), 1);
}

#[test]
fn tun_without_ipv6_skips_what_sing_box_would_refuse() {
    let c = tun_config(&tun_spec(Route::Global, AppRouting::default()), &host(false));
    assert_eq!(c["inbounds"][0]["address"], json!(["172.19.0.1/30"]));
    assert_eq!(c["inbounds"][0]["strict_route"], false);
    let with = tun_config(&tun_spec(Route::Global, AppRouting::default()), &host(true));
    assert_eq!(with["inbounds"][0]["strict_route"], true);
}

#[test]
fn tun_upstream_is_locked_and_sends_everything_to_the_server() {
    let s = Server::parse(VLESS_WS_TLS).unwrap();
    let up = Upstream { port: 20808, username: "u".into(), password: "p".into() };
    let c = tun_upstream_config(&s, None, &up);
    assert_eq!(c["inbounds"].as_array().unwrap().len(), 1);
    assert_eq!(c["inbounds"][0]["settings"]["auth"], "password");
    assert_eq!(c["inbounds"][0]["settings"]["accounts"][0]["pass"], "p");
    assert_eq!(c["routing"]["rules"][0]["outboundTag"], "proxy");
}
