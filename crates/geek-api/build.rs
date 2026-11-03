//! The bootstrap proxy (`GEEK_BOOTSTRAP_PROXY`, e.g.
//! `socks5h://user:pass@host:port`) comes from the build environment (a CI
//! secret), never from the source. It is written XOR-obfuscated into
//! `OUT_DIR`, so `strings` on the binary shows no host or password. Unset:
//! no bootstrap proxy, and sign-in goes direct only.

use std::io::Write;

const K: u8 = 0x5A;
const STEP: usize = 13;

fn main() {
    println!("cargo:rerun-if-env-changed=GEEK_BOOTSTRAP_PROXY");
    let raw = std::env::var("GEEK_BOOTSTRAP_PROXY").unwrap_or_default();
    let raw = raw.trim();
    let ok = ["socks5h://", "socks5://", "http://", "https://"].iter().any(|s| raw.starts_with(s));
    if !raw.is_empty() && !ok {
        panic!("GEEK_BOOTSTRAP_PROXY must be a socks5h://, socks5://, http:// or https:// URL");
    }
    let enc: Vec<String> = if ok {
        raw.bytes().enumerate().map(|(i, b)| format!("0x{:02x}", b ^ K ^ ((i * STEP) & 0xFF) as u8)).collect()
    } else {
        vec![]
    };
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("bootstrap_proxy.rs");
    let mut f = std::fs::File::create(out).expect("bootstrap_proxy.rs");
    writeln!(f, "const PROXY_ENC: &[u8] = &[{}];", enc.join(", ")).expect("write");
}
