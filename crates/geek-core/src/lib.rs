//! geekcore as a child process: spawn it, call it, and notice when it dies.
//!
//! The UI never waits on the core directly. Every call has a timeout, and a
//! crash fails the calls in flight at once with the last lines the core
//! wrote, so the app can say what happened instead of hanging (§5 of the
//! brief: the UI must survive the core).

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{broadcast, oneshot, Mutex as AsyncMutex};

/// Lines of the core's stderr kept for error messages and problem reports.
const STDERR_TAIL: usize = 60;

#[derive(Debug, Clone, thiserror::Error)]
pub enum CoreError {
    #[error("geekcore could not start: {0}")]
    Spawn(String),
    /// The process is gone. `tail` is what it last wrote to stderr.
    #[error("geekcore exited: {tail}")]
    Exited { tail: String },
    #[error("geekcore did not answer {0} in time")]
    Timeout(String),
    /// The core ran the request and refused it (a bad config, a port in use).
    #[error("{0}")]
    Refused(String),
}

/// An unsolicited line from the core: delay-test results, scanner progress.
#[derive(Debug, Clone, Deserialize)]
pub struct CoreEvent {
    pub event: String,
    pub data: Value,
}

type Pending = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, CoreError>>>>>;

pub struct CoreProcess {
    child: AsyncMutex<Child>,
    stdin: AsyncMutex<ChildStdin>,
    pending: Pending,
    next_id: AtomicI64,
    events: broadcast::Sender<CoreEvent>,
    tail: Arc<Mutex<VecDeque<String>>>,
    alive: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Deserialize)]
struct Reply {
    id: Option<i64>,
    result: Option<Value>,
    error: Option<String>,
    event: Option<String>,
    data: Option<Value>,
}

impl CoreProcess {
    /// Starts `binary`. `geo_dir` holds geoip.dat and geosite.dat, which
    /// Xray finds through XRAY_LOCATION_ASSET.
    pub fn spawn(binary: &Path, geo_dir: &Path) -> Result<Self, CoreError> {
        let mut cmd = Command::new(binary);
        cmd.env("XRAY_LOCATION_ASSET", geo_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // If the app dies, so does its core: a tunnel nobody controls is
            // worse than none.
            .kill_on_drop(true);
        #[cfg(windows)]
        {
            // No console window flashing up beside the app.
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd.spawn().map_err(|e| CoreError::Spawn(e.to_string()))?;
        let stdin = child.stdin.take().ok_or_else(|| CoreError::Spawn("no stdin".into()))?;
        let stdout = child.stdout.take().ok_or_else(|| CoreError::Spawn("no stdout".into()))?;
        let stderr = child.stderr.take().ok_or_else(|| CoreError::Spawn("no stderr".into()))?;

        let pending: Pending = Arc::default();
        // A scan reports every address it tries; the app reads them as they come,
        // and a slow reader skips ahead rather than losing the finish.
        let (events, _) = broadcast::channel(1024);
        let tail: Arc<Mutex<VecDeque<String>>> = Arc::default();
        let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));

        {
            let tail = tail.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut t = tail.lock().expect("tail lock");
                    if t.len() == STDERR_TAIL {
                        t.pop_front();
                    }
                    t.push_back(line);
                }
            });
        }
        {
            let pending = pending.clone();
            let events = events.clone();
            let tail = tail.clone();
            let alive = alive.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let Ok(reply) = serde_json::from_str::<Reply>(&line) else { continue };
                    if let (Some(name), Some(data)) = (reply.event, reply.data.clone()) {
                        let _ = events.send(CoreEvent { event: name, data });
                        continue;
                    }
                    let Some(id) = reply.id else { continue };
                    let Some(tx) = pending.lock().expect("pending lock").remove(&id) else { continue };
                    let _ = tx.send(match reply.error {
                        Some(e) => Err(CoreError::Refused(e)),
                        None => Ok(reply.result.unwrap_or(Value::Null)),
                    });
                }
                // stdout closed: the process is gone. Give stderr a moment to
                // drain so the message carries the panic, not half of it.
                alive.store(false, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(100)).await;
                let text = tail_text(&tail);
                for (_, tx) in pending.lock().expect("pending lock").drain() {
                    let _ = tx.send(Err(CoreError::Exited { tail: text.clone() }));
                }
                let _ = events.send(CoreEvent { event: "core.exited".into(), data: json!({ "tail": text }) });
            });
        }

        Ok(Self {
            child: AsyncMutex::new(child),
            stdin: AsyncMutex::new(stdin),
            pending,
            next_id: AtomicI64::new(1),
            events,
            tail,
            alive,
        })
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    pub fn events(&self) -> broadcast::Receiver<CoreEvent> {
        self.events.subscribe()
    }

    /// The core's recent log, for errors and problem reports.
    pub fn log_tail(&self) -> String {
        tail_text(&self.tail)
    }

    pub async fn call(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, CoreError> {
        if !self.is_alive() {
            return Err(CoreError::Exited { tail: self.log_tail() });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().expect("pending lock").insert(id, tx);
        let mut line = serde_json::to_vec(&json!({ "id": id, "method": method, "params": params }))
            .map_err(|e| CoreError::Refused(e.to_string()))?;
        line.push(b'\n');
        {
            let mut stdin = self.stdin.lock().await;
            if stdin.write_all(&line).await.is_err() || stdin.flush().await.is_err() {
                self.pending.lock().expect("pending lock").remove(&id);
                return Err(CoreError::Exited { tail: self.log_tail() });
            }
        }
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CoreError::Exited { tail: self.log_tail() }),
            Err(_) => {
                self.pending.lock().expect("pending lock").remove(&id);
                Err(CoreError::Timeout(method.into()))
            }
        }
    }

    pub async fn kill(&self) {
        let _ = self.child.lock().await.kill().await;
    }
}

fn tail_text(tail: &Mutex<VecDeque<String>>) -> String {
    tail.lock().map(|t| t.iter().cloned().collect::<Vec<_>>().join("\n")).unwrap_or_default()
}
