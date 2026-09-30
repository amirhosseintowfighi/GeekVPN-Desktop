//! The wire shapes of GeekVPNBot's app API.
//!
//! The backend serialises camelCase (`presentation/api/base_schema.py`) and
//! its request models forbid unknown fields, so every request here spells
//! exactly the fields the server declares and nothing else.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Who is signing in. Sent with every sign-in so the bot's approval prompt
/// and its device list can name this computer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// Stable per machine, at most 64 characters (see `geek-secrets`).
    pub device_id: String,
    /// The computer's hostname, at most 64 characters.
    pub device_name: String,
    /// `windows`, `macos` or `linux`.
    pub platform: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkStart {
    pub request_id: String,
    pub poll_token: String,
    /// `https://t.me/<bot>?start=applogin_<code>`.
    pub deep_link: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkStatus {
    Pending,
    Approved,
    Denied,
    /// Also what a request whose tokens were already collected reads as.
    Expired,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkPoll {
    pub status: LinkStatus,
    pub tokens: Option<Tokens>,
    pub user: Option<AppUser>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
    pub session_id: String,
}

/// The customer, as sign-in and `/api/v1/auth/me` describe them. Only the
/// fields the app shows; serde ignores the rest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUser {
    pub id: String,
    pub telegram_id: i64,
    pub display_name: String,
    pub username: Option<String>,
    pub referral_code: String,
    pub photo_url: Option<String>,
}

/// A finished sign-in: what the app keeps in the OS keychain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedIn {
    pub tokens: Tokens,
    pub user: AppUser,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollRequest<'a> {
    pub poll_token: &'a str,
    pub wait: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PasswordRequest<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub device_name: &'a str,
    pub platform: &'a str,
    pub app_version: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RefreshRequest<'a> {
    pub refresh_token: &'a str,
}

/// RFC 9457 problem details (`presentation/api/errors.py`). `message_fa` is
/// the one field written for the customer, and the one key not in camelCase.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Problem {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub message_fa: Option<String>,
}

/// One service, as `/api/miniapp/subscriptions` lists it (camelCased by the
/// Mini App router). Only what the desktop app uses.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionCard {
    pub subscription_id: String,
    pub product_name_fa: String,
    pub plan_name_fa: String,
    /// active | expired | exhausted | suspended | revoked
    pub state: String,
    pub expires_at: Option<DateTime<Utc>>,
    /// None: unlimited.
    pub quota_gib: Option<f64>,
    #[serde(default)]
    pub used_gib: f64,
    pub subscription_url: Option<String>,
    #[serde(default)]
    pub remote_username: String,
    /// direct | tunnel | elite; None for a service adopted from a link.
    pub tier: Option<String>,
}
