//! «قفل برنامه»: a PIN in front of the window, so whoever sits at the
//! computer cannot read the account, the links or the QR codes. The PIN is
//! kept only as an Argon2 hash; wrong tries are slowed down. It is a privacy
//! screen for the window, not encryption: the tunnel and the tray keep
//! working while locked.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::servers::ServersState;

pub const CHANGED: &str = "lock://changed";
const FREE_TRIES: u32 = 5;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppLock {
    /// Argon2 PHC string; `None` means no lock.
    pub pin_hash: Option<String>,
    /// Lock again after this many idle minutes; 0 = only at start.
    pub idle_minutes: u32,
}

pub struct LockState {
    locked: AtomicBool,
    tries: Mutex<(u32, Option<Instant>)>,
}

impl LockState {
    /// Locked from the start when a PIN is set.
    pub fn new(lock: &AppLock) -> Self {
        Self { locked: AtomicBool::new(lock.pin_hash.is_some()), tries: Mutex::new((0, None)) }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LockView {
    pub enabled: bool,
    pub locked: bool,
    pub idle_minutes: u32,
    /// Seconds to wait before the next try, after too many wrong ones.
    pub wait_secs: u64,
}

fn pin_ok(pin: &str) -> bool {
    (4..=8).contains(&pin.chars().count()) && pin.chars().all(|c| c.is_ascii_digit())
}

pub fn hash(pin: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default().hash_password(pin.as_bytes(), &salt).map(|h| h.to_string()).map_err(|e| e.to_string())
}

pub fn verify(pin: &str, stored: &str) -> bool {
    PasswordHash::new(stored).is_ok_and(|h| Argon2::default().verify_password(pin.as_bytes(), &h).is_ok())
}

/// After `FREE_TRIES` wrong PINs, each further one waits longer: 30 s, 60 s, … up to 10 min.
fn backoff(wrong: u32) -> Duration {
    if wrong < FREE_TRIES {
        return Duration::ZERO;
    }
    Duration::from_secs((30u64 << (wrong - FREE_TRIES).min(5)).min(600))
}

fn wait_left(state: &LockState) -> u64 {
    let t = state.tries.lock().unwrap_or_else(|e| e.into_inner());
    t.1.map(|until| until.saturating_duration_since(Instant::now()).as_secs()).unwrap_or(0)
}

async fn view(servers: &ServersState, lock: &LockState) -> LockView {
    let data = &servers.store.lock().await.data.lock;
    LockView {
        enabled: data.pin_hash.is_some(),
        locked: lock.locked.load(Ordering::SeqCst) && data.pin_hash.is_some(),
        idle_minutes: data.idle_minutes,
        wait_secs: wait_left(lock),
    }
}

async fn changed(app: &AppHandle) -> Result<LockView, String> {
    let v = view(&app.state::<ServersState>(), &app.state::<LockState>()).await;
    let _ = app.emit(CHANGED, v.clone());
    Ok(v)
}

#[tauri::command]
pub async fn lock_state(servers: State<'_, ServersState>, lock: State<'_, LockState>) -> Result<LockView, String> {
    Ok(view(&servers, &lock).await)
}

/// Sets or changes the PIN; changing needs the current one.
#[tauri::command]
pub async fn lock_set_pin(app: AppHandle, current: Option<String>, pin: String) -> Result<LockView, String> {
    if !pin_ok(&pin) {
        return Err("رمز باید ۴ تا ۸ رقم باشد.".into());
    }
    {
        let servers = app.state::<ServersState>();
        let mut store = servers.store.lock().await;
        if let Some(stored) = &store.data.lock.pin_hash {
            if !current.as_deref().is_some_and(|c| verify(c, stored)) {
                return Err("رمز فعلی درست نیست.".into());
            }
        }
        store.data.lock.pin_hash = Some(hash(&pin)?);
        store.save()?;
    }
    changed(&app).await
}

/// Turns the lock off; needs the PIN.
#[tauri::command]
pub async fn lock_disable(app: AppHandle, pin: String) -> Result<LockView, String> {
    {
        let servers = app.state::<ServersState>();
        let mut store = servers.store.lock().await;
        if !store.data.lock.pin_hash.as_deref().is_some_and(|h| verify(&pin, h)) {
            return Err("رمز درست نیست.".into());
        }
        store.data.lock.pin_hash = None;
        store.save()?;
    }
    app.state::<LockState>().locked.store(false, Ordering::SeqCst);
    changed(&app).await
}

#[tauri::command]
pub async fn lock_set_idle(app: AppHandle, minutes: u32) -> Result<LockView, String> {
    {
        let servers = app.state::<ServersState>();
        let mut store = servers.store.lock().await;
        store.data.lock.idle_minutes = minutes.min(240);
        store.save()?;
    }
    changed(&app).await
}

#[tauri::command]
pub async fn lock_now(app: AppHandle) -> Result<LockView, String> {
    app.state::<LockState>().locked.store(true, Ordering::SeqCst);
    changed(&app).await
}

#[tauri::command]
pub async fn lock_unlock(app: AppHandle, pin: String) -> Result<LockView, String> {
    let lock = app.state::<LockState>();
    let wait = wait_left(&lock);
    if wait > 0 {
        return Err(format!("چند بار اشتباه زدی؛ {wait} ثانیه صبر کن."));
    }
    let ok = {
        let servers = app.state::<ServersState>();
        let store = servers.store.lock().await;
        store.data.lock.pin_hash.as_deref().is_none_or(|h| verify(&pin, h))
    };
    {
        let mut t = lock.tries.lock().unwrap_or_else(|e| e.into_inner());
        if ok {
            *t = (0, None);
        } else {
            t.0 += 1;
            let d = backoff(t.0);
            t.1 = (!d.is_zero()).then(|| Instant::now() + d);
        }
    }
    if !ok {
        let _ = changed(&app).await;
        return Err("رمز درست نیست.".into());
    }
    lock.locked.store(false, Ordering::SeqCst);
    changed(&app).await
}

/// «رمز را فراموش کردم»: the lock goes, and with it everything it guarded:
/// the account is signed out and every service and link is removed. Signing
/// in again brings the account's services back.
#[tauri::command]
pub async fn lock_forgot(app: AppHandle) -> Result<LockView, String> {
    let servers = app.state::<ServersState>();
    servers.tunnel.disconnect(&app).await;
    {
        let mut store = servers.store.lock().await;
        store.data.lock = AppLock::default();
        store.data.sources.clear();
        store.data.active_source = None;
        store.data.selected = None;
        store.data.lan_share = geek_config::LanShare::default();
        store.save()?;
    }
    let auth = app.state::<crate::auth::AuthState>();
    crate::auth::logout(app.clone(), auth).await?;
    app.state::<LockState>().locked.store(false, Ordering::SeqCst);
    changed(&app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_are_hashed_and_checked() {
        let h = hash("2468").unwrap();
        assert!(!h.contains("2468"));
        assert!(verify("2468", &h));
        assert!(!verify("2469", &h));
        assert!(!verify("2468", "not a hash"));
    }

    #[test]
    fn pin_shape() {
        assert!(pin_ok("1234") && pin_ok("12345678"));
        assert!(!pin_ok("123") && !pin_ok("123456789") && !pin_ok("12a4") && !pin_ok("۱۲۳۴"));
    }

    #[test]
    fn wrong_tries_slow_down() {
        assert_eq!(backoff(4), Duration::ZERO);
        assert_eq!(backoff(5), Duration::from_secs(30));
        assert_eq!(backoff(6), Duration::from_secs(60));
        assert_eq!(backoff(30), Duration::from_secs(600));
    }
}
