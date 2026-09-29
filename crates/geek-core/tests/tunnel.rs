//! The real engine, end to end: a local VLESS server (itself a geekcore),
//! a client tunnel to it, and an HTTP request through the tunnel's proxy.
//!
//! Needs the built binary: `scripts/build-geekcore.sh` puts it where
//! `GEEKCORE` defaults to. Skipped with a message if it is missing.

use std::path::PathBuf;
use std::time::Duration;

use geek_config::{client_config, delay_config, LocalPorts, Route, Server};
use geek_core::{CoreError, CoreProcess};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn binary() -> Option<PathBuf> {
    let p = std::env::var("GEEKCORE").map(PathBuf::from).unwrap_or_else(|_| {
        let triple = if cfg!(target_os = "linux") { "x86_64-unknown-linux-gnu" } else { "unsupported" };
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../apps/desktop/src-tauri/binaries/geekcore-{triple}"))
    });
    p.exists().then_some(p)
}

fn geo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src-tauri/resources/geo")
}

async fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap().port()
}

/// A tiny HTTP origin that answers 204, standing in for generate_204 and for
/// "a website".
async fn origin() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = l.accept().await else { return };
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf).await;
                let _ = s.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
            });
        }
    });
    port
}

const UUID: &str = "b831381d-6324-4d53-ad4f-8cda48b30811";

#[tokio::test(flavor = "multi_thread")]
async fn a_vless_tunnel_carries_http_through_the_local_proxy() {
    let Some(bin) = binary() else {
        eprintln!("geekcore not built; run scripts/build-geekcore.sh");
        return;
    };
    // The server side: plain VLESS over ws, the tunnel services' shape, which
    // only works with the allow-plain-vless patch applied.
    let server_port = free_port().await;
    let server = CoreProcess::spawn(&bin, &geo()).unwrap();
    server
        .call(
            "core.start",
            json!({ "config": {
                "log": { "loglevel": "warning" },
                "inbounds": [{ "listen": "127.0.0.1", "port": server_port, "protocol": "vless",
                    "settings": { "clients": [{ "id": UUID }], "decryption": "none" },
                    "streamSettings": { "network": "ws", "wsSettings": { "path": "/t" } } }],
                // Xray 26 blocks private targets from proxied traffic by default;
                // the test origin lives on loopback, so this server allows it.
                "outbounds": [{ "protocol": "freedom", "settings": { "finalRules": [{ "action": "allow", "ip": ["127.0.0.0/8"] }] } }]
            }}),
            Duration::from_secs(10),
        )
        .await
        .expect("server start");

    let link = format!("vless://{UUID}@127.0.0.1:{server_port}?type=ws&path=%2Ft&security=none#local");
    let s = Server::parse(&link).unwrap();
    let web = origin().await;

    let client = CoreProcess::spawn(&bin, &geo()).unwrap();
    // The delay test first, as the app does before connecting.
    let probe = format!("http://127.0.0.1:{web}/generate_204");
    let results = client
        .call(
            "test.delay",
            json!({ "items": [{ "id": s.id, "config": delay_config(&s, None) }], "url": probe, "concurrency": 4 }),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
    assert!(results[0]["ms"].as_i64().unwrap() >= 0, "delay: {results}");

    // Global: local addresses would otherwise go direct and prove nothing.
    let ports = LocalPorts { socks: free_port().await, http: free_port().await };
    let cfg = client_config(&s, None, Route::Global, ports);
    // Reach the origin through the tunnel even though it is on loopback.
    let mut cfg = cfg;
    cfg["routing"]["rules"] = json!([{ "type": "field", "outboundTag": "proxy", "network": "tcp,udp" }]);
    client.call("core.start", json!({ "config": cfg }), Duration::from_secs(10)).await.expect("client start");

    // An HTTP proxy request to the origin, as a browser behind the system proxy.
    let mut conn = TcpStream::connect(("127.0.0.1", ports.http)).await.unwrap();
    let req = format!("GET http://127.0.0.1:{web}/ HTTP/1.1\r\nHost: 127.0.0.1:{web}\r\nConnection: close\r\n\r\n");
    conn.write_all(req.as_bytes()).await.unwrap();
    let mut resp = String::new();
    conn.read_to_string(&mut resp).await.unwrap();
    assert!(resp.starts_with("HTTP/1.1 204"), "through the tunnel: {resp}");

    let traffic = client.call("core.traffic", json!({}), Duration::from_secs(5)).await.unwrap();
    assert!(traffic["up"].as_i64().unwrap() > 0 && traffic["down"].as_i64().unwrap() > 0, "{traffic}");

    let live = client.call("core.delay", json!({ "url": probe }), Duration::from_secs(20)).await.unwrap();
    assert!(live["ms"].as_i64().unwrap() >= 0);

    client.call("core.stop", json!({}), Duration::from_secs(5)).await.unwrap();
    assert!(TcpStream::connect(("127.0.0.1", ports.http)).await.is_err(), "stopped core still listens");
    server.kill().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bad_config_is_refused_and_a_crash_is_reported() {
    let Some(bin) = binary() else { return };
    let core = CoreProcess::spawn(&bin, &geo()).unwrap();
    let err = core.call("core.start", json!({ "config": { "outbounds": [{ "protocol": "nope" }] } }), Duration::from_secs(5)).await;
    assert!(matches!(err, Err(CoreError::Refused(_))), "{err:?}");

    let mut events = core.events();
    core.kill().await;
    let exited = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if events.recv().await.unwrap().event == "core.exited" {
                break;
            }
        }
    })
    .await;
    assert!(exited.is_ok(), "no core.exited event");
    assert!(matches!(core.call("version", json!({}), Duration::from_secs(1)).await, Err(CoreError::Exited { .. })));
}
