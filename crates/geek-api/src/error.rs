use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    /// The request never got an answer: no network, DNS, TLS, timeout.
    #[error("network: {0}")]
    Network(String),
    /// The session is over: an invalid or already-used refresh token, or a
    /// revoked session. The only way on is signing in again.
    #[error("unauthorized")]
    Unauthorized,
    #[error("rate limited")]
    RateLimited,
    /// Any other refusal. `message_fa` is the server's own Persian wording.
    #[error("http {status}: {title}")]
    Http {
        status: u16,
        title: String,
        message_fa: Option<String>,
    },
    /// The server answered something this client cannot read: a proxy's
    /// HTML page, or a contract change.
    #[error("unexpected response: {0}")]
    Decode(String),
}

impl ApiError {
    /// What the customer is told. Plain Persian that says what to do next.
    pub fn user_message(&self) -> String {
        match self {
            ApiError::Network(_) => {
                "به سرور GeekVPN وصل نشدیم. اتصال اینترنت را بررسی کن و دوباره امتحان کن.".into()
            }
            ApiError::Unauthorized => "نشست تو تمام شده. دوباره وارد شو.".into(),
            ApiError::RateLimited => "تعداد تلاش‌ها زیاد بود. چند دقیقه صبر کن و دوباره امتحان کن.".into(),
            ApiError::Http { message_fa: Some(m), .. } if !m.trim().is_empty() => m.clone(),
            ApiError::Http { status, .. } if *status >= 500 => {
                "سرور GeekVPN الان جواب نمی‌دهد. کمی بعد دوباره امتحان کن.".into()
            }
            ApiError::Http { .. } => "درخواست انجام نشد. دوباره امتحان کن.".into(),
            ApiError::Decode(_) => {
                "جواب سرور قابل خواندن نبود. اگر پشت پروکسی یا فیلترشکن دیگری هستی، آن را خاموش کن.".into()
            }
        }
    }

    /// Worth retrying the same request later (the login poll does).
    pub fn is_transient(&self) -> bool {
        matches!(self, ApiError::Network(_)) || matches!(self, ApiError::Http { status, .. } if *status >= 500)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            ApiError::Decode(e.to_string())
        } else {
            ApiError::Network(e.to_string())
        }
    }
}
