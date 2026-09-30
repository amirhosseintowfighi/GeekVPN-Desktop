//! The Mini App's routes the desktop app uses beyond sign-in: the shop and
//! its payments, the wallet, the free trial, referral, support tickets and
//! a service's daily usage (`presentation/api/routers/miniapp.py`).
//!
//! Shapes follow the backend's read models, camelCased on the wire. Fields
//! the server may leave out or null are `Option` or defaulted, so a new
//! backend field never breaks an old app, and a missing one reads as absent
//! rather than as a made-up zero. Amounts are whole tomans.

use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::{send_session, ApiClient};
use crate::error::ApiError;

// -- shop ---------------------------------------------------------------------

/// `/storefront`: categories, their products, their priced plans.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Storefront {
    pub categories: Vec<StoreCategory>,
    pub wallet_balance: i64,
    #[serde(default)]
    pub loyalty_tier: String,
    #[serde(default)]
    pub is_first_purchase: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreCategory {
    pub category_id: String,
    pub name_fa: String,
    pub icon: Option<String>,
    pub products: Vec<StoreProduct>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreProduct {
    pub product_id: String,
    /// direct | tunnel | elite
    pub tier: String,
    pub name_fa: String,
    pub tagline_fa: Option<String>,
    pub description_fa: Option<String>,
    #[serde(default)]
    pub features_fa: Vec<String>,
    pub badge_fa: Option<String>,
    #[serde(default)]
    pub is_featured: bool,
    pub plans: Vec<StorePlan>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorePlan {
    pub plan_id: String,
    pub name_fa: String,
    #[serde(default)]
    pub plan_type: String,
    pub duration_days: i64,
    pub price: i64,
    /// Only where a real discount exists.
    pub compare_at_price: Option<i64>,
    /// None: unlimited.
    pub quota_gib: Option<i64>,
    pub daily_quota_gib: Option<i64>,
    #[serde(default)]
    pub device_limit: i64,
    pub badge_fa: Option<String>,
    #[serde(default)]
    pub is_featured: bool,
    pub description_fa: Option<String>,
}

/// `/quote`: what this customer pays today. Only the server prices.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub plan_id: String,
    pub base_price: i64,
    pub total: i64,
    #[serde(default)]
    pub total_discount: i64,
    #[serde(default)]
    pub discount_percent: i64,
    #[serde(default)]
    pub cashback: i64,
    #[serde(default)]
    pub lines: Vec<QuoteLine>,
    pub compare_at_price: Option<i64>,
    pub campaign_label: Option<String>,
    pub coupon_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteLine {
    pub kind: String,
    pub label: String,
    pub amount: i64,
    #[serde(default)]
    pub is_deduction: bool,
}

/// `/coupon/preview`. A refusal is data: `is_valid` false and the reason in
/// `message_fa`, not an HTTP error.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CouponPreview {
    pub code: String,
    pub is_valid: bool,
    #[serde(default)]
    pub discount: i64,
    #[serde(default)]
    pub total_after: i64,
    #[serde(default)]
    pub message_fa: String,
}

/// One of `/payment-methods`: `card`, `crypto` or an online gateway's key.
/// The wallet is never listed; the screen offers it itself.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
    pub key: String,
    pub label_fa: String,
}

/// A payment that still waits for proof or for review.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingPayment {
    pub payment_id: String,
    /// The short handle the customer quotes to support.
    pub reference: String,
    pub amount: i64,
    /// wallet | card | crypto | gateway
    pub method: String,
    /// awaiting_proof | pending_review | approved | rejected | cancelled | …
    pub state: String,
    pub created_at: Option<DateTime<Utc>>,
}

/// `/payments/pending`: the same, with the card to pay to.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentView {
    pub payment_id: String,
    pub reference: String,
    pub amount: i64,
    pub method: String,
    pub state: String,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub card: Option<CardInfo>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardInfo {
    pub card_number: String,
    #[serde(default)]
    pub card_holder_fa: String,
    #[serde(default)]
    pub bank_fa: String,
    #[serde(default)]
    pub review_sla_fa: String,
}

/// What starting a payment answers. The server sends one of several shapes
/// that share no field; this is the one that arrived.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PaymentStart {
    /// Paid from the wallet: the service exists already.
    Done { subscription_id: String },
    /// Card to card: transfer, then send the receipt.
    Card {
        card: CardInfo,
        payment: Option<PendingPayment>,
    },
    /// An online gateway: a bank page to open, instructions to read, or both.
    Gateway {
        url: String,
        body_fa: String,
        payment_id: Option<String>,
    },
    /// A crypto address; the transaction hash is the proof.
    Crypto {
        network: String,
        asset: String,
        amount_display: String,
        address: String,
        payment: Option<PendingPayment>,
    },
}

impl PaymentStart {
    /// Tells the shapes apart by the fields only each one has.
    pub fn from_value(v: Value) -> Result<Self, ApiError> {
        let decode = |e: serde_json::Error| ApiError::Decode(e.to_string());
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        if let Some(id) = s("subscriptionId") {
            return Ok(Self::Done {
                subscription_id: id,
            });
        }
        let payment = match v.get("payment") {
            Some(p) if !p.is_null() => Some(serde_json::from_value(p.clone()).map_err(decode)?),
            _ => None,
        };
        if v.get("cardNumber").is_some() {
            return Ok(Self::Card {
                card: serde_json::from_value(v.clone()).map_err(decode)?,
                payment,
            });
        }
        if let Some(address) = s("address") {
            return Ok(Self::Crypto {
                network: s("network").unwrap_or_default(),
                asset: s("asset").unwrap_or_default(),
                amount_display: s("amountDisplay").unwrap_or_default(),
                address,
                payment,
            });
        }
        let (url, body_fa) = (
            s("url").unwrap_or_default(),
            s("bodyFa").unwrap_or_default(),
        );
        if !url.is_empty() || !body_fa.is_empty() {
            return Ok(Self::Gateway {
                url,
                body_fa,
                payment_id: s("paymentId"),
            });
        }
        Err(ApiError::Decode("unknown payment shape".into()))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanRequest<'a> {
    plan_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    coupon_code: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    renews_subscription_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gateway_key: Option<&'a str>,
}

/// What a checkout buys: a plan, maybe with a coupon, maybe renewing a
/// service the customer already has.
#[derive(Debug, Clone, Default)]
pub struct Purchase {
    pub plan_id: String,
    pub coupon_code: Option<String>,
    pub renews_subscription_id: Option<String>,
}

impl Purchase {
    fn request<'a>(&'a self, gateway_key: Option<&'a str>) -> PlanRequest<'a> {
        PlanRequest {
            plan_id: &self.plan_id,
            coupon_code: self.coupon_code.as_deref().filter(|c| !c.trim().is_empty()),
            renews_subscription_id: self.renews_subscription_id.as_deref(),
            gateway_key,
        }
    }
}

// -- wallet, trial, referral, usage -------------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    pub balance: i64,
    #[serde(default)]
    pub lifetime_spend: i64,
    #[serde(default)]
    pub pending_credit: i64,
    #[serde(default)]
    pub tier: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletTransaction {
    pub transaction_id: String,
    /// topup | purchase | cashback | referral | refund | adjustment | …
    pub kind: String,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub description_fa: String,
    pub balance_after: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletTransactions {
    pub items: Vec<WalletTransaction>,
    pub page: u32,
    pub page_size: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialOffer {
    pub available: bool,
    pub traffic_mib: i64,
    pub duration_days: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialClaim {
    pub subscription_ids: Vec<String>,
    /// Services whose panel account is still being made; a later refresh
    /// brings them.
    #[serde(default)]
    pub pending: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Referral {
    pub code: String,
    #[serde(default)]
    pub invited_count: i64,
    /// Invitees who bought something.
    #[serde(default)]
    pub converted_count: i64,
    #[serde(default)]
    pub total_earned: i64,
    #[serde(default)]
    pub pending_earned: i64,
    /// What the invitee gets for joining.
    #[serde(default)]
    pub invitee_bonus: i64,
    /// Basis points (1% = 100) of the invitee's first purchase.
    #[serde(default)]
    pub first_purchase_bps: i64,
    /// Basis points of each later purchase.
    #[serde(default)]
    pub recurring_bps: i64,
}

/// One day of a service's traffic, all its devices together (Tehran days).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDay {
    /// `YYYY-MM-DD`.
    pub day: String,
    pub used_mib: i64,
}

// -- support ------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticket {
    pub ticket_id: String,
    /// Printed in every bot message about the ticket.
    pub reference: String,
    #[serde(default)]
    pub topic_fa: String,
    /// open | waiting (the customer's turn) | answered | closed
    pub state: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_message_fa: String,
    #[serde(default)]
    pub unread_count: u32,
    pub last_reply_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketMessage {
    pub message_id: String,
    #[serde(default)]
    pub from_support: bool,
    pub body_fa: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct OpenTicket<'a> {
    topic: &'a str,
    subject: &'a str,
    message: &'a str,
}

#[derive(Serialize)]
struct Message<'a> {
    message: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Topup<'a> {
    amount: i64,
    method: &'a str,
}

#[derive(Serialize)]
struct Txid<'a> {
    txid: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CouponRequest<'a> {
    plan_id: &'a str,
    code: &'a str,
}

/// A receipt photo's type, from its first bytes: the server takes JPEG, PNG
/// and WebP, up to 8 MiB.
pub fn receipt_type(image: &[u8]) -> Option<&'static str> {
    if image.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if image.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if image.len() >= 12 && &image[..4] == b"RIFF" && &image[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

pub const MAX_RECEIPT_BYTES: usize = 8 * 1024 * 1024;

/// Path segments are ids the server handed out; anything else is refused
/// before it can change which route a request reaches.
fn id(value: &str) -> Result<&str, ApiError> {
    if !value.is_empty()
        && value.len() <= 64
        && value.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        Ok(value)
    } else {
        Err(ApiError::Decode(format!("not an id: {value:?}")))
    }
}

impl ApiClient {
    pub async fn storefront(&self, token: &str) -> Result<Storefront, ApiError> {
        self.get_json("api/miniapp/storefront", token).await
    }

    pub async fn quote(&self, token: &str, purchase: &Purchase) -> Result<Quote, ApiError> {
        self.post_json("api/miniapp/quote", token, &purchase.request(None))
            .await
    }

    pub async fn coupon_preview(
        &self,
        token: &str,
        plan_id: &str,
        code: &str,
    ) -> Result<CouponPreview, ApiError> {
        self.post_json(
            "api/miniapp/coupon/preview",
            token,
            &CouponRequest { plan_id, code },
        )
        .await
    }

    pub async fn payment_methods(&self, token: &str) -> Result<Vec<PaymentMethod>, ApiError> {
        self.get_json("api/miniapp/payment-methods", token).await
    }

    /// Pays by `method`: `wallet`, `card`, `crypto`, or a gateway's key.
    pub async fn checkout(
        &self,
        token: &str,
        purchase: &Purchase,
        method: &str,
    ) -> Result<PaymentStart, ApiError> {
        let v: Value = match method {
            "wallet" | "card" | "crypto" => {
                self.post_json(
                    &format!("api/miniapp/checkout/{method}"),
                    token,
                    &purchase.request(None),
                )
                .await?
            }
            key => {
                self.post_json(
                    "api/miniapp/checkout/gateway",
                    token,
                    &purchase.request(Some(key)),
                )
                .await?
            }
        };
        PaymentStart::from_value(v)
    }

    pub async fn pending_payments(&self, token: &str) -> Result<Vec<PaymentView>, ApiError> {
        self.get_json("api/miniapp/payments/pending", token).await
    }

    /// The Android app's receipt upload: the image is the body, its type the
    /// header. The bot forwards it to the operator for review.
    pub async fn upload_receipt(
        &self,
        token: &str,
        payment_id: &str,
        image: Vec<u8>,
    ) -> Result<Value, ApiError> {
        if image.len() > MAX_RECEIPT_BYTES {
            return Err(ApiError::Http {
                status: 413,
                title: "too large".into(),
                message_fa: Some("عکس رسید بزرگ‌تر از ۸ مگابایت است.".into()),
            });
        }
        let Some(kind) = receipt_type(&image) else {
            return Err(ApiError::Http {
                status: 415,
                title: "unsupported".into(),
                message_fa: Some("رسید باید عکس JPEG، PNG یا WebP باشد.".into()),
            });
        };
        let req = self
            .request(
                Method::POST,
                &format!("api/miniapp/payments/{}/receipt-photo", id(payment_id)?),
            )?
            .bearer_auth(token)
            .header("content-type", kind)
            .timeout(std::time::Duration::from_secs(90))
            .body(image);
        send_session(req).await
    }

    pub async fn attach_txid(
        &self,
        token: &str,
        payment_id: &str,
        txid: &str,
    ) -> Result<Value, ApiError> {
        self.post_json(
            &format!("api/miniapp/payments/{}/txid", id(payment_id)?),
            token,
            &Txid { txid },
        )
        .await
    }

    pub async fn wallet(&self, token: &str) -> Result<Wallet, ApiError> {
        self.get_json("api/miniapp/wallet", token).await
    }

    pub async fn wallet_transactions(
        &self,
        token: &str,
        page: u32,
        page_size: u32,
    ) -> Result<WalletTransactions, ApiError> {
        self.get_json(
            &format!("api/miniapp/wallet/transactions?page={page}&page_size={page_size}"),
            token,
        )
        .await
    }

    /// Tops the wallet up by `method` (`card`, `crypto` or a gateway's key).
    pub async fn topup(
        &self,
        token: &str,
        amount: i64,
        method: &str,
    ) -> Result<PaymentStart, ApiError> {
        let v: Value = self
            .post_json("api/miniapp/wallet/topup", token, &Topup { amount, method })
            .await?;
        PaymentStart::from_value(v)
    }

    pub async fn trial_offer(&self, token: &str) -> Result<TrialOffer, ApiError> {
        self.get_json("api/miniapp/trial", token).await
    }

    pub async fn trial_claim(&self, token: &str) -> Result<TrialClaim, ApiError> {
        self.post_json("api/miniapp/trial", token, &serde_json::json!({}))
            .await
    }

    pub async fn referral(&self, token: &str) -> Result<Referral, ApiError> {
        self.get_json("api/miniapp/referral", token).await
    }

    pub async fn usage_days(
        &self,
        token: &str,
        subscription_id: &str,
        days: u32,
    ) -> Result<Vec<UsageDay>, ApiError> {
        let days = days.clamp(1, 60);
        self.get_json(
            &format!(
                "api/miniapp/subscriptions/{}/usage-days?days={days}",
                id(subscription_id)?
            ),
            token,
        )
        .await
    }

    pub async fn tickets(&self, token: &str) -> Result<Vec<Ticket>, ApiError> {
        self.get_json("api/miniapp/tickets", token).await
    }

    /// `topic` is a category key (connection, payment, account, speed,
    /// other); a non-empty `subject` becomes the ticket's title.
    pub async fn open_ticket(
        &self,
        token: &str,
        topic: &str,
        subject: &str,
        message: &str,
    ) -> Result<Ticket, ApiError> {
        self.post_json(
            "api/miniapp/tickets",
            token,
            &OpenTicket {
                topic,
                subject,
                message,
            },
        )
        .await
    }

    pub async fn ticket_messages(
        &self,
        token: &str,
        ticket_id: &str,
    ) -> Result<Vec<TicketMessage>, ApiError> {
        self.get_json(
            &format!("api/miniapp/tickets/{}/messages", id(ticket_id)?),
            token,
        )
        .await
    }

    pub async fn ticket_reply(
        &self,
        token: &str,
        ticket_id: &str,
        message: &str,
    ) -> Result<TicketMessage, ApiError> {
        self.post_json(
            &format!("api/miniapp/tickets/{}/messages", id(ticket_id)?),
            token,
            &Message { message },
        )
        .await
    }
}
