//! Picks the backend address at build time. Nothing real is committed: the
//! address comes from the environment (GitHub Secrets in CI), like the
//! Android app's `GEEK_API_BASE_*`.

fn main() {
    for var in ["GEEK_ENV", "GEEK_API_BASE_PROD", "GEEK_API_BASE_STAGING", "GEEK_BOT_USERNAME"] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    let env = std::env::var("GEEK_ENV").unwrap_or_else(|_| if release { "prod" } else { "staging" }.into());
    let var = match env.as_str() {
        "prod" => "GEEK_API_BASE_PROD",
        "staging" => "GEEK_API_BASE_STAGING",
        other => panic!("GEEK_ENV must be prod or staging, not {other:?}"),
    };
    // Unset means a build that cannot reach any server rather than one that
    // quietly talks to the wrong one.
    let base = std::env::var(var).unwrap_or_else(|_| "https://api.geekvpn.invalid/".into());
    // Plain http only for a backend on this machine in a debug build: the
    // end-to-end runs against a local GeekVPNBot. Everything shipped is https.
    let loopback = ["http://127.0.0.1", "http://localhost"].iter().any(|p| base.starts_with(p));
    assert!(
        base.starts_with("https://") || (loopback && !release),
        "{var} must be an https:// address"
    );
    let base = if base.ends_with('/') { base } else { format!("{base}/") };
    println!("cargo:rustc-env=GEEK_API_BASE={base}");
    println!("cargo:rustc-env=GEEK_ENV={env}");
    // The bot, for invite links and «ربات پشتیبانی»; empty hides both.
    let bot = std::env::var("GEEK_BOT_USERNAME").unwrap_or_default();
    let bot = bot.trim().trim_start_matches('@');
    assert!(
        bot.is_empty() || (5..=32).contains(&bot.len()) && bot.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "GEEK_BOT_USERNAME must be a Telegram username, got {bot:?}"
    );
    println!("cargo:rustc-env=GEEK_BOT_USERNAME={bot}");
    tauri_build::build()
}
