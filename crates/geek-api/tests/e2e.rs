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

use geek_api::{wait_for_approval, ApiClient, ApiError, DeviceInfo, LinkOutcome, PaymentStart, Purchase, SignedIn};
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

/// Signs in through the approver, as the other tests do.
async fn sign_in(api: &ApiClient) -> SignedIn {
    let approver = std::env::var("GEEK_E2E_APPROVER").expect("GEEK_E2E_APPROVER");
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
    match wait_for_approval(api, &start.poll_token, Instant::now() + Duration::from_secs(30), &CancellationToken::new()).await.expect("poll") {
        LinkOutcome::Approved(s) => *s,
        other => panic!("not approved: {other:?}"),
    }
}

/// The shop, wallet, referral and support routes against the real backend.
/// Needs a published catalog with at least one plan and an active card
/// account in its database.
#[tokio::test]
#[ignore = "needs a running backend (GEEK_E2E_API) with a catalog"]
async fn shop_wallet_referral_and_support_against_the_real_backend() {
    let base = std::env::var("GEEK_E2E_API").expect("GEEK_E2E_API");
    let api = ApiClient::new(Url::parse(&base).unwrap(), "GeekVPN/e2e (linux)").unwrap();
    let t = sign_in(&api).await.tokens.access_token;

    let store = api.storefront(&t).await.expect("storefront");
    let plan = store.categories.iter().flat_map(|c| &c.products).flat_map(|p| &p.plans).next().expect("a plan").clone();
    let purchase = Purchase { plan_id: plan.plan_id.clone(), ..Default::default() };
    let quote = api.quote(&t, &purchase).await.expect("quote");
    assert_eq!(quote.plan_id.replace('-', ""), plan.plan_id.replace('-', ""));
    assert!(quote.total > 0 && quote.total <= plan.compare_at_price.unwrap_or(i64::MAX));

    // An unknown code is a 404 with the reason in Persian; a known one that
    // does not apply is `isValid: false`. Either way the customer reads why.
    match api.coupon_preview(&t, &plan.plan_id, "NO-SUCH-CODE").await {
        Ok(c) => assert!(!c.is_valid && !c.message_fa.is_empty(), "{c:?}"),
        Err(ApiError::Http { status: 404, message_fa: Some(m), .. }) => assert!(!m.is_empty()),
        Err(e) => panic!("coupon: {e:?}"),
    }

    let methods = api.payment_methods(&t).await.expect("methods");
    assert!(methods.iter().any(|m| m.key == "card"), "{methods:?}");
    let PaymentStart::Card { card, payment } = api.checkout(&t, &purchase, "card").await.expect("card checkout") else {
        panic!("card checkout answered another shape")
    };
    assert!(!card.card_number.is_empty());
    let payment = payment.expect("the payment to follow");
    let pending = api.pending_payments(&t).await.expect("pending");
    let listed = pending.iter().find(|p| p.payment_id == payment.payment_id).expect("the new payment is pending");
    assert_eq!(listed.card.as_ref().map(|c| c.card_number.as_str()), Some(card.card_number.as_str()));
    // Not an image: refused here, before the server spends anything on it.
    assert!(api.upload_receipt(&t, &payment.payment_id, b"not an image".to_vec()).await.is_err());

    let wallet = api.wallet(&t).await.expect("wallet");
    assert!(wallet.balance >= 0);
    api.wallet_transactions(&t, 1, 20).await.expect("transactions");
    let topup = api.topup(&t, 50_000, "card").await.expect("topup");
    assert!(matches!(topup, PaymentStart::Card { .. }), "{topup:?}");

    let referral = api.referral(&t).await.expect("referral");
    assert!(!referral.code.is_empty());
    api.trial_offer(&t).await.expect("trial offer");

    let ticket = api.open_ticket(&t, "connection", "", "از صبح روی ایرانسل وصل نمی‌شود.").await.expect("open ticket");
    let tickets = api.tickets(&t).await.expect("tickets");
    assert!(tickets.iter().any(|x| x.ticket_id == ticket.ticket_id));
    let reply = api.ticket_reply(&t, &ticket.ticket_id, "هنوز هم وصل نمی‌شود، حتی با وای‌فای.").await.expect("reply");
    assert!(!reply.from_support);
    let thread = api.ticket_messages(&t, &ticket.ticket_id).await.expect("thread");
    assert_eq!(thread.len(), 2, "{thread:?}");
}

fn geek_e2e_device_id() -> String {
    // Fresh per run so the per-device start limit (5 per 10 minutes) never bites.
    format!("{:064x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())
}
