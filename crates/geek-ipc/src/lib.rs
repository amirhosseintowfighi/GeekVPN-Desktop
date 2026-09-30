//! GeekVPN ↔ geekvpn-helper.
//!
//! The helper runs as root / SYSTEM and does the few things that need it:
//! the TUN device (through sing-box) and the kill-switch firewall. The app
//! never runs elevated; it asks the helper over a local socket (a named pipe
//! on Windows), one JSON object per line.
//!
//! Every request is typed. There is no "run this config" or "use this path":
//! the helper builds sing-box's config itself from a [`TunSpec`], so a
//! caller can decide routing, and nothing else, as root.
//!
//! Any local user may talk to the helper, as with Mullvad's daemon: routing
//! is not a secret, and the socket gives no way to run code or touch files.

mod client;
mod transport;

use serde::{Deserialize, Serialize};

pub use client::{HelperClient, IpcError};
pub use geek_config::TunSpec;
pub use transport::{bind, connect, Listener, Stream, ENDPOINT};

/// Bumped whenever a message changes shape. The app checks it on connect
/// and offers to reinstall the helper when the two disagree.
pub const PROTOCOL: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Request {
    Hello,
    Status,
    /// Starts (or restarts with a new spec) the TUN tunnel. With
    /// `kill_switch`, the firewall is engaged first, so nothing leaks while
    /// the device comes up.
    TunStart { spec: Box<TunSpec>, kill_switch: Option<KillSwitch> },
    /// Stops the tunnel and releases the kill switch.
    TunStop,
    /// Releases a kill switch left engaged (strict mode after a crash).
    KillSwitchRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KillSwitch {
    /// Printers, NAS and the router stay reachable.
    pub allow_lan: bool,
    /// Stay blocked after a crash or a reboot, until the app connects or
    /// the user turns it off. Otherwise a crash of the app opens it.
    pub strict: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub protocol: u32,
    pub version: String,
    pub sing_box: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunInfo {
    pub interface: String,
    /// sing-box's Clash API, for the Connections page.
    pub clash_port: u16,
    pub clash_secret: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub tun: Option<TunInfo>,
    pub kill_switch: Option<KillSwitch>,
}

/// Why the helper refused; the app turns these into Persian sentences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    /// The request did not parse, or the protocol versions differ.
    BadRequest,
    /// sing-box is missing next to the helper.
    NoEngine,
    /// sing-box refused the config or exited while starting.
    EngineFailed,
    /// The firewall rules could not be applied.
    Firewall,
    /// Something on the machine the helper did not expect.
    Internal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{code:?}: {detail}")]
pub struct HelperError {
    pub code: ErrorCode,
    /// English, for logs and problem reports.
    pub detail: String,
}

impl HelperError {
    pub fn new(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self { code, detail: detail.into() }
    }
}

/// Unsolicited news from the helper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Event {
    /// sing-box died while the tunnel was up. The kill switch, if engaged,
    /// stays engaged: that is its job.
    TunExited { detail: String },
}

/// A request on the wire.
#[derive(Debug, Serialize, Deserialize)]
pub struct Envelope {
    pub id: u64,
    #[serde(flatten)]
    pub request: Request,
}

/// Every line from the helper is one of these.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Incoming {
    Reply { id: u64, #[serde(flatten)] outcome: Outcome },
    Event(Event),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Ok(serde_json::Value),
    Error(HelperError),
}

#[cfg(test)]
mod tests;
