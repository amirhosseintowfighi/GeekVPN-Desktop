use geek_config::{AppRouting, Route, RuleLists, Upstream};
use serde_json::json;

use crate::*;

fn spec() -> TunSpec {
    TunSpec {
        upstream: Upstream { port: 1, username: "u".into(), password: "p".into() },
        route: Route::Smart,
        direct: RuleLists::default(),
        apps: AppRouting::default(),
        core_paths: vec!["/x/geekcore".into()],
    }
}

#[test]
fn wire_shapes() {
    let start = serde_json::to_value(Envelope {
        id: 7,
        request: Request::TunStart { spec: Box::new(spec()), kill_switch: Some(KillSwitch { allow_lan: true, strict: false }) },
    })
    .unwrap();
    assert_eq!(start["id"], 7);
    assert_eq!(start["method"], "tunStart");
    assert_eq!(start["killSwitch"]["allowLan"], true);
    assert_eq!(start["spec"]["corePaths"][0], "/x/geekcore");
    assert_eq!(serde_json::to_value(Envelope { id: 1, request: Request::Hello }).unwrap(), json!({ "id": 1, "method": "hello" }));

    let ok: Incoming = serde_json::from_value(json!({ "id": 3, "ok": { "protocol": 1 } })).unwrap();
    assert!(matches!(ok, Incoming::Reply { id: 3, outcome: Outcome::Ok(_) }));
    let err: Incoming = serde_json::from_value(json!({ "id": 4, "error": { "code": "noEngine", "detail": "x" } })).unwrap();
    assert!(matches!(err, Incoming::Reply { id: 4, outcome: Outcome::Error(HelperError { code: ErrorCode::NoEngine, .. }) }));
    let ev: Incoming = serde_json::from_value(json!({ "event": "tunExited", "detail": "boom" })).unwrap();
    assert!(matches!(ev, Incoming::Event(Event::TunExited { .. })));
}

/// The real transport, with a stand-in helper: replies by id, pushes an
/// event, and a closed connection fails the call in flight.
#[cfg(unix)]
#[tokio::test]
async fn client_over_the_socket() {
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let path = std::env::temp_dir().join(format!("geek-ipc-{}.sock", std::process::id()));
    let path = path.to_str().unwrap().to_string();
    let mut listener = bind(&path).unwrap();
    tokio::spawn(async move {
        let s = listener.accept().await.unwrap();
        let (r, mut w) = tokio::io::split(s);
        let mut lines = BufReader::new(r).lines();
        while let Some(line) = lines.next_line().await.unwrap() {
            let env: Envelope = serde_json::from_str(&line).unwrap();
            let reply = match env.request {
                Request::Hello => json!({ "id": env.id, "ok": { "protocol": PROTOCOL, "version": "t", "singBox": "1.14.2" } }),
                Request::TunStop => {
                    w.write_all(b"{\"event\":\"tunExited\",\"detail\":\"gone\"}\n").await.unwrap();
                    json!({ "id": env.id, "error": { "code": "internal", "detail": "no" } })
                }
                // Hang up without answering.
                _ => return,
            };
            w.write_all(format!("{reply}\n").as_bytes()).await.unwrap();
        }
    });

    let c = HelperClient::connect(&path).await.unwrap();
    let mut events = c.events();
    let hello: Hello = c.call(Request::Hello, Duration::from_secs(2)).await.unwrap();
    assert_eq!(hello.protocol, PROTOCOL);
    let err = c.call::<serde_json::Value>(Request::TunStop, Duration::from_secs(2)).await.unwrap_err();
    assert!(matches!(err, IpcError::Helper(HelperError { code: ErrorCode::Internal, .. })));
    assert_eq!(events.recv().await.unwrap(), Event::TunExited { detail: "gone".into() });
    let closed = c.call::<serde_json::Value>(Request::Status, Duration::from_secs(2)).await.unwrap_err();
    assert!(matches!(closed, IpcError::Closed), "{closed:?}");
    let _ = std::fs::remove_file(&path);

    assert!(matches!(HelperClient::connect(&path).await, Err(IpcError::NotRunning(_))));
}
