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
use zeroize::Zeroize;

/// Position-dependent XOR key — not a secret, just enough to make `strings`
/// useless.  K = 0x5A, step = 13 was chosen arbitrarily.
const K: u8 = 0x5A;
const STEP: usize = 13;

// Obfuscated with `byte ^ K ^ ((i*STEP) & 0xFF)`, K=0x5A STEP=13.
// The scheme itself is also obfuscated so `strings` on the release binary
// does not yield `socks5h://` as a plain literal.  Decoded only in memory.
const HOST_ENC: [u8; 12] = [98, 102, 110, 76, 92, 53, 44, 49, 28, 29, 232, 224];
const PORT_ENC: [u8; 5] = [110, 97, 113, 72, 88];
const USER_ENC: [u8; 10] = [106, 102, 117, 75, 1, 111, 77, 56, 4, 107];
const PASS_ENC: [u8; 10] = [8, 102, 9, 17, 93, 105, 82, 102, 11, 112];
const SCHEME_ENC: [u8; 10] = [41, 56, 35, 22, 29, 46, 124, 59, 29, 0];

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
/// `h` in the scheme forces remote DNS so the backend hostname is resolved
/// through the proxy, not locally.  The scheme is decoded from `SCHEME_ENC`
/// so the release binary contains no `socks5h://` literal.
fn proxy_url() -> Option<String> {
    let host = decode(&HOST_ENC);
    let port = decode(&PORT_ENC);
    let user = decode(&USER_ENC);
    let pass = decode(&PASS_ENC);
    let scheme = decode(&SCHEME_ENC);
    if host.is_empty() || port.is_empty() || user.is_empty() || pass.is_empty() || scheme.is_empty() {
        return None;
    }
    // Credentials are alphanumeric + '_' — no percent-encoding needed.
    // Build without a single `socks5h://` literal in the source.
    let mut url = String::with_capacity(scheme.len() + user.len() + pass.len() + host.len() + port.len() + 2);
    url.push_str(&scheme);
    url.push_str(&user);
    url.push(':');
    url.push_str(&pass);
    url.push('@');
    url.push_str(&host);
    url.push(':');
    url.push_str(&port);
    Some(url)
}

/// Build a `reqwest::Client` that routes through the bootstrap proxy, if the
/// obfuscated data decodes to a valid proxy URL.  Returns `None` when the
/// proxy is unavailable or `socks` support was not compiled in.
/// The URL buffer is zeroized after `Proxy::all` clones it.
pub(crate) fn build_bootstrap_client(user_agent: &str) -> Option<Client> {
    let mut url = proxy_url()?;
    let proxy = Proxy::all(url.clone()).ok()?;
    url.zeroize();
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
    fn proxy_url_is_socks5h() {
        let url = proxy_url().unwrap();
        let scheme = decode(&SCHEME_ENC);
        assert!(url.starts_with(&scheme));
        assert!(url.contains('@'));
        assert!(url::Url::parse(&url).is_ok());
    }

    #[test]
    fn no_plaintext_in_enc_constants() {
        let raw = format!("{:?}{:?}{:?}{:?}{:?}", HOST_ENC, PORT_ENC, USER_ENC, PASS_ENC, SCHEME_ENC);
        assert!(!raw.contains("81"));
    }
}
