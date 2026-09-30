//! sing-box, supervised: started from a config the helper writes itself,
//! declared up only once its API answers, and reported the moment it dies.

use std::collections::VecDeque;
use std::net::TcpListener;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use geek_config::{tun_config, TunHost, TunSpec};
use geek_ipc::{ErrorCode, HelperError, TunInfo};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};

use crate::paths::{self, TUN_NAME};

const TAIL: usize = 40;
const START_TIMEOUT: Duration = Duration::from_secs(15);

/// A running sing-box. Dropping it without `stop` still kills the process.
pub struct Running {
    pub info: TunInfo,
    pub pid: Option<u32>,
    pub generation: u64,
    stop: Option<oneshot::Sender<oneshot::Sender<()>>>,
}

impl Running {
    pub async fn stop(mut self) {
        if let Some(tx) = self.stop.take() {
            let (done, wait) = oneshot::channel();
            if tx.send(done).is_ok() {
                let _ = wait.await;
            }
        }
    }
}

/// `exited` hears (generation, last log lines) if sing-box dies on its own.
pub async fn start(spec: &TunSpec, state_dir: &Path, generation: u64, exited: mpsc::UnboundedSender<(u64, String)>) -> Result<Running, HelperError> {
    let bin = paths::engine_binary();
    if !bin.exists() {
        return Err(HelperError::new(ErrorCode::NoEngine, format!("{} is missing", bin.display())));
    }
    let clash_port = free_port().map_err(|e| HelperError::new(ErrorCode::Internal, format!("no free port: {e}")))?;
    let host = TunHost {
        interface: (!cfg!(target_os = "macos")).then(|| TUN_NAME.to_string()),
        ipv6: ipv6_available(),
        clash_port,
        clash_secret: secret(),
        mark: cfg!(target_os = "linux").then_some(geek_config::LINUX_MARK),
    };
    let config = tun_config(spec, &host);
    let config_path = state_dir.join("engine.json");
    write_private(&config_path, &serde_json::to_vec_pretty(&config).unwrap_or_default())
        .map_err(|e| HelperError::new(ErrorCode::Internal, format!("writing {}: {e}", config_path.display())))?;

    let before = interfaces();
    let mut cmd = Command::new(&bin);
    cmd.arg("run").arg("-c").arg(&config_path).arg("-D").arg(state_dir);
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = cmd.spawn().map_err(|e| HelperError::new(ErrorCode::NoEngine, format!("{}: {e}", bin.display())))?;
    let tail: Arc<Mutex<VecDeque<String>>> = Arc::default();
    if let Some(out) = child.stdout.take() {
        keep_tail(out, tail.clone());
    }
    if let Some(err) = child.stderr.take() {
        keep_tail(err, tail.clone());
    }

    wait_until_up(&mut child, clash_port, &tail).await?;
    let interface = if cfg!(target_os = "macos") { new_interface(&before).unwrap_or_default() } else { TUN_NAME.to_string() };

    let pid = child.id();
    let (stop_tx, stop_rx) = oneshot::channel::<oneshot::Sender<()>>();
    tokio::spawn(async move {
        tokio::select! {
            done = stop_rx => {
                let _ = child.kill().await;
                if let Ok(done) = done {
                    let _ = done.send(());
                }
            }
            status = child.wait() => {
                let why = format!("sing-box exited ({}): {}", status.map(|s| s.to_string()).unwrap_or_default(), last_lines(&tail));
                let _ = exited.send((generation, why));
            }
        }
    });
    Ok(Running {
        info: TunInfo { interface, clash_port, clash_secret: host.clash_secret },
        pid,
        generation,
        stop: Some(stop_tx),
    })
}

/// Up means the API answers and the process is still there a moment later:
/// sing-box opens its API before the TUN's routes are in place, and a
/// failure there (no IPv6, a rule it cannot add) ends it just after.
async fn wait_until_up(child: &mut Child, port: u16, tail: &Arc<Mutex<VecDeque<String>>>) -> Result<(), HelperError> {
    let deadline = Instant::now() + START_TIMEOUT;
    let failed = |tail: &Arc<Mutex<VecDeque<String>>>| HelperError::new(ErrorCode::EngineFailed, last_lines(tail));
    loop {
        if let Ok(Some(_)) = child.try_wait() {
            tokio::time::sleep(Duration::from_millis(100)).await; // let the tail readers finish
            return Err(failed(tail));
        }
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill().await;
            return Err(HelperError::new(ErrorCode::EngineFailed, format!("not up after {START_TIMEOUT:?}: {}", last_lines(tail))));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    if let Ok(Some(_)) = child.try_wait() {
        tokio::time::sleep(Duration::from_millis(100)).await;
        return Err(failed(tail));
    }
    Ok(())
}

fn keep_tail(r: impl AsyncRead + Unpin + Send + 'static, tail: Arc<Mutex<VecDeque<String>>>) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(r).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            let mut t = tail.lock().unwrap();
            if t.len() == TAIL {
                t.pop_front();
            }
            t.push_back(strip_ansi(&l));
        }
    });
}

fn last_lines(tail: &Arc<Mutex<VecDeque<String>>>) -> String {
    let t = tail.lock().unwrap();
    t.iter().rev().take(5).rev().cloned().collect::<Vec<_>>().join("\n")
}

/// sing-box colours its log even into a pipe.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn free_port() -> std::io::Result<u16> {
    Ok(TcpListener::bind(("127.0.0.1", 0))?.local_addr()?.port())
}

fn secret() -> String {
    let mut b = [0u8; 16];
    // A failed RNG leaves zeros; the API is on loopback either way, the
    // secret only keeps other local users out of the connection list.
    let _ = getrandom::fill(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A kernel built without IPv6 makes sing-box fail on any IPv6 address or
/// policy rule.
fn ipv6_available() -> bool {
    if cfg!(target_os = "linux") {
        Path::new("/proc/net/if_inet6").exists()
    } else {
        true
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// macOS names the device itself (`utunN`); the new one is ours.
fn interfaces() -> Vec<String> {
    if !cfg!(target_os = "macos") {
        return vec![];
    }
    std::process::Command::new("ifconfig")
        .arg("-l")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).split_whitespace().map(String::from).collect())
        .unwrap_or_default()
}

fn new_interface(before: &[String]) -> Option<String> {
    interfaces().into_iter().find(|i| i.starts_with("utun") && !before.contains(i))
}

/// "1.14.2", from `sing-box version`.
pub fn version() -> String {
    std::process::Command::new(paths::engine_binary())
        .arg("version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).lines().next().and_then(|l| l.split_whitespace().last().map(String::from)))
        .unwrap_or_default()
}

/// A sing-box the helper left behind when it died, if that pid still is one.
pub fn kill_stale(pid: u32) {
    #[cfg(unix)]
    {
        let comm = std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        // `comm` may be cut at 15 characters.
        if !comm.is_empty() && paths::ENGINE_FILE.starts_with(comm.rsplit('/').next().unwrap_or("")) {
            let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).status();
        }
    }
    #[cfg(windows)]
    {
        let _ = pid;
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", paths::ENGINE_FILE])
            .creation_flags(0x0800_0000)
            .status();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn strips_colours() {
        assert_eq!(super::strip_ansi("\u{1b}[31mFATAL\u{1b}[0m[0000] start"), "FATAL[0000] start");
    }
}
