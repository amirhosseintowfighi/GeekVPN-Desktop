//! Running as a system service, and putting ourselves in place as one.
//!
//! `install` copies the helper and sing-box into a directory only root /
//! SYSTEM can write (`paths::install_dir`) and registers the service from
//! there, never from where the app happens to live: an AppImage mounts in
//! /tmp, and a per-user Windows install sits in the user's own profile.

use std::path::Path;

use crate::paths::{self, ENGINE_FILE, HELPER_FILE, HEV_FILE, MSYS_FILE, WINTUN_FILE};

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

pub fn run(console: bool) -> Result<(), String> {
    #[cfg(windows)]
    if !console {
        return imp::run_service();
    }
    let _ = console;
    runtime()?.block_on(crate::server::serve(shutdown_signal()))
}

pub fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        if let (Ok(mut term), Ok(mut int)) = (signal(SignalKind::terminate()), signal(SignalKind::interrupt())) {
            tokio::select! {
                _ = term.recv() => {}
                _ = int.recv() => {}
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

pub fn install() -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    return imp::install();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    Err("no service support on this system".into())
}

pub fn uninstall() -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    return imp::uninstall();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    Err("no service support on this system".into())
}

/// Copies the helper and sing-box from beside the running executable into
/// the install directory, unless we already run from there.
#[allow(dead_code)]
fn copy_binaries() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let from = exe.parent().ok_or("no executable directory")?;
    let to = paths::install_dir();
    if same_dir(from, &to) {
        return Ok(());
    }
    std::fs::create_dir_all(&to).map_err(|e| format!("{}: {e}", to.display()))?;
    let tun_files: &[&str] = if cfg!(windows) { &[HEV_FILE, WINTUN_FILE, MSYS_FILE] } else { &[ENGINE_FILE] };
    for name in std::iter::once(HELPER_FILE).chain(tun_files.iter().copied()) {
        let src = from.join(name);
        if !src.exists() {
            return Err(format!("{} is missing", src.display()));
        }
        // Copy beside, then rename: the old binary may still be mapped by a
        // process that is shutting down.
        let tmp = to.join(format!("{name}.new"));
        std::fs::copy(&src, &tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&tmp, to.join(name)).map_err(|e| format!("{}: {e}", to.join(name).display()))?;
    }
    Ok(())
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Runs a command, turning a failure into its stderr.
#[allow(dead_code)]
fn sh(cmd: &str, args: &[&str]) -> Result<(), String> {
    let out = std::process::Command::new(cmd).args(args).output().map_err(|e| format!("{cmd}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("{cmd} {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
    }
}
