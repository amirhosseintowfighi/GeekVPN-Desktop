//! What GeekVPN changes in the operating system's network settings.
//!
//! Every change returns a `Snapshot` of what it replaced. The app writes the
//! snapshot to disk before applying, so a crash, a kill or a power cut still
//! leaves enough behind to put the user's settings back at the next start:
//! a system proxy pointing at a dead port is no internet at all.

use serde::{Deserialize, Serialize};

mod proxy;

pub use proxy::{apply_system_proxy, restore_system_proxy, Snapshot};

/// Where the system proxy should point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySpec {
    pub http_port: u16,
    pub socks_port: u16,
}

impl ProxySpec {
    pub const HOST: &'static str = "127.0.0.1";
}

/// Addresses that never go through the proxy: this machine and the local
/// network (router pages, printers, NAS).
pub const BYPASS: &[&str] = &[
    "localhost",
    "127.*",
    "10.*",
    "172.16.*",
    "172.17.*",
    "172.18.*",
    "172.19.*",
    "172.2*",
    "172.30.*",
    "172.31.*",
    "192.168.*",
    "*.local",
];

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    /// The desktop has no system proxy we know how to set (a bare window
    /// manager on Linux). TUN mode is the answer there.
    #[error("this desktop has no system proxy setting GeekVPN can change")]
    Unsupported,
    #[error("{0}")]
    Failed(String),
}

impl NetError {
    pub fn user_message(&self) -> String {
        match self {
            NetError::Unsupported => {
                "این محیط دسکتاپ تنظیم پروکسی سیستمی ندارد که برنامه بتواند عوضش کند. حالت TUN را انتخاب کن.".into()
            }
            NetError::Failed(e) => format!("تنظیم پروکسی سیستم انجام نشد: {e}"),
        }
    }
}
