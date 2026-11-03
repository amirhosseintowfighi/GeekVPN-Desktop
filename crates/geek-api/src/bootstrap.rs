//! Bootstrap proxy for the very first sign-in.
//!
//! When the user is behind a filter that blocks Telegram / the GeekVPN API,
//! the app needs *something* to get the first `link/start` / `link/poll`
//! through. The proxy is injected at build time from the
//! `GEEK_BOOTSTRAP_PROXY` secret (see `build.rs`); the source holds nothing.
//! A build without it simply has no fallback.
//!
//! ### Threat model
//! The bytes in the binary are **obfuscated, not encrypted**: a determined
//! reverse engineer can recover the proxy. It should be a rate-limited proxy
//! that is rotated (by changing the secret and releasing) if it is abused.
//!
//! ### Behaviour
//! * Direct connection is always tried first.
//! * Only on a **network error** (`ApiError::Network`) the same request is
//!   retried **once** through this proxy.

use std::time::Duration;

use reqwest::{Client, Proxy};
use zeroize::Zeroize;

const K: u8 = 0x5A;
const STEP: usize = 13;

include!(concat!(env!("OUT_DIR"), "/bootstrap_proxy.rs"));

fn decode(enc: &[u8]) -> String {
    let bytes: Vec<u8> = enc.iter().enumerate().map(|(i, &b)| b ^ K ^ (((i * STEP) & 0xFF) as u8)).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The proxy URL, decoded in memory, or `None` when the build has none.
fn proxy_url() -> Option<String> {
    if PROXY_ENC.is_empty() {
        return None;
    }
    let url = decode(PROXY_ENC);
    url::Url::parse(&url).ok().map(|_| url)
}

/// A `reqwest::Client` through the bootstrap proxy, or `None` when this
/// build has none. The URL buffer is zeroized once `Proxy::all` has it.
pub(crate) fn build_bootstrap_client(user_agent: &str) -> Option<Client> {
    let mut url = proxy_url()?;
    let proxy = Proxy::all(url.as_str()).ok();
    url.zeroize();
    Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .proxy(proxy?)
        .build()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_reverses_the_build_script() {
        let plain = b"socks5h://u:p@example.com:1080";
        let enc: Vec<u8> = plain.iter().enumerate().map(|(i, &b)| b ^ K ^ (((i * STEP) & 0xFF) as u8)).collect();
        assert_eq!(decode(&enc).as_bytes(), plain);
    }

    #[test]
    fn a_build_without_the_secret_has_no_proxy() {
        if PROXY_ENC.is_empty() {
            assert!(proxy_url().is_none());
            assert!(build_bootstrap_client("t").is_none());
        }
    }
}
