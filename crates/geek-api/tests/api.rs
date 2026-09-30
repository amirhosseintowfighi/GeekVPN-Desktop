//! The client against a mock server that answers with fixtures generated from
//! GeekVPNBot's own response models (tests/fixtures, see README there).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use geek_api::{
    wait_for_approval, ApiClient, ApiError, DeviceInfo, LinkOutcome, Session, SessionError, SignedIn, TokenStore, Tokens,
};
use serde_json::{json, Value};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use url::Url;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture(name: &str) -> Value {
    let raw = std::fs::read_to_string(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn client(server: &MockServer) -> ApiClient {
    ApiClient::new(Url::parse(&format!("{}/", server.uri())).unwrap(), "GeekVPN-test").unwrap()
}

fn device() -> DeviceInfo {
    DeviceInfo {
        device_id: "a".repeat(64),
        device_name: "DESKTOP-7Q2".into(),
        platform: "windows".into(),
        app_version: "0.1.0".into(),
    }
}

#[derive(Default)]
struct MemoryStore {
    saved: Mutex<Option<SignedIn>>,
}

impl TokenStore for MemoryStore {
    fn load(&self) -> Result<Option<SignedIn>, String> {
        Ok(self.saved.lock().unwrap().clone())
    }
    fn save(&self, s: &SignedIn) -> Result<(), String> {
        *self.saved.lock().unwrap() = Some(s.clone());
        Ok(())
    }
    fn clear(&self) -> Result<(), String> {
        *self.saved.lock().unwrap() = None;
        Ok(())
    }
}

fn signed_in(access_valid_for: ChronoDuration) -> SignedIn {
    let approved: SignedIn = serde_json::from_value(fixture("password_ok.json")).unwrap();
    SignedIn {
        tokens: Tokens { access_expires_at: Utc::now() + access_valid_for, ..approved.tokens },
        user: approved.user,
    }
}

/// The refresh fixture with an expiry in the future, so the rotated token
/// counts as fresh whenever the test runs.
fn fresh_refresh() -> Value {
    let mut v = fixture("refresh.json");
    v["accessExpiresAt"] = json!((Utc::now() + ChronoDuration::minutes(15)).to_rfc3339());
    v
}

// -- sign-in ------------------------------------------------------------------

#[tokio::test]
async fn link_start_sends_exactly_the_fields_the_server_declares() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/app/auth/link/start"))
        // The server's request model forbids unknown fields and reads camelCase.
        .and(body_json(json!({
            "deviceId": "a".repeat(64),
            "deviceName": "DESKTOP-7Q2",
            "platform": "windows",
            "appVersion": "0.1.0",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("link_start.json")))
        .expect(1)
        .mount(&server)
        .await;

    let start = client(&server).link_start(&device()).await.unwrap();
    assert!(start.deep_link.starts_with("https://t.me/GeekVPNBot?start=applogin_"));
    assert_eq!(start.expires_in, 300);
}

#[tokio::test]
async fn the_poll_waits_through_pending_and_a_network_hiccup_until_approval() {
    let server = MockServer::start().await;
    let poll = || Mock::given(method("POST")).and(path("/api/app/auth/link/poll"));
    poll().respond_with(ResponseTemplate::new(200).set_body_json(fixture("poll_pending.json")))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    poll().respond_with(ResponseTemplate::new(503)).up_to_n_times(1).mount(&server).await;
    poll().respond_with(ResponseTemplate::new(200).set_body_json(fixture("poll_approved.json")))
        .mount(&server)
        .await;

    let outcome = wait_for_approval(
        &client(&server),
        "p",
        Instant::now() + Duration::from_secs(30),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    match outcome {
        LinkOutcome::Approved(s) => {
            assert_eq!(s.tokens.access_token, "acc-1");
            assert_eq!(s.user.telegram_id, 1011788123);
        }
        other => panic!("expected approval, got {other:?}"),
    }
}

#[tokio::test]
async fn a_denied_or_forgotten_request_ends_the_wait() {
    let server = MockServer::start().await;
    Mock::given(path("/api/app/auth/link/poll"))
        .and(body_json(json!({"pollToken": "denied", "wait": true})))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("poll_denied.json")))
        .mount(&server)
        .await;
    Mock::given(path("/api/app/auth/link/poll"))
        .and(body_json(json!({"pollToken": "unknown", "wait": true})))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let api = client(&server);
    let later = Instant::now() + Duration::from_secs(30);
    let never = CancellationToken::new();

    assert!(matches!(wait_for_approval(&api, "denied", later, &never).await.unwrap(), LinkOutcome::Denied));
    assert!(matches!(wait_for_approval(&api, "unknown", later, &never).await.unwrap(), LinkOutcome::Expired));
}

#[tokio::test]
async fn cancel_stops_a_poll_the_server_is_still_holding() {
    let server = MockServer::start().await;
    Mock::given(path("/api/app/auth/link/poll"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(20)).set_body_json(fixture("poll_pending.json")))
        .mount(&server)
        .await;
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        trigger.cancel();
    });

    let started = std::time::Instant::now();
    let outcome = wait_for_approval(&client(&server), "p", Instant::now() + Duration::from_secs(60), &cancel).await.unwrap();
    assert!(matches!(outcome, LinkOutcome::Cancelled));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn a_wrong_password_carries_the_servers_persian_message() {
    let server = MockServer::start().await;
    Mock::given(path("/api/app/auth/password"))
        .respond_with(ResponseTemplate::new(401).set_body_json(fixture("problem_401.json")))
        .mount(&server)
        .await;

    let err = client(&server).password_login("amir", "nope", &device()).await.unwrap_err();
    assert!(matches!(err, ApiError::Http { status: 401, .. }));
    assert_eq!(err.user_message(), "نام کاربری یا گذرواژه درست نیست.");
}

// -- the session ----------------------------------------------------------------

#[tokio::test]
async fn concurrent_callers_share_one_refresh() {
    let server = MockServer::start().await;
    // Exactly one: a second use of the same refresh token would make the
    // backend revoke the session as stolen.
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/refresh"))
        .and(body_json(json!({"refreshToken": "ref-1-xxxxxxxxxxxxxxxxxxxx"})))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(150)).set_body_json(fresh_refresh()))
        .expect(1)
        .mount(&server)
        .await;
    let store = Arc::new(MemoryStore::default());
    store.save(&signed_in(ChronoDuration::seconds(10))).unwrap();
    let session = Arc::new(Session::restore(client(&server), store.clone()));

    let tasks: Vec<_> = (0..5)
        .map(|_| {
            let s = session.clone();
            tokio::spawn(async move { s.access_token().await.unwrap() })
        })
        .collect();
    for t in tasks {
        assert_eq!(t.await.unwrap(), "acc-2");
    }
    // The rotated pair is what survives a restart.
    assert_eq!(store.load().unwrap().unwrap().tokens.refresh_token, "ref-2-yyyyyyyyyyyyyyyyyyyy");
}

#[tokio::test]
async fn a_rejected_refresh_signs_out_and_forgets_the_tokens() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v1/auth/refresh"))
        .respond_with(ResponseTemplate::new(401).set_body_json(fixture("problem_401.json")))
        .mount(&server)
        .await;
    let store = Arc::new(MemoryStore::default());
    store.save(&signed_in(ChronoDuration::seconds(-5))).unwrap();
    let session = Session::restore(client(&server), store.clone());

    let err = session.access_token().await.unwrap_err();
    assert!(matches!(err, SessionError::Api(ApiError::Unauthorized)));
    assert!(session.user().await.is_none());
    assert!(store.load().unwrap().is_none());
    assert!(matches!(session.access_token().await, Err(SessionError::SignedOut)));
}

#[tokio::test]
async fn a_401_on_a_fresh_token_gets_one_refresh_and_one_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/auth/me"))
        .and(header("authorization", "Bearer acc-1"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/auth/me"))
        .and(header("authorization", "Bearer acc-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("me.json")))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1/auth/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fresh_refresh()))
        .expect(1)
        .mount(&server)
        .await;
    let store = Arc::new(MemoryStore::default());
    store.save(&signed_in(ChronoDuration::minutes(10))).unwrap();
    let session = Session::restore(client(&server), store.clone());

    let user = session.refresh_user().await.unwrap();
    assert_eq!(user.display_name, "امیر");
    assert_eq!(store.load().unwrap().unwrap().user.display_name, "امیر");
}

#[tokio::test]
async fn sign_out_is_local_even_when_the_server_is_unreachable() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v1/auth/logout")).respond_with(ResponseTemplate::new(502)).mount(&server).await;
    let store = Arc::new(MemoryStore::default());
    store.save(&signed_in(ChronoDuration::minutes(10))).unwrap();
    let session = Session::restore(client(&server), store.clone());

    session.sign_out().await;
    assert!(session.user().await.is_none());
    assert!(store.load().unwrap().is_none());
}

#[tokio::test]
async fn subscriptions_read_the_mini_apps_camelcase_cards() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/miniapp/subscriptions"))
        .and(header("authorization", "Bearer acc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("subscriptions.json")))
        .mount(&server)
        .await;
    let cards = client(&server).subscriptions("acc-1").await.unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].subscription_url.as_deref(), Some("https://sub.example.com/sub/TOKEN"));
    assert_eq!(cards[0].tier.as_deref(), Some("direct"));
    assert_eq!(cards[0].quota_gib, Some(40.0));
}
