//! The kill switch: while engaged, nothing leaves the machine except
//! through the tunnel.
//!
//! Allowed out, on every OS: loopback, the TUN device, sing-box's own
//! connections (the tunnel's transport, and apps routed direct), DHCP, and
//! the local network if the user allows it. Everything else is dropped, so
//! if sing-box or the core dies the internet stops instead of leaking.
//!
//! How sing-box's own traffic is recognised differs:
//! - Linux: nftables, by the firewall mark sing-box sets on its sockets.
//! - Windows: WFP, by sing-box's executable (ALE app id).
//! - macOS: pf cannot match a process; it passes root's sockets, which is
//!   sing-box and system daemons, never a user's apps.

use std::path::Path;

use geek_ipc::KillSwitch;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(target_os = "macos")]
use macos as imp;
#[cfg(windows)]
use windows as imp;

/// IPv4 and IPv6 ranges of the local network, for «اجازه به شبکه‌ی محلی».
pub const LAN_V4: [&str; 5] = ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", "169.254.0.0/16", "224.0.0.0/4"];
pub const LAN_V6: [&str; 3] = ["fe80::/10", "fc00::/7", "ff00::/8"];

#[derive(Default)]
pub struct Firewall {
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    inner: imp::Firewall,
}

impl Firewall {
    /// Engages, or re-engages with new settings, atomically: there is no
    /// moment between the old rules and the new ones with nothing in place.
    /// `tun` is the device once it exists (macOS and Windows need its name
    /// to let its packets through; before that, nothing but the rest).
    /// `extra_apps` are other executables that must bypass the kill switch
    /// (Windows: geekcore paths so the tunnel's own transport goes direct).
    pub fn engage(&mut self, ks: &KillSwitch, tun: Option<&str>, engine: &Path) -> Result<(), String> {
        self.engage_with_apps(ks, tun, engine, &[])
    }

    pub fn engage_with_apps(&mut self, ks: &KillSwitch, tun: Option<&str>, engine: &Path, extra_apps: &[String]) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "macos", windows))]
        return self.inner.engage_with_apps(ks, tun, engine, extra_apps);
        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        {
            let _ = (ks, tun, engine, extra_apps);
            Err("no kill switch on this system".into())
        }
    }

    pub fn release(&mut self) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "macos", windows))]
        return self.inner.release();
        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        Ok(())
    }
}
