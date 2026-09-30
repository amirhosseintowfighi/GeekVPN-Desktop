//! The app's side: one connection, requests matched to replies by id,
//! events broadcast. Every call has a timeout.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, WriteHalf};
use tokio::sync::{broadcast, oneshot, Mutex as AsyncMutex};

use crate::transport::{connect, Stream};
use crate::{Envelope, Event, HelperError, Incoming, Outcome, Request};

#[derive(Debug, Clone, thiserror::Error)]
pub enum IpcError {
    /// Nothing listens: the helper is not installed, or not running.
    #[error("helper not running: {0}")]
    NotRunning(String),
    /// The connection dropped (the helper stopped or crashed).
    #[error("helper connection closed")]
    Closed,
    #[error("helper did not answer in time")]
    Timeout,
    #[error("{0}")]
    Helper(HelperError),
    #[error("unexpected reply: {0}")]
    Protocol(String),
}

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Outcome>>>>;

pub struct HelperClient {
    writer: AsyncMutex<WriteHalf<Box<dyn Stream>>>,
    pending: Pending,
    next_id: AtomicU64,
    events: broadcast::Sender<Event>,
    alive: Arc<AtomicBool>,
}

impl HelperClient {
    pub async fn connect(endpoint: &str) -> Result<Self, IpcError> {
        let stream = connect(endpoint).await.map_err(|e: io::Error| IpcError::NotRunning(e.to_string()))?;
        let (read, writer) = tokio::io::split(stream);
        let pending: Pending = Arc::default();
        let (events, _) = broadcast::channel(32);
        let alive = Arc::new(AtomicBool::new(true));
        {
            let (pending, events, alive) = (pending.clone(), events.clone(), alive.clone());
            tokio::spawn(async move {
                let mut lines = BufReader::new(read).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    match serde_json::from_str::<Incoming>(&line) {
                        Ok(Incoming::Reply { id, outcome }) => {
                            if let Some(tx) = pending.lock().unwrap().remove(&id) {
                                let _ = tx.send(outcome);
                            }
                        }
                        Ok(Incoming::Event(e)) => {
                            let _ = events.send(e);
                        }
                        Err(_) => {}
                    }
                }
                alive.store(false, Ordering::SeqCst);
                // Dropping the senders fails every call in flight with Closed.
                pending.lock().unwrap().clear();
            });
        }
        Ok(Self { writer: AsyncMutex::new(writer), pending, next_id: AtomicU64::new(1), events, alive })
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    pub async fn call<T: DeserializeOwned>(&self, request: Request, timeout: Duration) -> Result<T, IpcError> {
        if !self.is_alive() {
            return Err(IpcError::Closed);
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let mut line = serde_json::to_vec(&Envelope { id, request }).map_err(|e| IpcError::Protocol(e.to_string()))?;
        line.push(b'\n');
        {
            let mut w = self.writer.lock().await;
            if w.write_all(&line).await.is_err() || w.flush().await.is_err() {
                self.pending.lock().unwrap().remove(&id);
                return Err(IpcError::Closed);
            }
        }
        let outcome = match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(o)) => o,
            Ok(Err(_)) => return Err(IpcError::Closed),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                return Err(IpcError::Timeout);
            }
        };
        match outcome {
            Outcome::Ok(v) => serde_json::from_value(v).map_err(|e| IpcError::Protocol(e.to_string())),
            Outcome::Error(e) => Err(IpcError::Helper(e)),
        }
    }
}
