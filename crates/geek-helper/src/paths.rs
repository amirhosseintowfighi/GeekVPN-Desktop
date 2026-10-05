//! Where the helper lives and keeps its state. Every directory here is
//! writable only by root / SYSTEM: a service must never run, or read its
//! config from, a place an ordinary user can change.

use std::path::PathBuf;

/// The TUN device's name where the OS lets us choose one.
pub const TUN_NAME: &str = if cfg!(windows) { "GeekVPN" } else { "geekvpn0" };

pub const ENGINE_FILE: &str = if cfg!(windows) { "geekvpn-sing-box.exe" } else { "geekvpn-sing-box" };
pub const HELPER_FILE: &str = if cfg!(windows) { "geekvpn-helper.exe" } else { "geekvpn-helper" };
/// Windows-only: hev-socks5-tunnel + Wintun replace sing-box as the TUN engine.
pub const HEV_FILE: &str = "hev-socks5-tunnel.exe";
pub const WINTUN_FILE: &str = "wintun.dll";
pub const MSYS_FILE: &str = "msys-2.0.dll";

/// sing-box sits next to the helper, in the install directory.
/// On Windows the TUN engine is hev (see `hev_binary`); sing-box is not used.
#[cfg(windows)]
pub fn engine_binary() -> PathBuf {
    hev_binary()
}
#[cfg(not(windows))]
pub fn engine_binary() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(ENGINE_FILE)))
        .unwrap_or_else(|| install_dir().join(ENGINE_FILE))
}

#[cfg(windows)]
pub fn hev_binary() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(HEV_FILE)))
        .unwrap_or_else(|| install_dir().join(HEV_FILE))
}

#[cfg(windows)]
pub fn wintun_dll() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(WINTUN_FILE)))
        .unwrap_or_else(|| install_dir().join(WINTUN_FILE))
}

#[cfg(windows)]
pub fn msys_dll() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(MSYS_FILE)))
        .unwrap_or_else(|| install_dir().join(MSYS_FILE))
}

#[cfg(target_os = "linux")]
pub fn install_dir() -> PathBuf {
    PathBuf::from("/usr/local/lib/geekvpn")
}
#[cfg(target_os = "macos")]
pub fn install_dir() -> PathBuf {
    PathBuf::from("/Library/PrivilegedHelperTools/com.geekvpn.helper")
}
#[cfg(windows)]
pub fn install_dir() -> PathBuf {
    let base = std::env::var_os("ProgramFiles").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    base.join("GeekVPN Helper")
}
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub fn install_dir() -> PathBuf {
    PathBuf::from("/usr/local/lib/geekvpn")
}

#[cfg(target_os = "macos")]
pub fn state_dir() -> PathBuf {
    PathBuf::from("/Library/Application Support/GeekVPN Helper")
}
#[cfg(windows)]
pub fn state_dir() -> PathBuf {
    let base = std::env::var_os("ProgramData").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    base.join("GeekVPN").join("helper")
}
#[cfg(not(any(target_os = "macos", windows)))]
pub fn state_dir() -> PathBuf {
    PathBuf::from("/var/lib/geekvpn-helper")
}
