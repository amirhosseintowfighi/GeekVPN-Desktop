use geek_api::{SignedIn, TokenStore};
use keyring::Entry;

const SERVICE: &str = "com.geekvpn.desktop";
const SESSION: &str = "session";
const FALLBACK_DEVICE: &str = "device-fallback-id";

/// The session in the OS keychain: Credential Manager on Windows, Keychain on
/// macOS, the Secret Service (GNOME Keyring, KWallet) on Linux. Nothing is
/// ever written to a plain file; with no keychain the session lasts until
/// the app quits and the UI says why.
pub struct KeychainStore {
    session: Entry,
}

impl KeychainStore {
    pub fn new() -> Result<Self, String> {
        Ok(Self { session: Entry::new(SERVICE, SESSION).map_err(describe)? })
    }

    /// A random id kept in the keychain, for machines whose OS id is
    /// unavailable. Returns a fresh one (unsaved) if even that fails.
    pub fn fallback_device_id(random: impl FnOnce() -> String) -> String {
        let fresh = random();
        let Ok(entry) = Entry::new(SERVICE, FALLBACK_DEVICE) else { return fresh };
        match entry.get_password() {
            Ok(id) => id,
            Err(_) => {
                let _ = entry.set_password(&fresh);
                fresh
            }
        }
    }
}

impl TokenStore for KeychainStore {
    fn load(&self) -> Result<Option<SignedIn>, String> {
        match self.session.get_password() {
            Ok(json) => serde_json::from_str(&json)
                .map(Some)
                // An unreadable blob (an older format) is as good as none.
                .or(Ok(None)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(describe(e)),
        }
    }

    fn save(&self, session: &SignedIn) -> Result<(), String> {
        let json = serde_json::to_string(session).map_err(|e| e.to_string())?;
        self.session.set_password(&json).map_err(describe)
    }

    fn clear(&self) -> Result<(), String> {
        match self.session.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(describe(e)),
        }
    }
}

fn describe(e: keyring::Error) -> String {
    match e {
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_) => {
            "کلیدساز امن سیستم (Keychain / Credential Manager / Secret Service) در دسترس نیست؛ ورود فقط تا بستن برنامه می‌ماند.".into()
        }
        other => format!("کلیدساز امن سیستم خطا داد: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geek_api::{AppUser, Tokens};

    fn sample() -> SignedIn {
        SignedIn {
            tokens: Tokens {
                access_token: "a".into(),
                refresh_token: "r".into(),
                access_expires_at: "2026-09-29T12:15:00Z".parse().unwrap(),
                refresh_expires_at: "2026-10-29T12:00:00Z".parse().unwrap(),
                session_id: "s".into(),
            },
            user: AppUser {
                id: "u".into(),
                telegram_id: 1,
                display_name: "امیر".into(),
                username: None,
                referral_code: "K".into(),
                photo_url: None,
            },
        }
    }

    #[test]
    fn round_trips_through_the_keychain() {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        let store = KeychainStore::new().unwrap();
        // The mock keeps one credential per Entry, so this exercises the
        // real code paths without touching the machine's keychain.
        assert_eq!(store.load().unwrap(), None);
        store.save(&sample()).unwrap();
        assert_eq!(store.load().unwrap(), Some(sample()));
        store.clear().unwrap();
        assert_eq!(store.load().unwrap(), None);
        store.clear().unwrap();
    }
}
