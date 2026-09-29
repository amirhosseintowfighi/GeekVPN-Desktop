//! The GNOME path against real gsettings. Run inside a D-Bus session with
//! dconf (`dbus-run-session -- cargo test -p geek-netplat --test gnome -- --ignored`).

#[cfg(target_os = "linux")]
#[test]
#[ignore = "changes the session's proxy settings; needs gsettings and a D-Bus session"]
fn apply_then_restore_leaves_gnome_as_it_was() {
    use geek_netplat::{apply_system_proxy, restore_system_proxy, ProxySpec};
    use std::process::Command;

    let get = |s: &str, k: &str| {
        String::from_utf8(Command::new("gsettings").args(["get", s, k]).output().unwrap().stdout).unwrap().trim().to_string()
    };
    let before = (get("org.gnome.system.proxy", "mode"), get("org.gnome.system.proxy.http", "port"));

    let snap = apply_system_proxy(&ProxySpec { http_port: 10809, socks_port: 10808 }).unwrap();
    assert_eq!(get("org.gnome.system.proxy", "mode"), "'manual'");
    assert_eq!(get("org.gnome.system.proxy.http", "host"), "'127.0.0.1'");
    assert_eq!(get("org.gnome.system.proxy.http", "port"), "10809");
    assert_eq!(get("org.gnome.system.proxy.socks", "port"), "10808");

    restore_system_proxy(&snap).unwrap();
    assert_eq!((get("org.gnome.system.proxy", "mode"), get("org.gnome.system.proxy.http", "port")), before);
}
