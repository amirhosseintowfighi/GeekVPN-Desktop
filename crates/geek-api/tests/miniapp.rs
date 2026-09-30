//! The Mini App routes against fixtures serialised from GeekVPNBot's own read
//! models (tests/fixtures/miniapp_fixtures.py).

use geek_api::{ApiClient, ApiError, PaymentStart, Purchase};
use serde_json::{json, Value};
use url::Url;
use wiremock::matchers::{body_bytes, body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture(name: &str) -> Value {
    let raw = std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn client(server: &MockServer) -> ApiClient {
    ApiClient::new(
        Url::parse(&format!("{}/", server.uri())).unwrap(),
        "GeekVPN-test",
    )
    .unwrap()
}

async fn serve(server: &MockServer, verb: &str, route: &str, body: Value) {
    Mock::given(method(verb))
        .and(path(route))
        .and(header("authorization", "Bearer T"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

fn purchase() -> Purchase {
    Purchase {
        plan_id: "00000000-0000-0000-0000-000000000002".into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn storefront_quote_and_coupon_read_the_servers_shapes() {
    let server = MockServer::start().await;
    serve(
        &server,
        "GET",
        "/api/miniapp/storefront",
        fixture("storefront.json"),
    )
    .await;
    Mock::given(method("POST"))
        .and(path("/api/miniapp/quote"))
        // No coupon, no renewal: the request model forbids unknown fields,
        // so empty ones are left out rather than sent as null.
        .and(body_json(
            json!({ "planId": "00000000-0000-0000-0000-000000000002" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("quote.json")))
        .expect(1)
        .mount(&server)
        .await;
    serve(
        &server,
        "POST",
        "/api/miniapp/coupon/preview",
        fixture("coupon_rejected.json"),
    )
    .await;
    let api = client(&server);

    let store = api.storefront("T").await.unwrap();
    assert_eq!(store.wallet_balance, 50000);
    let direct = &store.categories[0].products[0];
    assert_eq!(direct.tier, "direct");
    assert_eq!(direct.plans.len(), 3);
    assert_eq!(direct.plans[1].compare_at_price, Some(180000));
    assert_eq!(direct.plans[2].quota_gib, None, "unlimited");

    let quote = api.quote("T", &purchase()).await.unwrap();
    assert_eq!((quote.total, quote.discount_percent), (150000, 16));
    assert!(quote.lines[1].is_deduction);

    let coupon = api
        .coupon_preview("T", &purchase().plan_id, "NOPE")
        .await
        .unwrap();
    assert!(!coupon.is_valid);
    assert_eq!(coupon.message_fa, "این کد معتبر نیست.");
}

#[tokio::test]
async fn each_checkout_route_answers_its_own_shape() {
    let server = MockServer::start().await;
    serve(
        &server,
        "POST",
        "/api/miniapp/checkout/wallet",
        fixture("checkout_wallet.json"),
    )
    .await;
    serve(
        &server,
        "POST",
        "/api/miniapp/checkout/card",
        fixture("checkout_card.json"),
    )
    .await;
    Mock::given(method("POST"))
        .and(path("/api/miniapp/checkout/gateway"))
        .and(body_json(json!({
            "planId": "00000000-0000-0000-0000-000000000002",
            "couponCode": "OFF20",
            "renewsSubscriptionId": "sub-1",
            "gatewayKey": "zarinpal",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("checkout_gateway.json")))
        .expect(1)
        .mount(&server)
        .await;
    let api = client(&server);

    let done = api.checkout("T", &purchase(), "wallet").await.unwrap();
    assert_eq!(
        done,
        PaymentStart::Done {
            subscription_id: "00000000-0000-0000-0000-000000000009".into()
        }
    );

    let PaymentStart::Card { card, payment } =
        api.checkout("T", &purchase(), "card").await.unwrap()
    else {
        panic!()
    };
    assert_eq!(card.card_number, "6037991234567890");
    let payment = payment.unwrap();
    assert_eq!(
        (payment.state.as_str(), payment.amount),
        ("awaiting_proof", 150000)
    );

    let renewing = Purchase {
        coupon_code: Some("OFF20".into()),
        renews_subscription_id: Some("sub-1".into()),
        ..purchase()
    };
    let PaymentStart::Gateway {
        url, payment_id, ..
    } = api.checkout("T", &renewing, "zarinpal").await.unwrap()
    else {
        panic!()
    };
    assert_eq!(url, "https://pay.example.com/start/abc");
    assert!(payment_id.is_some());
}

#[tokio::test]
async fn a_receipt_goes_up_as_the_raw_image_with_its_type() {
    let server = MockServer::start().await;
    let png = b"\x89PNG\r\n\x1a\nrest-of-image".to_vec();
    Mock::given(method("POST"))
        .and(path(
            "/api/miniapp/payments/00000000-0000-0000-0000-00000000004d/receipt-photo",
        ))
        .and(header("content-type", "image/png"))
        .and(body_bytes(png.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let api = client(&server);
    api.upload_receipt("T", "00000000-0000-0000-0000-00000000004d", png)
        .await
        .unwrap();

    // Not an image, or an id that would change the route: refused here.
    let err = api
        .upload_receipt(
            "T",
            "00000000-0000-0000-0000-00000000004d",
            b"%PDF-1.7".to_vec(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ApiError::Http { status: 415, .. }));
    assert!(api
        .upload_receipt("T", "../wallet", b"\xFF\xD8\xFFx".to_vec())
        .await
        .is_err());
}

#[tokio::test]
async fn wallet_trial_referral_and_usage() {
    let server = MockServer::start().await;
    serve(
        &server,
        "GET",
        "/api/miniapp/wallet",
        fixture("wallet.json"),
    )
    .await;
    Mock::given(method("GET"))
        .and(path("/api/miniapp/wallet/transactions"))
        .and(query_param("page", "2"))
        .and(query_param("page_size", "20"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("wallet_transactions.json")))
        .expect(1)
        .mount(&server)
        .await;
    serve(&server, "GET", "/api/miniapp/trial", fixture("trial.json")).await;
    serve(
        &server,
        "GET",
        "/api/miniapp/referral",
        fixture("referral.json"),
    )
    .await;
    Mock::given(method("GET"))
        .and(path(
            "/api/miniapp/subscriptions/00000000-0000-0000-0000-000000000009/usage-days",
        ))
        .and(query_param("days", "60"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("usage_days.json")))
        .expect(1)
        .mount(&server)
        .await;
    let api = client(&server);

    assert_eq!(api.wallet("T").await.unwrap().balance, 50000);
    let tx = api.wallet_transactions("T", 2, 20).await.unwrap();
    assert_eq!((tx.total, tx.items[0].kind.as_str()), (1, "topup"));
    let trial = api.trial_offer("T").await.unwrap();
    assert!(trial.available);
    assert_eq!((trial.traffic_mib, trial.duration_days), (50, 2));
    let referral = api.referral("T").await.unwrap();
    assert_eq!(
        (
            referral.code.as_str(),
            referral.first_purchase_bps,
            referral.pending_earned
        ),
        ("K7Q2MX", 1000, 15000)
    );
    // The server allows 1..=60 days.
    let days = api
        .usage_days("T", "00000000-0000-0000-0000-000000000009", 365)
        .await
        .unwrap();
    assert_eq!(days[0].used_mib, 1536);
}

#[tokio::test]
async fn tickets_threads_and_replies() {
    let server = MockServer::start().await;
    serve(
        &server,
        "GET",
        "/api/miniapp/tickets",
        fixture("tickets.json"),
    )
    .await;
    let thread = "/api/miniapp/tickets/00000000-0000-0000-0000-000000000028/messages";
    serve(&server, "GET", thread, fixture("ticket_messages.json")).await;
    Mock::given(method("POST"))
        .and(path(thread))
        .and(body_json(json!({ "message": "هنوز وصل نمی‌شود." })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixture("ticket_messages.json")[1].clone()),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/miniapp/tickets"))
        .and(body_json(
            json!({ "topic": "connection", "subject": "", "message": "از صبح وصل نمی‌شود." }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("tickets.json")[0].clone()))
        .expect(1)
        .mount(&server)
        .await;
    let api = client(&server);

    let tickets = api.tickets("T").await.unwrap();
    assert_eq!(
        (tickets[0].state.as_str(), tickets[0].unread_count),
        ("waiting", 2)
    );
    let messages = api
        .ticket_messages("T", &tickets[0].ticket_id)
        .await
        .unwrap();
    assert!(!messages[0].from_support && messages[1].from_support);
    api.ticket_reply("T", &tickets[0].ticket_id, "هنوز وصل نمی‌شود.")
        .await
        .unwrap();
    let opened = api
        .open_ticket("T", "connection", "", "از صبح وصل نمی‌شود.")
        .await
        .unwrap();
    assert_eq!(opened.reference, "SUP-1405-000040");
}
