use geek_api::DeviceInfo;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Not a secret: it only keeps the raw machine id (Windows `MachineGuid`,
/// macOS `IOPlatformUUID`, Linux `/etc/machine-id`) from leaving the
/// computer, and makes GeekVPN's id useless to anyone correlating by it.
/// Overridable at build time via `GEEK_DEVICE_ID_KEY` (set in CI Secrets);
/// `build.rs` injects it as compile-time env so the literal never appears
/// in the binary when overridden.
fn device_id_key() -> &'static [u8] {
    match option_env!("GEEK_DEVICE_ID_KEY") {
        Some(v) => v.as_bytes(),
        None => b"geekvpn-desktop/device-id/v1",
    }
}

/// The server's limits (`app_link_login._MAX_DEVICE_ID`, `_MAX_DEVICE_NAME`).
const MAX_DEVICE_NAME: usize = 64;

/// HMAC-SHA256 of the machine id, as 64 hex characters: stable across
/// reinstalls and sign-outs, so the bot's device list and its per-device rate
/// limit see one computer as one device.
pub fn device_id_from(machine_id: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(device_id_key()).expect("HMAC takes any key length");
    mac.update(machine_id.trim().as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// This computer as sign-in describes it. `fallback_id` is used only when the
/// OS will not tell us its machine id (a stripped container, say); the app
/// keeps one in the keychain for that case.
pub fn device_info(app_version: &str, fallback_id: impl FnOnce() -> String) -> DeviceInfo {
    let device_id = match machine_uid::get() {
        Ok(id) if !id.trim().is_empty() => device_id_from(&id),
        _ => device_id_from(&fallback_id()),
    };
    let host = gethostname::gethostname().to_string_lossy().trim().to_string();
    DeviceInfo {
        device_id,
        device_name: host.chars().take(MAX_DEVICE_NAME).collect(),
        platform: platform().into(),
        app_version: app_version.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_id_is_64_hex_characters_and_stable() {
        let a = device_id_from("4c4c4544-0042-3510-8051-b4c04f4d4e32");
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a, device_id_from("4c4c4544-0042-3510-8051-b4c04f4d4e32\n"));
        assert_ne!(a, device_id_from("another-machine"));
    }

    #[test]
    fn the_raw_machine_id_is_not_in_it() {
        let raw = "4c4c4544004235108051b4c04f4d4e32";
        assert!(!device_id_from(raw).contains(raw));
    }

    #[test]
    fn device_info_fits_the_servers_limits() {
        let info = device_info("0.1.0", || "fallback".into());
        assert_eq!(info.device_id.len(), 64);
        assert!(info.device_name.chars().count() <= MAX_DEVICE_NAME);
        assert!(["windows", "macos", "linux"].contains(&info.platform.as_str()));
    }
}
