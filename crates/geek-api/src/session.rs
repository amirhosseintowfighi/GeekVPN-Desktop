use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};
use tokio::sync::Mutex;

use crate::client::ApiClient;
use crate::error::ApiError;
use crate::types::{AppUser, SignedIn};

/// Where a session survives restarts: the OS keychain in the app,
/// memory in tests. Failures are reported, never silently swallowed, because
/// a session that was not saved means signing in again after a restart.
pub trait TokenStore: Send + Sync {
    fn load(&self) -> Result<Option<SignedIn>, String>;
    fn save(&self, session: &SignedIn) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
}

/// Refresh this long before the access token expires, so a request never
/// leaves with a token that dies in flight.
const REFRESH_MARGIN: ChronoDuration = ChronoDuration::seconds(60);

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("not signed in")]
    SignedOut,
    #[error(transparent)]
    Api(#[from] ApiError),
}

impl SessionError {
    pub fn user_message(&self) -> String {
        match self {
            SessionError::SignedOut => "برای این کار وارد حسابت شو.".into(),
            SessionError::Api(e) => e.user_message(),
        }
    }
}

/// The one owner of the token pair.
///
/// Refresh tokens are single use and the backend treats a reused one as
/// theft (`sessions.rotate`), revoking the session. So refreshing happens
/// only here, under one lock: callers that need a token while a refresh is
/// in flight wait for it and get the new pair instead of starting their own.
pub struct Session {
    api: ApiClient,
    store: Arc<dyn TokenStore>,
    state: Mutex<Option<SignedIn>>,
    /// Set when the keychain refused a save; the session then lives only
    /// until the app quits, and the UI says so.
    store_error: std::sync::Mutex<Option<String>>,
}

impl Session {
    /// Loads whatever the store holds. A store that cannot be read counts as
    /// signed out; the error is kept for the UI.
    pub fn restore(api: ApiClient, store: Arc<dyn TokenStore>) -> Self {
        let (initial, err) = match store.load() {
            Ok(s) => (s, None),
            Err(e) => (None, Some(e)),
        };
        Self { api, store, state: Mutex::new(initial), store_error: std::sync::Mutex::new(err) }
    }

    pub fn api(&self) -> &ApiClient {
        &self.api
    }

    pub async fn user(&self) -> Option<AppUser> {
        self.state.lock().await.as_ref().map(|s| s.user.clone())
    }

    pub fn store_error(&self) -> Option<String> {
        self.store_error.lock().ok().and_then(|e| e.clone())
    }

    /// Adopts a fresh sign-in (link approval or password).
    pub async fn sign_in(&self, signed_in: SignedIn) {
        self.persist(&signed_in);
        *self.state.lock().await = Some(signed_in);
    }

    /// Ends the session on the server (best effort) and forgets it here. The
    /// local half always happens: a sign-out that fails offline must still
    /// sign out.
    pub async fn sign_out(&self) {
        let token = self.state.lock().await.as_ref().map(|s| s.tokens.access_token.clone());
        if let Some(token) = token {
            let _ = self.api.logout(&token).await;
        }
        self.forget().await;
    }

    async fn forget(&self) {
        *self.state.lock().await = None;
        if let Err(e) = self.store.clear() {
            self.set_store_error(Some(e));
        }
    }

    /// A usable access token, refreshing first if it is about to expire.
    pub async fn access_token(&self) -> Result<String, SessionError> {
        self.token(false).await
    }

    async fn token(&self, force_refresh: bool) -> Result<String, SessionError> {
        let mut guard = self.state.lock().await;
        let current = guard.as_ref().ok_or(SessionError::SignedOut)?;
        let fresh = current.tokens.access_expires_at - Utc::now() > REFRESH_MARGIN;
        if fresh && !force_refresh {
            return Ok(current.tokens.access_token.clone());
        }
        let refresh_token = current.tokens.refresh_token.clone();
        match self.api.refresh(&refresh_token).await {
            Ok(tokens) => {
                let next = SignedIn { tokens, user: current.user.clone() };
                let access = next.tokens.access_token.clone();
                self.persist(&next);
                *guard = Some(next);
                Ok(access)
            }
            Err(ApiError::Unauthorized) => {
                // Revoked, expired, or disconnected from the bot's device
                // list: nothing left to refresh with.
                *guard = None;
                drop(guard);
                if let Err(e) = self.store.clear() {
                    self.set_store_error(Some(e));
                }
                Err(SessionError::Api(ApiError::Unauthorized))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Runs `call` with a token. A 401 on a token that looked valid (revoked
    /// early, clock skew) gets one forced refresh and one retry.
    pub async fn authorized<T, F, Fut>(&self, call: F) -> Result<T, SessionError>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<T, ApiError>>,
    {
        let token = self.token(false).await?;
        match call(token).await {
            Err(ApiError::Unauthorized) => {
                let token = self.token(true).await?;
                call(token).await.map_err(SessionError::from)
            }
            other => other.map_err(SessionError::from),
        }
    }

    /// Re-reads the profile, which also proves the session still stands.
    pub async fn refresh_user(&self) -> Result<AppUser, SessionError> {
        let api = self.api.clone();
        let user = self.authorized(|t| {
            let api = api.clone();
            async move { api.me(&t).await }
        })
        .await?;
        let mut guard = self.state.lock().await;
        if let Some(s) = guard.as_mut() {
            s.user = user.clone();
            let snapshot = s.clone();
            drop(guard);
            self.persist(&snapshot);
        }
        Ok(user)
    }

    fn persist(&self, s: &SignedIn) {
        self.set_store_error(self.store.save(s).err());
    }

    fn set_store_error(&self, e: Option<String>) {
        if let Ok(mut slot) = self.store_error.lock() {
            *slot = e;
        }
    }
}
