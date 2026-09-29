use std::time::Duration;

use tokio::time::{sleep, Instant};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::client::ApiClient;
use crate::error::ApiError;
use crate::types::{LinkStatus, SignedIn};

/// How a Telegram sign-in ended.
#[derive(Debug)]
pub enum LinkOutcome {
    Approved(Box<SignedIn>),
    Denied,
    /// The five minutes ran out (or the tokens were already collected).
    Expired,
    Cancelled,
}

/// Longest pause between polls after a network error: short enough that an
/// approval right after the network returns is picked up within seconds.
const MAX_BACKOFF: Duration = Duration::from_secs(8);

/// Long-polls `link/poll` until the customer decides in the bot, the request
/// expires at `deadline`, or `cancel` fires.
///
/// Each poll waits server-side for up to 25 seconds, so this is a handful of
/// requests over five minutes, not a busy loop. Network errors are retried
/// with backoff: a laptop changing Wi-Fi mid-sign-in must not lose the
/// approval the customer is about to give.
pub async fn wait_for_approval(
    api: &ApiClient,
    poll_token: &str,
    deadline: Instant,
    cancel: &CancellationToken,
) -> Result<LinkOutcome, ApiError> {
    let mut backoff = Duration::from_secs(1);
    loop {
        if Instant::now() >= deadline {
            return Ok(LinkOutcome::Expired);
        }
        let poll = tokio::select! {
            _ = cancel.cancelled() => return Ok(LinkOutcome::Cancelled),
            r = api.link_poll(poll_token, true) => r,
        };
        match poll {
            Ok(p) => {
                backoff = Duration::from_secs(1);
                match p.status {
                    LinkStatus::Pending => continue,
                    LinkStatus::Denied => return Ok(LinkOutcome::Denied),
                    LinkStatus::Expired => return Ok(LinkOutcome::Expired),
                    LinkStatus::Approved => {
                        return match (p.tokens, p.user) {
                            (Some(tokens), Some(user)) => Ok(LinkOutcome::Approved(Box::new(SignedIn { tokens, user }))),
                            _ => Err(ApiError::Decode("approved without tokens".into())),
                        };
                    }
                }
            }
            // The server no longer knows this poll token.
            Err(ApiError::Http { status: 404, .. }) => return Ok(LinkOutcome::Expired),
            Err(e) if e.is_transient() || matches!(e, ApiError::RateLimited) => {
                tokio::select! {
                    _ = cancel.cancelled() => return Ok(LinkOutcome::Cancelled),
                    _ = sleep(backoff) => {}
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
            Err(e) => return Err(e),
        }
    }
}

/// The server's `https://t.me/<bot>?start=<param>` as a `tg://` link, which
/// opens Telegram Desktop directly instead of a browser tab. None if the
/// link is not the shape the backend documents.
pub fn telegram_app_link(deep_link: &str) -> Option<String> {
    let url = Url::parse(deep_link).ok()?;
    if url.host_str()? != "t.me" {
        return None;
    }
    let domain = url.path().trim_matches('/');
    if domain.is_empty() || !domain.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let start = url.query_pairs().find(|(k, _)| k == "start").map(|(_, v)| v.into_owned())?;
    if !start.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return None;
    }
    Some(format!("tg://resolve?domain={domain}&start={start}"))
}

#[cfg(test)]
mod tests {
    use super::telegram_app_link;

    #[test]
    fn converts_the_documented_deep_link() {
        assert_eq!(
            telegram_app_link("https://t.me/GeekVPNBot?start=applogin_Ab-9_x").as_deref(),
            Some("tg://resolve?domain=GeekVPNBot&start=applogin_Ab-9_x")
        );
    }

    #[test]
    fn refuses_anything_else() {
        assert_eq!(telegram_app_link("https://evil.example/GeekVPNBot?start=x"), None);
        assert_eq!(telegram_app_link("https://t.me/Geek&x=1?start=a"), None);
        assert_eq!(telegram_app_link("https://t.me/GeekVPNBot?start=a%26b"), None);
        assert_eq!(telegram_app_link("https://t.me/GeekVPNBot"), None);
    }
}
