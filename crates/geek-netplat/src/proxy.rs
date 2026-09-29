use serde::{Deserialize, Serialize};

use crate::{NetError, ProxySpec};

/// What the system proxy looked like before GeekVPN changed it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Snapshot {
    /// GNOME and relatives: the `org.gnome.system.proxy` keys, verbatim.
    Gnome { values: Vec<(String, String, String)> },
    /// KDE: kioslaverc's `[Proxy Settings]` entries, verbatim (None = unset).
    Kde { tool: String, values: Vec<(String, Option<String>)> },
    /// Windows: the Internet Settings registry values (None = absent).
    Windows { values: Vec<(String, Option<RegValue>)> },
    /// macOS: per network service, the three proxies' `networksetup` state.
    Macos { services: Vec<MacService> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegValue {
    Dword(u32),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacService {
    pub name: String,
    /// (kind, enabled, host, port) for web, secureweb and socksfirewall.
    pub proxies: Vec<(String, bool, String, String)>,
    pub bypass: Vec<String>,
}

/// Points the system proxy at GeekVPN's local ports and returns what was there.
pub fn apply_system_proxy(spec: &ProxySpec) -> Result<Snapshot, NetError> {
    imp::apply(spec)
}

/// Puts back what `apply_system_proxy` replaced.
pub fn restore_system_proxy(snapshot: &Snapshot) -> Result<(), NetError> {
    imp::restore(snapshot)
}

#[cfg(any(target_os = "linux", test))]
pub(crate) mod linux_values {
    use crate::{ProxySpec, BYPASS};

    /// The GNOME keys GeekVPN sets: schema, key, GVariant text.
    pub fn gnome(spec: &ProxySpec) -> Vec<(String, String, String)> {
        let h = format!("'{}'", ProxySpec::HOST);
        let ignore = format!("[{}]", BYPASS.iter().map(|b| format!("'{b}'")).collect::<Vec<_>>().join(", "));
        [
            ("org.gnome.system.proxy", "mode", "'manual'".to_string()),
            ("org.gnome.system.proxy", "ignore-hosts", ignore),
            ("org.gnome.system.proxy.http", "host", h.clone()),
            ("org.gnome.system.proxy.http", "port", spec.http_port.to_string()),
            ("org.gnome.system.proxy.https", "host", h.clone()),
            ("org.gnome.system.proxy.https", "port", spec.http_port.to_string()),
            ("org.gnome.system.proxy.socks", "host", h),
            ("org.gnome.system.proxy.socks", "port", spec.socks_port.to_string()),
        ]
        .into_iter()
        .map(|(s, k, v)| (s.to_string(), k.to_string(), v))
        .collect()
    }

    /// kioslaverc `[Proxy Settings]`: ProxyType 1 is "manual".
    pub fn kde(spec: &ProxySpec) -> Vec<(String, String)> {
        let http = format!("http://{} {}", ProxySpec::HOST, spec.http_port);
        vec![
            ("ProxyType".into(), "1".into()),
            ("httpProxy".into(), http.clone()),
            ("httpsProxy".into(), http),
            ("socksProxy".into(), format!("socks://{} {}", ProxySpec::HOST, spec.socks_port)),
            ("NoProxyFor".into(), BYPASS.join(",")),
        ]
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::process::Command;

    use super::{linux_values, Snapshot};
    use crate::{NetError, ProxySpec};

    fn run(cmd: &str, args: &[&str]) -> Result<String, NetError> {
        let out = Command::new(cmd).args(args).output().map_err(|e| NetError::Failed(format!("{cmd}: {e}")))?;
        if !out.status.success() {
            return Err(NetError::Failed(format!("{cmd}: {}", String::from_utf8_lossy(&out.stderr).trim())));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn desktop() -> String {
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_ascii_uppercase()
    }

    fn kde_tool() -> Option<&'static str> {
        ["kwriteconfig6", "kwriteconfig5"].into_iter().find(|t| Command::new(t).arg("--help").output().is_ok())
    }

    fn has_gnome_schema() -> bool {
        run("gsettings", &["list-keys", "org.gnome.system.proxy"]).is_ok()
    }

    pub fn apply(spec: &ProxySpec) -> Result<Snapshot, NetError> {
        if desktop().contains("KDE") {
            if let Some(tool) = kde_tool() {
                let read = tool.replace("write", "read");
                let values = linux_values::kde(spec);
                let before = values
                    .iter()
                    .map(|(k, _)| {
                        let v = run(&read, &["--file", "kioslaverc", "--group", "Proxy Settings", "--key", k]).ok();
                        (k.clone(), v.filter(|s| !s.is_empty()))
                    })
                    .collect();
                for (k, v) in &values {
                    run(tool, &["--file", "kioslaverc", "--group", "Proxy Settings", "--key", k, v])?;
                }
                kde_notify();
                return Ok(Snapshot::Kde { tool: tool.into(), values: before });
            }
        }
        if has_gnome_schema() {
            let values = linux_values::gnome(spec);
            let before = values
                .iter()
                .map(|(s, k, _)| Ok((s.clone(), k.clone(), run("gsettings", &["get", s, k])?)))
                .collect::<Result<Vec<_>, NetError>>()?;
            // Mode last: the ports must be right before anything uses them.
            for (s, k, v) in values.iter().rev() {
                run("gsettings", &["set", s, k, v])?;
            }
            return Ok(Snapshot::Gnome { values: before });
        }
        Err(NetError::Unsupported)
    }

    pub fn restore(snapshot: &Snapshot) -> Result<(), NetError> {
        match snapshot {
            Snapshot::Gnome { values } => {
                for (s, k, v) in values {
                    run("gsettings", &["set", s, k, v])?;
                }
                Ok(())
            }
            Snapshot::Kde { tool, values } => {
                for (k, v) in values {
                    match v {
                        Some(v) => run(tool, &["--file", "kioslaverc", "--group", "Proxy Settings", "--key", k, v])?,
                        None => run(tool, &["--file", "kioslaverc", "--group", "Proxy Settings", "--key", k, "--delete"])?,
                    };
                }
                kde_notify();
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// KIO re-reads kioslaverc only when told to.
    fn kde_notify() {
        let _ = Command::new("dbus-send")
            .args(["--type=signal", "/KIO/Scheduler", "org.kde.KIO.Scheduler.reparseSlaveConfiguration", "string:"])
            .status();
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;
    use windows_sys::Win32::Networking::WinInet::{
        InternetSetOptionW, INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED,
    };

    use super::{RegValue, Snapshot};
    use crate::{NetError, ProxySpec, BYPASS};

    const PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    const DWORDS: &[&str] = &["ProxyEnable"];
    const STRINGS: &[&str] = &["ProxyServer", "ProxyOverride", "AutoConfigURL"];

    fn key(write: bool) -> Result<RegKey, NetError> {
        let flags = if write { KEY_READ | KEY_WRITE } else { KEY_READ };
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(PATH, flags).map_err(|e| NetError::Failed(e.to_string()))
    }

    /// WinINet caches the settings; without this, browsers keep the old ones.
    fn notify() {
        unsafe {
            InternetSetOptionW(std::ptr::null(), INTERNET_OPTION_SETTINGS_CHANGED, std::ptr::null(), 0);
            InternetSetOptionW(std::ptr::null(), INTERNET_OPTION_REFRESH, std::ptr::null(), 0);
        }
    }

    pub fn apply(spec: &ProxySpec) -> Result<Snapshot, NetError> {
        let k = key(true)?;
        let mut before = Vec::new();
        for name in DWORDS {
            before.push((name.to_string(), k.get_value::<u32, _>(name).ok().map(RegValue::Dword)));
        }
        for name in STRINGS {
            before.push((name.to_string(), k.get_value::<String, _>(name).ok().map(RegValue::Str)));
        }
        let server = format!(
            "http={h}:{p};https={h}:{p};socks={h}:{s}",
            h = ProxySpec::HOST,
            p = spec.http_port,
            s = spec.socks_port
        );
        let bypass = format!("{};<local>", BYPASS.join(";"));
        let err = |e: std::io::Error| NetError::Failed(e.to_string());
        k.set_value("ProxyServer", &server).map_err(err)?;
        k.set_value("ProxyOverride", &bypass).map_err(err)?;
        // A PAC script would win over the manual proxy.
        let _ = k.delete_value("AutoConfigURL");
        k.set_value("ProxyEnable", &1u32).map_err(err)?;
        notify();
        Ok(Snapshot::Windows { values: before })
    }

    pub fn restore(snapshot: &Snapshot) -> Result<(), NetError> {
        let Snapshot::Windows { values } = snapshot else { return Ok(()) };
        let k = key(true)?;
        let err = |e: std::io::Error| NetError::Failed(e.to_string());
        for (name, v) in values {
            match v {
                Some(RegValue::Dword(d)) => k.set_value(name, d).map_err(err)?,
                Some(RegValue::Str(s)) => k.set_value(name, s).map_err(err)?,
                None => {
                    let _ = k.delete_value(name);
                }
            }
        }
        notify();
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::process::Command;

    use super::{MacService, Snapshot};
    use crate::{NetError, ProxySpec, BYPASS};

    const KINDS: &[&str] = &["webproxy", "securewebproxy", "socksfirewallproxy"];

    fn ns(args: &[&str]) -> Result<String, NetError> {
        let out = Command::new("/usr/sbin/networksetup").args(args).output().map_err(|e| NetError::Failed(e.to_string()))?;
        if !out.status.success() {
            return Err(NetError::Failed(String::from_utf8_lossy(&out.stderr).trim().to_string()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    /// Enabled services only; a leading `*` marks a disabled one.
    fn services() -> Result<Vec<String>, NetError> {
        Ok(ns(&["-listallnetworkservices"])?
            .lines()
            .skip(1)
            .filter(|l| !l.starts_with('*') && !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .collect())
    }

    fn read(service: &str, kind: &str) -> (bool, String, String) {
        let text = ns(&[&format!("-get{kind}"), service]).unwrap_or_default();
        let field = |k: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(k).map(|v| v.trim().to_string()))
                .unwrap_or_default()
        };
        (field("Enabled:") == "Yes", field("Server:"), field("Port:"))
    }

    pub fn apply(spec: &ProxySpec) -> Result<Snapshot, NetError> {
        let mut saved = Vec::new();
        for s in services()? {
            let proxies = KINDS
                .iter()
                .map(|k| {
                    let (on, host, port) = read(&s, k);
                    (k.to_string(), on, host, port)
                })
                .collect();
            let bypass = ns(&["-getproxybypassdomains", &s])
                .unwrap_or_default()
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with("There aren't"))
                .map(String::from)
                .collect();
            saved.push(MacService { name: s.clone(), proxies, bypass });

            let http = spec.http_port.to_string();
            let socks = spec.socks_port.to_string();
            ns(&["-setwebproxy", &s, ProxySpec::HOST, &http])?;
            ns(&["-setsecurewebproxy", &s, ProxySpec::HOST, &http])?;
            ns(&["-setsocksfirewallproxy", &s, ProxySpec::HOST, &socks])?;
            let mut args = vec!["-setproxybypassdomains", s.as_str()];
            args.extend(BYPASS.iter().copied());
            ns(&args)?;
        }
        Ok(Snapshot::Macos { services: saved })
    }

    pub fn restore(snapshot: &Snapshot) -> Result<(), NetError> {
        let Snapshot::Macos { services } = snapshot else { return Ok(()) };
        for svc in services {
            for (kind, on, host, port) in &svc.proxies {
                if *on && !host.is_empty() {
                    ns(&[&format!("-set{kind}"), &svc.name, host, port])?;
                } else {
                    ns(&[&format!("-set{kind}state"), &svc.name, "off"])?;
                }
            }
            let mut args = vec!["-setproxybypassdomains".to_string(), svc.name.clone()];
            if svc.bypass.is_empty() {
                args.push("Empty".into());
            } else {
                args.extend(svc.bypass.iter().cloned());
            }
            ns(&args.iter().map(String::as_str).collect::<Vec<_>>())?;
        }
        Ok(())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod imp {
    use super::Snapshot;
    use crate::{NetError, ProxySpec};
    pub fn apply(_: &ProxySpec) -> Result<Snapshot, NetError> {
        Err(NetError::Unsupported)
    }
    pub fn restore(_: &Snapshot) -> Result<(), NetError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnome_sets_every_scheme_to_the_local_ports() {
        let v = linux_values::gnome(&ProxySpec { http_port: 10809, socks_port: 10808 });
        let get = |s: &str, k: &str| v.iter().find(|(a, b, _)| a == s && b == k).map(|x| x.2.clone()).unwrap();
        assert_eq!(get("org.gnome.system.proxy", "mode"), "'manual'");
        assert_eq!(get("org.gnome.system.proxy.https", "port"), "10809");
        assert_eq!(get("org.gnome.system.proxy.socks", "port"), "10808");
        assert!(get("org.gnome.system.proxy", "ignore-hosts").contains("'192.168.*'"));
    }

    #[test]
    fn snapshots_survive_the_disk() {
        let s = Snapshot::Windows { values: vec![("ProxyEnable".into(), Some(RegValue::Dword(0))), ("AutoConfigURL".into(), None)] };
        let back: Snapshot = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(s, back);
    }
}
