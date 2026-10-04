//! Bootstrap SOCKS5 proxy for the very first sign-in.
//!
//! When the user is behind a filter that blocks Telegram / the GeekVPN API,
//! the app needs *something* to get the first `link/start` / `link/poll`
//! through. This module holds a single fallback SOCKS5 proxy as **obfuscated**
//! bytes — not plaintext — so a plain `strings` / `grep` on the binary does
//! not yield a usable `host:port` / credential.
//!
//! ### Threat model
//! This is **obfuscation, not encryption**. A determined reverse engineer can
//! still recover the proxy by debugging the process (the proxy URL lives in
//! memory at request time and in the `Proxy` object). The goal is only to stop
//! trivial harvesting: copy-paste from `strings`, GitHub search, or automated
//! scanners. The proxy itself is rate-limited and can be rotated server-side
//! if it ever gets abused.
//!
//! ### Behaviour
//! * Direct connection is always tried first.
//! * Only on a **network error** (`ApiError::Network`) the same request is
//!   retried **once** through this proxy.
//! * After the user is signed in and the VPN is connected, the proxy is never
//!   used again.

use std::time::Duration;

use reqwest::{Client, Proxy};

/// Position-dependent XOR key — not a secret, just enough to make `strings`
/// useless.  K = 0x5A, step = 13 was chosen arbitrarily.
const K: u8 = 0x5A;
const STEP: usize = 13;

// Obfuscated with `byte ^ K ^ ((i*STEP) & 0xFF)`, K=0x5A STEP=13.
const HOST_ENC: [u8; 12] = [98, 102, 110, 76, 92, 53, 44, 49, 28, 29, 232, 224];
const PORT_ENC: [u8; 5] = [110, 97, 113, 72, 88];
const USER_ENC: [u8; 10] = [106, 102, 117, 75, 1, 111, 77, 56, 4, 107];
const PASS_ENC: [u8; 10] = [8, 102, 9, 17, 93, 105, 82, 102, 11, 112];

fn decode(enc: &[u8]) -> String {
    let bytes: Vec<u8> = enc
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ K ^ (((i * STEP) & 0xFF) as u8))
        .collect();
    // All four fields are ASCII, so UTF-8 is guaranteed.
    String::from_utf8(bytes).unwrap_or_default()
}

/// Returns `Some(proxy URL)` decoded in memory, or `None` if decoding failed.
/// The URL is `socks5h://user:pass@host:port` — `h` forces remote DNS so the
/// backend hostname is resolved through the proxy, not locally.
fn proxy_url() -> Option<String> {
    let host = decode(&HOST_ENC);
    let port = decode(&PORT_ENC);
    let user = decode(&USER_ENC);
    let pass = decode(&PASS_ENC);
    if host.is_empty() || port.is_empty() || user.is_empty() || pass.is_empty() {
        return None;
    }
    // Credentials are alphanumeric + '_' — no percent-encoding needed, but we
    // keep the format strict to avoid `Proxy::all` parsing surprises.
    Some(format!("socks5h://{user}:{pass}@{host}:{port}"))
}

/// Build a `reqwest::Client` that routes through the bootstrap proxy, if the
/// obfuscated data decodes to a valid proxy URL.  Returns `None` when the
/// proxy is unavailable or `socks` support was not compiled in.
pub(crate) fn build_bootstrap_client(user_agent: &str) -> Option<Client> {
    let url = proxy_url()?;
    let proxy = Proxy::all(url).ok()?;
    Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .proxy(proxy)
        .build()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_url_looks_like_socks5h() {
        let url = proxy_url().unwrap();
        assert!(url.starts_with("socks5h://"));
        assert!(url.contains('@'));
        // Must be parseable as a URL.
        assert!(url::Url::parse(&url).is_ok());
    }

    #[test]
    fn no_plaintext_in_enc_constants() {
        let raw = format!("{:?}{:?}{:?}{:?}", HOST_ENC, PORT_ENC, USER_ENC, PASS_ENC);
        // Encoded form must not contain the dot-separated IP as a substring.
        assert!(!raw.contains("81"));
    }
}
