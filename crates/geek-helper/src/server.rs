//! The socket loop and the rules of ownership:
//!
//! - The connection that started the tunnel owns it. When that connection
//!   closes (the app quit or crashed), the tunnel stops, since the core it
//!   feeds lived in the app and is gone too.
//! - The kill switch then stays only if it is strict; otherwise it is
//!   released, so a crashed app never leaves the user offline.
//! - If sing-box dies on its own, the owner hears `TunExited` and the kill
//!   switch stays: stopping traffic is the point of it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use geek_ipc::{Envelope, ErrorCode, Event, Hello, HelperError, Incoming, KillSwitch, Outcome, Request, Status, PROTOCOL};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};

use crate::engine::{self, Running};
use crate::firewall::Firewall;
use crate::paths::{self, TUN_NAME};
use crate::state::{State, StateFile};

struct Shared {
    tun: Option<(u64, Running)>,
    /// The connection that last started a tunnel, even if sing-box has since
    /// died: its going away is what releases a non-strict kill switch.
    owner: Option<u64>,
    kill_switch: Option<KillSwitch>,
    firewall: Firewall,
    state: StateFile,
    dir: PathBuf,
    generation: u64,
    /// Each connection's outbox, for events.
    clients: HashMap<u64, mpsc::UnboundedSender<String>>,
}

impl Shared {
    fn persist(&self) {
        self.state.save(&State {
            kill_switch: self.kill_switch,
            engine_pid: self.tun.as_ref().and_then(|(_, r)| r.pid),
        });
    }

    async fn stop_tun(&mut self) {
        if let Some((_, running)) = self.tun.take() {
            running.stop().await;
        }
    }

    fn release_kill_switch(&mut self) -> Result<(), HelperError> {
        self.kill_switch = None;
        self.firewall.release().map_err(|e| HelperError::new(ErrorCode::Firewall, e))
    }

    async fn handle(&mut self, conn: u64, req: Request, exits: &mpsc::UnboundedSender<(u64, String)>) -> Result<Value, HelperError> {
        let out = match req {
            Request::Hello => json!(Hello { protocol: PROTOCOL, version: env!("CARGO_PKG_VERSION").into(), sing_box: engine::version() }),
            Request::Status => json!(Status { tun: self.tun.as_ref().map(|(_, r)| r.info.clone()), kill_switch: self.kill_switch }),
            Request::TunStart { spec, kill_switch } => {
                let engine_bin = paths::engine_binary();
                // Firewall first: from here on nothing leaks, including while
                // the old tunnel goes down and the new one comes up.
                match kill_switch {
                    Some(ks) => {
                        let known = (!cfg!(target_os = "macos")).then_some(TUN_NAME);
                        self.firewall.engage(&ks, known, &engine_bin).map_err(|e| HelperError::new(ErrorCode::Firewall, e))?;
                        self.kill_switch = Some(ks);
                    }
                    None if self.kill_switch.is_some() => self.release_kill_switch()?,
                    None => {}
                }
                self.persist();
                self.stop_tun().await;
                self.generation += 1;
                let running = engine::start(&spec, &self.dir, self.generation, exits.clone()).await?;
                if let Some(ks) = self.kill_switch {
                    // Now that the device exists, let its packets through
                    // where the rules name it (macOS's utunN, Windows' LUID).
                    self.firewall
                        .engage(&ks, Some(&running.info.interface), &engine_bin)
                        .map_err(|e| HelperError::new(ErrorCode::Firewall, e))?;
                }
                let info = running.info.clone();
                self.tun = Some((conn, running));
                self.owner = Some(conn);
                self.persist();
                json!(info)
            }
            Request::TunStop => {
                self.stop_tun().await;
                self.owner = None;
                let r = self.release_kill_switch();
                self.persist();
                r?;
                json!({})
            }
            Request::KillSwitchRelease => {
                let r = self.release_kill_switch();
                self.persist();
                r?;
                json!({})
            }
        };
        Ok(out)
    }

    /// A connection closed.
    async fn gone(&mut self, conn: u64) {
        self.clients.remove(&conn);
        if self.owner == Some(conn) {
            self.owner = None;
            self.stop_tun().await;
            if self.kill_switch.is_some_and(|k| !k.strict) {
                let _ = self.release_kill_switch();
            }
            self.persist();
        }
    }
}

/// Runs until `shutdown` resolves, then leaves the machine as the rules say.
pub async fn serve(shutdown: impl std::future::Future<Output = ()>) -> Result<(), String> {
    let dir = paths::state_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let state = StateFile::new(&dir);
    let previous = state.load();
    let mut firewall = Firewall::default();

    // Whatever a previous run left: its sing-box goes; its kill switch stays
    // only if strict (and is then put back, in case the system rebooted).
    if let Some(pid) = previous.engine_pid {
        engine::kill_stale(pid);
    }
    let kill_switch = match previous.kill_switch {
        Some(ks) if ks.strict => {
            firewall.engage(&ks, None, &paths::engine_binary()).map(|()| Some(ks)).unwrap_or_else(|e| {
                eprintln!("geekvpn-helper: re-engaging the strict kill switch failed: {e}");
                None
            })
        }
        _ => {
            let _ = firewall.release();
            None
        }
    };
    state.save(&State { kill_switch, engine_pid: None });

    let (exits_tx, mut exits_rx) = mpsc::unbounded_channel::<(u64, String)>();
    let shared = Arc::new(Mutex::new(Shared {
        tun: None,
        owner: None,
        kill_switch,
        firewall,
        state,
        dir,
        generation: 0,
        clients: HashMap::new(),
    }));

    // sing-box died on its own.
    {
        let shared = shared.clone();
        tokio::spawn(async move {
            while let Some((generation, detail)) = exits_rx.recv().await {
                let mut s = shared.lock().await;
                let Some(owner) = s.tun.as_ref().filter(|(_, r)| r.generation == generation).map(|(o, _)| *o) else { continue };
                s.tun = None;
                s.persist();
                eprintln!("geekvpn-helper: {detail}");
                if let Some(tx) = s.clients.get(&owner) {
                    let _ = tx.send(serde_json::to_string(&Event::TunExited { detail }).unwrap_or_default());
                }
            }
        });
    }

    let mut listener = geek_ipc::bind(geek_ipc::ENDPOINT).map_err(|e| format!("{}: {e}", geek_ipc::ENDPOINT))?;
    let mut next_conn = 0u64;
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accepted = listener.accept() => {
                let Ok(stream) = accepted else { continue };
                next_conn += 1;
                tokio::spawn(connection(next_conn, stream, shared.clone(), exits_tx.clone()));
            }
        }
    }

    let mut s = shared.lock().await;
    s.stop_tun().await;
    if s.kill_switch.is_some_and(|k| !k.strict) {
        let _ = s.release_kill_switch();
    }
    s.persist();
    #[cfg(unix)]
    let _ = std::fs::remove_file(geek_ipc::ENDPOINT);
    Ok(())
}

async fn connection(id: u64, stream: Box<dyn geek_ipc::Stream>, shared: Arc<Mutex<Shared>>, exits: mpsc::UnboundedSender<(u64, String)>) {
    let (read, mut write) = tokio::io::split(stream);
    let (tx, mut outbox) = mpsc::unbounded_channel::<String>();
    shared.lock().await.clients.insert(id, tx.clone());
    let writer = tokio::spawn(async move {
        while let Some(line) = outbox.recv().await {
            if write.write_all(line.as_bytes()).await.is_err() || write.write_all(b"\n").await.is_err() {
                break;
            }
            let _ = write.flush().await;
        }
    });

    let mut lines = BufReader::new(read).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let reply = match serde_json::from_str::<Envelope>(&line) {
            Ok(env) => {
                let outcome = match shared.lock().await.handle(id, env.request, &exits).await {
                    Ok(v) => Outcome::Ok(v),
                    Err(e) => Outcome::Error(e),
                };
                Incoming::Reply { id: env.id, outcome }
            }
            Err(e) => {
                // Id 0: the app's ids start at 1, so this fails no call but
                // is still logged on its side.
                Incoming::Reply { id: 0, outcome: Outcome::Error(HelperError::new(ErrorCode::BadRequest, e.to_string())) }
            }
        };
        let _ = tx.send(serde_json::to_string(&reply).unwrap_or_default());
    }
    shared.lock().await.gone(id).await;
    drop(tx);
    let _ = writer.await;
}
