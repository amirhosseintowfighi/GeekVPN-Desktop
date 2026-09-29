//! Sign-in against a running GeekVPNBot API. Skipped unless asked for:
//!
//! ```text
//! GEEK_E2E_API=http://127.0.0.1:8765/ \
//! GEEK_E2E_APPROVER='python approve.py {link}' \
//! cargo test -p geek-api --test e2e -- --ignored
//! ```
//!
//! `GEEK_E2E_APPROVER` plays the bot: it must claim the link's code and press
//! approve in the backend's own database, as `handlers/app_login.py` does.

use std::process::Command;
use std::time::Duration;

use geek_api::{wait_for_approval, ApiClient, ApiError, DeviceInfo, LinkOutcome};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use url::Url;

#[tokio::test]
#[ignore = "needs a running backend (GEEK_E2E_API)"]
async fn telegram_sign_in_refresh_and_reuse_detection_against_the_real_backend() {
    let base = std::env::var("GEEK_E2E_API").expect("GEEK_E2E_API");
    let approver = std::env::var("GEEK_E2E_APPROVER").expect("GEEK_E2E_APPROVER");
    let api = ApiClient::new(Url::parse(&base).unwrap(), "GeekVPN/e2e (linux)").unwrap();
    let device = DeviceInfo {
        device_id: geek_e2e_device_id(),
        device_name: "e2e-desktop".into(),
        platform: "linux".into(),
        app_version: "0.1.0".into(),
    };

    let start = api.link_start(&device).await.expect("link/start");
    assert!(start.deep_link.contains("start=applogin_"));

    // The customer takes a few seconds to press the button; the poll is
    // already waiting server-side when they do.
    let link = start.deep_link.clone();
    let bot = tokio::task::spawn_blocking(move || {
        std::thread::sleep(Duration::from_secs(2));
        let status = Command::new("sh").arg("-c").arg(approver.replace("{link}", &link)).status().unwrap();
        assert!(status.success(), "approver failed");
    });
    let outcome = wait_for_approval(&api, &start.poll_token, Instant::now() + Duration::from_secs(60), &CancellationToken::new())
        .await
        .expect("poll");
    bot.await.unwrap();
    let LinkOutcome::Approved(signed_in) = outcome else { panic!("expected approval, got {outcome:?}") };

    // The tokens are collected once: polling again reads as expired.
    let again = api.link_poll(&start.poll_token, false).await.expect("second poll");
    assert_eq!(again.status, geek_api::LinkStatus::Expired);

    let me = api.me(&signed_in.tokens.access_token).await.unwrap_or_else(|e| panic!("me: {e:?}"));
    assert_eq!(me.telegram_id, signed_in.user.telegram_id);

    let rotated = api.refresh(&signed_in.tokens.refresh_token).await.expect("refresh");
    assert_ne!(rotated.refresh_token, signed_in.tokens.refresh_token);
    // Reusing the old refresh token is treated as theft and refused.
    let reuse = api.refresh(&signed_in.tokens.refresh_token).await.unwrap_err();
    assert!(matches!(reuse, ApiError::Unauthorized), "{reuse:?}");

    // That reuse revoked the whole session, rotated pair included.
    let after_theft = api.refresh(&rotated.refresh_token).await.unwrap_err();
    assert!(matches!(after_theft, ApiError::Unauthorized), "{after_theft:?}");
}

#[tokio::test]
#[ignore = "needs a running backend (GEEK_E2E_API)"]
async fn logout_ends_the_session_on_the_server() {
    let base = std::env::var("GEEK_E2E_API").expect("GEEK_E2E_API");
    let approver = std::env::var("GEEK_E2E_APPROVER").expect("GEEK_E2E_APPROVER");
    let api = ApiClient::new(Url::parse(&base).unwrap(), "GeekVPN/e2e (linux)").unwrap();
    let device = DeviceInfo {
        device_id: geek_e2e_device_id(),
        device_name: "e2e-desktop".into(),
        platform: "linux".into(),
        app_version: "0.1.0".into(),
    };
    let start = api.link_start(&device).await.expect("link/start");
    let link = start.deep_link.clone();
    tokio::task::spawn_blocking(move || {
        let status = Command::new("sh").arg("-c").arg(approver.replace("{link}", &link)).status().unwrap();
        assert!(status.success(), "approver failed");
    })
    .await
    .unwrap();
    let LinkOutcome::Approved(signed_in) =
        wait_for_approval(&api, &start.poll_token, Instant::now() + Duration::from_secs(30), &CancellationToken::new())
            .await
            .expect("poll")
    else {
        panic!("not approved")
    };

    api.logout(&signed_in.tokens.access_token).await.expect("logout");
    let refused = api.refresh(&signed_in.tokens.refresh_token).await.unwrap_err();
    assert!(matches!(refused, ApiError::Unauthorized), "{refused:?}");
}

fn geek_e2e_device_id() -> String {
    // Fresh per run so the per-device start limit (5 per 10 minutes) never bites.
    format!("{:064x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())
}
