use std::time::Duration;

use reqwest::{Method, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::Url;

use crate::error::ApiError;
use crate::types::{
    AppUser, DeviceInfo, LinkPoll, LinkStart, PasswordRequest, PollRequest, Problem, RefreshRequest, SignedIn,
    SubscriptionCard, Tokens,
};

/// The server holds a waiting poll for up to 25 seconds
/// (`app_auth.POLL_WAIT_SECONDS`); this leaves room for a slow link on top.
const POLL_TIMEOUT: Duration = Duration::from_secs(45);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// A thin, typed client for the endpoints the desktop app uses. It holds no
/// tokens: `Session` decides which token goes with which call.
#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    base: Url,
}

impl ApiClient {
    pub fn new(base: Url, user_agent: &str) -> Result<Self, ApiError> {
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ApiError::Network(e.to_string()))?;
        Ok(Self { http, base })
    }

    pub(crate) fn request(&self, method: Method, path: &str) -> Result<RequestBuilder, ApiError> {
        let url = self.base.join(path).map_err(|e| ApiError::Decode(e.to_string()))?;
        Ok(self.http.request(method, url).timeout(REQUEST_TIMEOUT))
    }

    // -- sign-in ------------------------------------------------------------

    pub async fn link_start(&self, device: &DeviceInfo) -> Result<LinkStart, ApiError> {
        let req = self.request(Method::POST, "api/app/auth/link/start")?.json(device);
        send(req, Auth::None).await
    }

    /// One poll. With `wait` the server holds the request until the customer
    /// decides or 25 seconds pass.
    pub async fn link_poll(&self, poll_token: &str, wait: bool) -> Result<LinkPoll, ApiError> {
        let req = self
            .request(Method::POST, "api/app/auth/link/poll")?
            .timeout(POLL_TIMEOUT)
            .json(&PollRequest { poll_token, wait });
        send(req, Auth::None).await
    }

    /// The username and password set in the bot (profile → «ورود به اپ با نام
    /// کاربری»). A wrong pair is a 401 whose Persian message says so.
    pub async fn password_login(&self, username: &str, password: &str, device: &DeviceInfo) -> Result<SignedIn, ApiError> {
        let req = self.request(Method::POST, "api/app/auth/password")?.json(&PasswordRequest {
            username,
            password,
            device_name: &device.device_name,
            platform: &device.platform,
            app_version: &device.app_version,
        });
        send(req, Auth::None).await
    }

    // -- the session --------------------------------------------------------

    /// Rotates the pair. The old refresh token is dead afterwards, and using
    /// it twice makes the backend revoke the whole session as stolen.
    pub async fn refresh(&self, refresh_token: &str) -> Result<Tokens, ApiError> {
        let req = self
            .request(Method::POST, "api/v1/auth/refresh")?
            .json(&RefreshRequest { refresh_token });
        send(req, Auth::Session).await
    }

    pub async fn logout(&self, access_token: &str) -> Result<(), ApiError> {
        let req = self.request(Method::POST, "api/v1/auth/logout")?.bearer_auth(access_token);
        send::<serde_json::Value>(req, Auth::Session).await.map(|_| ())
    }

    pub async fn me(&self, access_token: &str) -> Result<AppUser, ApiError> {
        let req = self.request(Method::GET, "api/v1/auth/me")?.bearer_auth(access_token);
        send(req, Auth::Session).await
    }

    pub async fn subscriptions(&self, access_token: &str) -> Result<Vec<SubscriptionCard>, ApiError> {
        self.get_json("api/miniapp/subscriptions", access_token).await
    }

    /// A subscription's body: the panel's share links, usually base64. It is
    /// public by URL (the token is in it), so no Bearer goes along.
    pub async fn fetch_subscription(&self, url: &str) -> Result<String, ApiError> {
        let resp = self.http.get(url).timeout(REQUEST_TIMEOUT).send().await?;
        if !resp.status().is_success() {
            return Err(ApiError::Http { status: resp.status().as_u16(), title: "subscription".into(), message_fa: None });
        }
        resp.text().await.map_err(|e| ApiError::Decode(e.to_string()))
    }

    /// Any Mini App route (`/api/miniapp/...`) over the app's Bearer token.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str, access_token: &str) -> Result<T, ApiError> {
        let req = self.request(Method::GET, path)?.bearer_auth(access_token);
        send(req, Auth::Session).await
    }

    pub async fn post_json<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        access_token: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let req = self.request(Method::POST, path)?.bearer_auth(access_token).json(body);
        send(req, Auth::Session).await
    }
}

/// Whether a 401 means "this session is over" (`Session`) or is an ordinary
/// refusal with its own message (a wrong password).
#[derive(Clone, Copy)]
enum Auth {
    None,
    Session,
}

/// A Bearer call whose 401 means the session is over.
pub(crate) async fn send_session<T: DeserializeOwned>(req: RequestBuilder) -> Result<T, ApiError> {
    send(req, Auth::Session).await
}

async fn send<T: DeserializeOwned>(req: RequestBuilder, auth: Auth) -> Result<T, ApiError> {
    let resp = req.send().await?;
    let status = resp.status();
    if status.is_success() {
        return resp.json::<T>().await.map_err(|e| ApiError::Decode(e.to_string()));
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(ApiError::RateLimited);
    }
    if status == StatusCode::UNAUTHORIZED && matches!(auth, Auth::Session) {
        return Err(ApiError::Unauthorized);
    }
    // A proxy or captive portal can answer with HTML; the status still counts.
    let problem: Problem = resp.json().await.unwrap_or_default();
    Err(ApiError::Http {
        status: status.as_u16(),
        title: problem.title,
        message_fa: problem.message_fa,
    })
}
