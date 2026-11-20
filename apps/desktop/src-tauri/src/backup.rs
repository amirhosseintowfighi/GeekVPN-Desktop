//! «پشتیبان‌گیری و بازگردانی»: the customer's own settings and the links
//! they added by hand, in one `.geekvpn` file to carry to another computer
//! or keep for a reinstall. The account's services are not in it (signing in
//! brings them back), nor the app lock, the session or per-network scan
//! data. Links are credentials, so the file can be sealed with a password:
//! Argon2id derives the key, XChaCha20-Poly1305 encrypts.

use std::collections::HashSet;
use std::path::PathBuf;

use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use geek_config::{AppRouting, CustomRule, FailoverThreshold, LanShare, Route, Tuning};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::networks::NetRules;
use crate::servers::ServersState;
use crate::store::{Mode, Persisted, Source, SourceKind};

const FORMAT: u32 = 1;
/// The UI asks for the password when a restore fails with exactly this.
pub const NEEDS_PASSWORD: &str = "password-needed";

/// What a backup carries. Every field has a default, so a file from an
/// older (or newer) app restores what it can.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BackupData {
    pub sources: Vec<Source>,
    pub favorites: HashSet<String>,
    pub auto_select: bool,
    pub route: Route,
    pub sort_by_ping: bool,
    pub mode: Mode,
    pub kill_switch: bool,
    pub strict: bool,
    pub allow_lan: bool,
    pub apps: AppRouting,
    pub auto_connect: bool,
    pub close_to_tray: bool,
    pub shortcut: bool,
    pub expiry_alert: bool,
    pub failover: FailoverThreshold,
    pub rules: Vec<CustomRule>,
    pub tuning: Tuning,
    pub networks: NetRules,
    pub lan_share: LanShare,
}

impl Default for BackupData {
    fn default() -> Self {
        Self::from(&Persisted::default())
    }
}

impl From<&Persisted> for BackupData {
    fn from(d: &Persisted) -> Self {
        Self {
            // The account's services come back by signing in.
            sources: d.sources.iter().filter(|s| !matches!(s.kind, SourceKind::Account { .. })).cloned().collect(),
            favorites: d.favorites.clone(),
            auto_select: d.auto_select,
            route: d.route,
            sort_by_ping: d.sort_by_ping,
            mode: d.mode,
            kill_switch: d.kill_switch,
            strict: d.strict,
            allow_lan: d.allow_lan,
            apps: d.apps.clone(),
            auto_connect: d.auto_connect,
            close_to_tray: d.close_to_tray,
            shortcut: d.shortcut,
            expiry_alert: d.expiry_alert,
            failover: d.failover,
            rules: d.rules.clone(),
            tuning: d.tuning.clone(),
            networks: d.networks.clone(),
            lan_share: d.lan_share.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSummary {
    pub links_added: usize,
    pub rules: usize,
}

impl BackupData {
    /// Settings are replaced; hand-added links are added beside what is
    /// there (same subscription URL or same id: kept once).
    pub fn apply(self, d: &mut Persisted) -> RestoreSummary {
        let mut added = 0;
        for s in self.sources {
            if matches!(s.kind, SourceKind::Account { .. }) {
                continue;
            }
            let dup = d.sources.iter().any(|x| x.id == s.id || (s.url.is_some() && x.url == s.url));
            if !dup {
                d.sources.push(s);
                added += 1;
            }
        }
        d.favorites.extend(self.favorites);
        d.auto_select = self.auto_select;
        d.route = self.route;
        d.sort_by_ping = self.sort_by_ping;
        d.mode = self.mode;
        d.kill_switch = self.kill_switch;
        d.strict = self.strict;
        d.allow_lan = self.allow_lan;
        d.apps = self.apps;
        d.auto_connect = self.auto_connect;
        d.close_to_tray = self.close_to_tray;
        d.shortcut = self.shortcut;
        d.expiry_alert = self.expiry_alert;
        d.failover = self.failover;
        let rules = self.rules.len();
        d.rules = self.rules;
        d.tuning = self.tuning.cleaned();
        d.networks = self.networks;
        d.lan_share = self.lan_share;
        RestoreSummary { links_added: added, rules }
    }
}

fn key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut key = [0u8; 32];
    Argon2::default().hash_password_into(password.as_bytes(), salt, &mut key).map_err(|e| e.to_string())?;
    Ok(key)
}

/// The file's text: plain JSON, or sealed with `password`.
pub fn seal(data: &BackupData, password: Option<&str>) -> Result<String, String> {
    let created_at = chrono::Utc::now().to_rfc3339();
    let file = match password.filter(|p| !p.is_empty()) {
        None => serde_json::json!({ "geekvpnBackup": FORMAT, "createdAt": created_at, "encrypted": false, "data": data }),
        Some(pw) => {
            let mut salt = [0u8; 16];
            chacha20poly1305::aead::rand_core::RngCore::fill_bytes(&mut OsRng, &mut salt);
            let cipher = XChaCha20Poly1305::new(&key(pw, &salt)?.into());
            let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
            let plain = serde_json::to_vec(data).map_err(|e| e.to_string())?;
            let sealed = cipher.encrypt(&nonce, plain.as_slice()).map_err(|e| e.to_string())?;
            serde_json::json!({
                "geekvpnBackup": FORMAT, "createdAt": created_at, "encrypted": true,
                "salt": B64.encode(salt), "nonce": B64.encode(nonce), "data": B64.encode(sealed),
            })
        }
    };
    serde_json::to_string_pretty(&file).map_err(|e| e.to_string())
}

/// Reads a backup's text. A sealed file without a password is
/// `NEEDS_PASSWORD`; a wrong password says so in Persian.
pub fn open(text: &str, password: Option<&str>) -> Result<BackupData, String> {
    let bad = || "این فایل، پشتیبان GeekVPN نیست یا خراب شده است.".to_string();
    let file: Value = serde_json::from_str(text).map_err(|_| bad())?;
    let version = file["geekvpnBackup"].as_u64().ok_or_else(bad)?;
    if version > u64::from(FORMAT) {
        return Err("این پشتیبان با نسخه‌ی تازه‌تری از برنامه ساخته شده؛ اول برنامه را به‌روز کن.".into());
    }
    if file["encrypted"] != Value::Bool(true) {
        return serde_json::from_value(file["data"].clone()).map_err(|_| bad());
    }
    let Some(pw) = password.filter(|p| !p.is_empty()) else { return Err(NEEDS_PASSWORD.into()) };
    let field = |k: &str| file[k].as_str().and_then(|s| B64.decode(s).ok()).ok_or_else(bad);
    let (salt, nonce, sealed) = (field("salt")?, field("nonce")?, field("data")?);
    if nonce.len() != 24 {
        return Err(bad());
    }
    let cipher = XChaCha20Poly1305::new(&key(pw, &salt)?.into());
    let plain = cipher.decrypt(XNonce::from_slice(&nonce), sealed.as_slice()).map_err(|_| "رمز پشتیبان درست نیست.".to_string())?;
    serde_json::from_slice(&plain).map_err(|_| bad())
}

fn backup_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("geekvpn")) {
        return Err("فایل پشتیبان باید پسوند geekvpn داشته باشد.".into());
    }
    Ok(path)
}

#[tauri::command]
pub async fn backup_save(state: State<'_, ServersState>, path: String, password: Option<String>) -> Result<(), String> {
    let path = backup_path(&path)?;
    let data = BackupData::from(&state.store.lock().await.data);
    let text = tauri::async_runtime::spawn_blocking(move || seal(&data, password.as_deref())).await.map_err(|e| e.to_string())??;
    std::fs::write(&path, text).map_err(|e| format!("ذخیره نشد: {e}"))
}

#[tauri::command]
pub async fn backup_restore(app: AppHandle, state: State<'_, ServersState>, path: String, password: Option<String>) -> Result<RestoreSummary, String> {
    let path = backup_path(&path)?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("خوانده نشد: {e}"))?;
    let data = tauri::async_runtime::spawn_blocking(move || open(&text, password.as_deref())).await.map_err(|e| e.to_string())??;
    let summary = {
        let mut store = state.store.lock().await;
        let summary = data.apply(&mut store.data);
        store.save()?;
        crate::desktop::apply(&app, &store.data);
        summary
    };
    // The pages read the store again.
    let _ = app.emit(crate::servers::CHANGED, ());
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Persisted {
        let mut d = Persisted::default();
        d.sources.push(Source { id: "m1".into(), name: "دستی".into(), kind: SourceKind::Manual, url: None, links: vec!["vless://a@b:443#x".into()], updated_at: None });
        d.sources.push(Source {
            id: "acc".into(),
            name: "سرویس".into(),
            kind: SourceKind::Account { subscription_id: "1".into(), tier: None, category_name_fa: None, state: "active".into(), expires_at: None, quota_gib: None, used_gib: 0.0 },
            url: Some("https://sub/acc".into()),
            links: vec![],
            updated_at: None,
        });
        d.route = Route::Direct;
        d.lock.pin_hash = Some("secret-hash".into());
        d
    }

    #[test]
    fn carries_settings_and_hand_added_links_only() {
        let data = BackupData::from(&sample());
        assert_eq!(data.sources.len(), 1);
        let text = seal(&data, None).unwrap();
        assert!(!text.contains("secret-hash") && !text.contains("https://sub/acc"));
        let back = open(&text, None).unwrap();
        let mut fresh = Persisted::default();
        let s = back.apply(&mut fresh);
        assert_eq!(s.links_added, 1);
        assert_eq!(fresh.route, Route::Direct);
        // Restoring twice adds nothing twice.
        assert_eq!(open(&text, None).unwrap().apply(&mut fresh).links_added, 0);
    }

    #[test]
    fn sealed_needs_the_right_password() {
        let text = seal(&BackupData::from(&sample()), Some("p@ss")).unwrap();
        assert!(!text.contains("vless://"));
        assert_eq!(open(&text, None).unwrap_err(), NEEDS_PASSWORD);
        assert!(open(&text, Some("wrong")).unwrap_err().contains("رمز"));
        assert_eq!(open(&text, Some("p@ss")).unwrap().sources.len(), 1);
    }

    #[test]
    fn rejects_other_files() {
        assert!(open("{}", None).is_err());
        assert!(open("not json", None).is_err());
        assert!(open(r#"{"geekvpnBackup":99,"data":{}}"#, None).unwrap_err().contains("تازه‌تر"));
        assert!(backup_path("/tmp/x.json").is_err());
    }
}
