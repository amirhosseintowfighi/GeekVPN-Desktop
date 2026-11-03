//! geekvpn-helper: the only part of GeekVPN that runs as root / SYSTEM.
//!
//! It owns what needs those rights and nothing else: sing-box's TUN device
//! and the kill-switch firewall. The app talks to it over a local socket
//! (see `geek-ipc`); it never runs code or reads files on a caller's say-so.
//!
//!     geekvpn-helper run          the service itself (systemd, launchd, SCM)
//!     geekvpn-helper install [--log <file>]
//!                                 copy to a root-owned place, register, start
//!                                 (with --log, the outcome is also written
//!                                 there: an elevated run has no console)
//!     geekvpn-helper uninstall    stop, unregister, remove
//!     geekvpn-helper version

mod engine;
mod firewall;
mod paths;
mod server;
mod service;
mod state;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let log = args.iter().position(|a| a == "--log").and_then(|i| args.get(i + 1)).cloned();
    let result = match args.get(1).map(String::as_str) {
        Some("run") => service::run(args.iter().any(|a| a == "--console")),
        Some("install") => service::install(),
        Some("uninstall") => service::uninstall(),
        Some("version") => {
            println!("geekvpn-helper {} (protocol {})", env!("CARGO_PKG_VERSION"), geek_ipc::PROTOCOL);
            Ok(())
        }
        _ => {
            eprintln!("usage: geekvpn-helper run | install | uninstall | version");
            return ExitCode::from(2);
        }
    };
    if let Some(log) = log {
        let line = match &result {
            Ok(()) => "ok\n".to_string(),
            Err(e) => format!("{e}\n"),
        };
        let _ = std::fs::write(log, line);
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("geekvpn-helper: {e}");
            ExitCode::FAILURE
        }
    }
}
