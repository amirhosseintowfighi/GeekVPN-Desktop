//! The TUN engine: sing-box on Linux/macOS, hev-socks5-tunnel + Wintun on Windows.
//!
//! On Windows the helper creates the "GeekVPN" Wintun adapter itself (visible
//! in ncpa.cpl like WireGuard/OpenVPN) and feeds it into geekcore's SOCKS
//! via hev. On other OSes sing-box owns the TUN device as before.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use geek_ipc::{ErrorCode, HelperError, TunInfo};
use tokio::sync::{mpsc, oneshot};

use crate::paths::{self, TUN_NAME};

/// A running TUN engine. Dropping it without `stop` still kills the process.
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

// ── Unix / macOS: sing-box ──────────────────────────────────────────────
#[cfg(not(windows))]
mod unix {
    use super::*;
    use std::net::TcpListener;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    const TAIL: usize = 40;
    const START_TIMEOUT: Duration = Duration::from_secs(15);

    use geek_config::{tun_config, TunHost, TunSpec};
    use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
    use tokio::process::{Child, Command};

    pub async fn start(
        spec: &TunSpec,
        state_dir: &Path,
        generation: u64,
        exited: mpsc::UnboundedSender<(u64, String)>,
    ) -> Result<Running, HelperError> {
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

    async fn wait_until_up(child: &mut Child, port: u16, tail: &Arc<Mutex<VecDeque<String>>>) -> Result<(), HelperError> {
        let deadline = Instant::now() + START_TIMEOUT;
        let failed = |tail: &Arc<Mutex<VecDeque<String>>>| HelperError::new(ErrorCode::EngineFailed, last_lines(tail));
        loop {
            if let Ok(Some(_)) = child.try_wait() {
                tokio::time::sleep(Duration::from_millis(100)).await;
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
        let _ = getrandom::fill(&mut b);
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

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

    pub fn version() -> String {
        std::process::Command::new(paths::engine_binary())
            .arg("version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).lines().next().and_then(|l| l.split_whitespace().last().map(String::from)))
            .unwrap_or_default()
    }

    pub fn kill_stale(pid: u32) {
        #[cfg(unix)]
        {
            let comm = std::process::Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "comm="])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_default();
            if !comm.is_empty() && paths::ENGINE_FILE.starts_with(comm.rsplit('/').next().unwrap_or("")) {
                let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).status();
            }
        }
        #[cfg(windows)]
        {
            let _ = pid;
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn strips_colours() {
            assert_eq!(super::strip_ansi("\u{1b}[31mFATAL\u{1b}[0m[0000] start"), "FATAL[0000] start");
        }
    }
}

#[cfg(not(windows))]
pub use unix::{kill_stale, start, version};

// ── Windows: hev-socks5-tunnel + Wintun ─────────────────────────────────
#[cfg(windows)]
mod win {
    use super::*;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    use geek_config::TunSpec;
    use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
    use tokio::process::{Child, Command};
    use windows_sys::Win32::NetworkManagement::IpHelper::ConvertInterfaceAliasToLuid;
    use windows_sys::Win32::NetworkManagement::Ndis::NET_LUID_LH;

    const TAIL: usize = 60;
    const START_TIMEOUT: Duration = Duration::from_secs(15);
    // Fixed GUID so the adapter is always "GeekVPN" and WFP filters know the LUID.
    const HEV_GUID: &str = "5f1c2b8e-6a0d-4c77-9e3a-1b7c0d2e4f00";

    pub async fn start(
        spec: &TunSpec,
        state_dir: &Path,
        generation: u64,
        exited: mpsc::UnboundedSender<(u64, String)>,
    ) -> Result<Running, HelperError> {
        let bin = paths::hev_binary();
        if !bin.exists() {
            return Err(HelperError::new(ErrorCode::NoEngine, format!("{} is missing", bin.display())));
        }
        // wintun.dll + msys-2.0.dll must sit next to the hev exe (or helper exe) for the loader.
        let wintun = paths::wintun_dll();
        if !wintun.exists() {
            eprintln!("geekvpn-helper: wintun.dll missing at {}, will rely on system search", wintun.display());
        }
        let msys = paths::msys_dll();
        if !msys.exists() {
            eprintln!("geekvpn-helper: msys-2.0.dll missing at {}, hev will fail to start", msys.display());
        }
        std::fs::create_dir_all(state_dir).map_err(|e| HelperError::new(ErrorCode::Internal, format!("{}: {e}", state_dir.display())))?;

        let yml_path = state_dir.join("hev.yml");
        let yml = hev_yaml(spec);
        std::fs::write(&yml_path, yml.as_bytes())
            .map_err(|e| HelperError::new(ErrorCode::Internal, format!("writing {}: {e}", yml_path.display())))?;

        let mut cmd = Command::new(&bin);
        cmd.arg(&yml_path);
        cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        let mut child = cmd.spawn().map_err(|e| HelperError::new(ErrorCode::NoEngine, format!("{}: {e}", bin.display())))?;
        let tail: Arc<Mutex<VecDeque<String>>> = Arc::default();
        if let Some(out) = child.stdout.take() {
            keep_tail(out, tail.clone());
        }
        if let Some(err) = child.stderr.take() {
            keep_tail(err, tail.clone());
        }

        // Wait for Wintun adapter "GeekVPN" to appear (hev creates it).
        wait_for_adapter(&mut child, &tail).await?;
        // Bypass for the server IP before the default routes: otherwise
        // hev's own TCP to the server loops back through the TUN. WFP's App
        // filter for geekcore covers most cases, but a concrete /32 route via
        // the real gateway is robust across firewalls/AV interop.
        for ip in &spec.bypass {
            if let Err(e) = add_bypass(ip) {
                eprintln!("geekvpn-helper: add_bypass {ip} warning: {e}");
            }
        }
        // Publish routes that send every other packet through GeekVPN.
        if let Err(e) = add_routes() {
            eprintln!("geekvpn-helper: add_routes warning: {e}");
            // Not fatal: the TUN still carries packets if WFP is correct, but log it.
        }

        let pid = child.id();
        let yml_path2 = yml_path.clone();
        let bypass_for_stop = spec.bypass.clone();
        let bypass_for_exit = spec.bypass.clone();
        let (stop_tx, stop_rx) = oneshot::channel::<oneshot::Sender<()>>();
        tokio::spawn(async move {
            tokio::select! {
                done = stop_rx => {
                    for ip in &bypass_for_stop { remove_bypass(ip); }
                    let _ = remove_routes();
                    let _ = child.kill().await;
                    // Give hev a moment to close the adapter handle.
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    let _ = std::fs::remove_file(&yml_path2);
                    if let Ok(done) = done {
                        let _ = done.send(());
                    }
                }
                status = child.wait() => {
                    for ip in &bypass_for_exit { remove_bypass(ip); }
                    let _ = remove_routes();
                    let why = format!("hev-socks5-tunnel exited ({}): {}", status.map(|s| s.to_string()).unwrap_or_default(), last_lines(&tail));
                    let _ = exited.send((generation, why));
                }
            }
        });

        Ok(Running {
            info: TunInfo { interface: TUN_NAME.to_string(), clash_port: 0, clash_secret: String::new() },
            pid,
            generation,
            stop: Some(stop_tx),
        })
    }

    fn hev_yaml(spec: &TunSpec) -> String {
        // Single quotes escaped by doubling? YAML single-quoted style: ''.
        let esc = |s: &str| s.replace('\'', "''");
        format!(
            "tunnel:\n  name: {name}\n  mtu: 1500\n  ipv4: 198.18.0.1\n  guid: {guid}\n  multi-queue: false\nsocks5:\n  address: 127.0.0.1\n  port: {port}\n  username: '{user}'\n  password: '{pass}'\n  udp: 'tcp'\nmisc:\n  log-level: warn\n",
            name = TUN_NAME,
            guid = HEV_GUID,
            port = spec.upstream.port,
            user = esc(&spec.upstream.username),
            pass = esc(&spec.upstream.password),
        )
    }

    async fn wait_for_adapter(child: &mut Child, tail: &Arc<Mutex<VecDeque<String>>>) -> Result<(), HelperError> {
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            if let Ok(Some(_)) = child.try_wait() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                return Err(HelperError::new(ErrorCode::EngineFailed, last_lines(tail)));
            }
            if adapter_exists() {
                // Give the IP stack a moment to bind 198.18.0.1.
                tokio::time::sleep(Duration::from_millis(500)).await;
                if let Ok(Some(_)) = child.try_wait() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    return Err(HelperError::new(ErrorCode::EngineFailed, last_lines(tail)));
                }
                return Ok(());
            }
            if Instant::now() > deadline {
                let _ = child.kill().await;
                return Err(HelperError::new(
                    ErrorCode::EngineFailed,
                    format!("GeekVPN adapter not up after {START_TIMEOUT:?}: {}", last_lines(tail)),
                ));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    fn adapter_exists() -> bool {
        let name: Vec<u16> = TUN_NAME.encode_utf16().chain(Some(0)).collect();
        let mut luid: NET_LUID_LH = unsafe { std::mem::zeroed() };
        let rc = unsafe { ConvertInterfaceAliasToLuid(name.as_ptr(), &mut luid) };
        rc == 0
    }

    fn default_v4_route() -> Option<(String, String)> {
        use std::os::windows::process::CommandExt;
        let script = "$r=Get-NetRoute -DestinationPrefix 0.0.0.0/0 -ErrorAction SilentlyContinue | Select-Object -First 1; if($r){\"{0}|{1}\" -f $r.InterfaceAlias, $r.NextHop}";
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(0x0800_0000)
            .output().ok()?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let (iface, gw) = s.split_once('|')?;
        let iface = iface.trim();
        let gw = gw.trim();
        if iface.is_empty() { None } else { Some((iface.to_string(), gw.to_string())) }
    }

    fn default_v6_route() -> Option<(String, String)> {
        use std::os::windows::process::CommandExt;
        let script = "$r=Get-NetRoute -DestinationPrefix ::/0 -ErrorAction SilentlyContinue | Select-Object -First 1; if($r){\"{0}|{1}\" -f $r.InterfaceAlias, $r.NextHop}";
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(0x0800_0000)
            .output().ok()?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let (iface, gw) = s.split_once('|')?;
        let iface = iface.trim();
        let gw = gw.trim();
        if iface.is_empty() { None } else { Some((iface.to_string(), gw.to_string())) }
    }

    fn add_bypass(ip: &str) -> Result<(), String> {
        let ip = ip.trim();
        if ip.is_empty() { return Ok(()); }
        let addr: std::net::IpAddr = ip.parse().map_err(|_| format!("bad ip {ip}"))?;
        if addr.to_string() == "198.18.0.1" || addr.to_string() == "198.18.0.2" { return Ok(()); }
        match addr {
            std::net::IpAddr::V4(v4) => {
                let prefix = format!("{v4}/32");
                if let Some((iface, gw)) = default_v4_route() {
                    // netsh needs interface quoted if it contains spaces (e.g. "Wi-Fi 2").
                    let iface_arg = format!("interface=\"{iface}\"");
                    // 0.0.0.0 means on-link; use discovered gateway, fallback to 0.0.0.0 if empty.
                    let gw = if gw.is_empty() || gw == "0.0.0.0" { "0.0.0.0".to_string() } else { gw };
                    run_hidden("netsh", &["interface", "ipv4", "add", "route", &format!("prefix={prefix}"), &iface_arg, &format!("nexthop={gw}"), "metric=0", "store=active"])?;
                }
                Ok(())
            }
            std::net::IpAddr::V6(v6) => {
                let prefix = format!("{v6}/128");
                if let Some((iface, gw)) = default_v6_route() {
                    let iface_arg = format!("interface=\"{iface}\"");
                    let gw = if gw.is_empty() { "::".to_string() } else { gw };
                    run_hidden("netsh", &["interface", "ipv6", "add", "route", &format!("prefix={prefix}"), &iface_arg, &format!("nexthop={gw}"), "metric=0", "store=active"])?;
                }
                Ok(())
            }
        }
    }

    fn remove_bypass(ip: &str) {
        let ip = ip.trim();
        if ip.is_empty() { return; }
        let Ok(addr): Result<std::net::IpAddr, _> = ip.parse() else { return };
        match addr {
            std::net::IpAddr::V4(v4) => {
                let prefix = format!("{v4}/32");
                if let Some((iface, _)) = default_v4_route() {
                    let iface_arg = format!("interface=\"{iface}\"");
                    let _ = run_hidden("netsh", &["interface", "ipv4", "delete", "route", &format!("prefix={prefix}"), &iface_arg]);
                }
                // Also try without iface (best effort).
                let _ = run_hidden("netsh", &["interface", "ipv4", "delete", "route", &format!("prefix={prefix}"), "interface=GeekVPN"]);
            }
            std::net::IpAddr::V6(v6) => {
                let prefix = format!("{v6}/128");
                if let Some((iface, _)) = default_v6_route() {
                    let iface_arg = format!("interface=\"{iface}\"");
                    let _ = run_hidden("netsh", &["interface", "ipv6", "delete", "route", &format!("prefix={prefix}"), &iface_arg]);
                }
                let _ = run_hidden("netsh", &["interface", "ipv6", "delete", "route", &format!("prefix={prefix}"), "interface=GeekVPN"]);
            }
        }
    }

    fn add_routes() -> Result<(), String> {
        // 0.0.0.0/1 + 128.0.0.0/1 cover 0.0.0.0/0 but do not replace the default route entry itself,
        // so removing them restores the system (no table rebuild). Metric 0 = highest priority.
        for prefix in ["0.0.0.0/1", "128.0.0.0/1"] {
            run_hidden("netsh", &["interface", "ipv4", "add", "route", &format!("prefix={prefix}"), "interface=GeekVPN", "nexthop=0.0.0.0", "metric=0", "store=active"])?;
        }
        Ok(())
    }

    fn remove_routes() -> Result<(), String> {
        for prefix in ["0.0.0.0/1", "128.0.0.0/1"] {
            let _ = run_hidden("netsh", &["interface", "ipv4", "delete", "route", &format!("prefix={prefix}"), "interface=GeekVPN"]);
        }
        Ok(())
    }

    fn run_hidden(cmd: &str, args: &[&str]) -> Result<(), String> {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new(cmd)
            .args(args)
            .creation_flags(0x0800_0000)
            .output()
            .map_err(|e| format!("{cmd}: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!("{cmd} {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
        }
    }

    fn keep_tail(r: impl AsyncRead + Unpin + Send + 'static, tail: Arc<Mutex<VecDeque<String>>>) {
        tokio::spawn(async move {
            let mut lines = BufReader::new(r).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let mut t = tail.lock().unwrap();
                if t.len() == TAIL {
                    t.pop_front();
                }
                t.push_back(l);
            }
        });
    }

    fn last_lines(tail: &Arc<Mutex<VecDeque<String>>>) -> String {
        let t = tail.lock().unwrap();
        t.iter().rev().take(5).rev().cloned().collect::<Vec<_>>().join("\n")
    }

    pub fn version() -> String {
        // hev has no --version flag; report the pinned release we ship.
        // Try running it without args to get a hint, otherwise fixed string.
        if let Ok(out) = std::process::Command::new(paths::hev_binary()).arg("--help").output() {
            let txt = String::from_utf8_lossy(&out.stdout);
            if txt.contains("hev-socks5-tunnel") {
                return "hev-socks5-tunnel 2.18.0 (wintun)".into();
            }
        }
        "hev-socks5-tunnel 2.18.0 (wintun)".into()
    }

    pub fn kill_stale(_pid: u32) {
        use std::os::windows::process::CommandExt;
        // Any orphaned hev left by a crashed helper: kill by image name.
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", paths::HEV_FILE])
            .creation_flags(0x0800_0000)
            .status();
        // wintun adapter itself is left behind intentionally (reused next start);
        // routes are active-store only so they are gone after reboot.
        let _ = remove_routes();
    }
}

#[cfg(windows)]
pub use win::{kill_stale, start, version};
