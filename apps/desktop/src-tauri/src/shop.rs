//! «فروشگاه», the wallet, the free trial, «دعوت از دوستان» and a service's
//! daily usage: GeekVPNBot's Mini App routes, called with the session.
//!
//! Only the server prices. The app shows the quote it is given and pays by
//! whatever `/payment-methods` offers; the wallet is offered by the screen,
//! as the Mini App does.

use geek_api::{
    CouponPreview, PaymentMethod, PaymentStart, PaymentView, Purchase, Quote, Referral, Storefront,
    TrialClaim, TrialOffer, UsageDay, Wallet, WalletTransactions, MAX_RECEIPT_BYTES,
};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

use crate::auth::{call, AuthState};

/// The bank page of the payment in progress. The UI may reopen it, never
/// open an address of its own.
#[derive(Default)]
pub struct ShopState {
    gateway: Mutex<Option<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopView {
    store: Storefront,
    methods: Vec<PaymentMethod>,
    /// None when the server could not say; the card then stays hidden.
    trial: Option<TrialOffer>,
    pending: Vec<PaymentView>,
}

#[tauri::command]
pub async fn shop_load(app: AppHandle, auth: State<'_, AuthState>) -> Result<ShopView, String> {
    let (store, methods, trial, pending) = tokio::join!(
        call(
            &app,
            &auth,
            |api, t| async move { api.storefront(&t).await }
        ),
        call(&app, &auth, |api, t| async move {
            api.payment_methods(&t).await
        }),
        call(
            &app,
            &auth,
            |api, t| async move { api.trial_offer(&t).await }
        ),
        call(&app, &auth, |api, t| async move {
            api.pending_payments(&t).await
        }),
    );
    Ok(ShopView {
        store: store?,
        methods: methods?,
        trial: trial.ok(),
        pending: pending.unwrap_or_default(),
    })
}

fn purchase(plan_id: String, coupon: Option<String>, renews: Option<String>) -> Purchase {
    Purchase {
        plan_id,
        coupon_code: coupon
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty()),
        renews_subscription_id: renews.filter(|r| !r.is_empty()),
    }
}

#[tauri::command]
pub async fn shop_quote(
    app: AppHandle,
    auth: State<'_, AuthState>,
    plan_id: String,
    coupon: Option<String>,
    renews: Option<String>,
) -> Result<Quote, String> {
    let p = purchase(plan_id, coupon, renews);
    call(&app, &auth, |api, t| {
        let p = p.clone();
        async move { api.quote(&t, &p).await }
    })
    .await
}

#[tauri::command]
pub async fn shop_coupon(
    app: AppHandle,
    auth: State<'_, AuthState>,
    plan_id: String,
    code: String,
) -> Result<CouponPreview, String> {
    let code = code.trim().to_string();
    if code.is_empty() || code.len() > 64 {
        return Err("کد تخفیف را درست وارد کن.".into());
    }
    call(&app, &auth, |api, t| {
        let (plan_id, code) = (plan_id.clone(), code.clone());
        async move { api.coupon_preview(&t, &plan_id, &code).await }
    })
    .await
}

/// A gateway's page opens in the browser at once; everything else is shown
/// by the UI.
async fn follow(
    app: &AppHandle,
    shop: &ShopState,
    start: PaymentStart,
) -> Result<PaymentStart, String> {
    if let PaymentStart::Gateway { url, .. } = &start {
        if !url.is_empty() {
            if !safe_url(url) {
                return Err("آدرس درگاه پرداخت معتبر نیست. به پشتیبانی خبر بده.".into());
            }
            *shop.gateway.lock().await = Some(url.clone());
            let _ = app.opener().open_url(url, None::<&str>);
        }
    }
    Ok(start)
}

/// Only a web page: a gateway link is handed to the OS, which would run
/// any other scheme's handler.
fn safe_url(u: &str) -> bool {
    url::Url::parse(u)
        .is_ok_and(|u| u.scheme() == "https" || (cfg!(debug_assertions) && u.scheme() == "http"))
}

#[tauri::command]
pub async fn shop_checkout(
    app: AppHandle,
    auth: State<'_, AuthState>,
    shop: State<'_, ShopState>,
    plan_id: String,
    coupon: Option<String>,
    renews: Option<String>,
    method: String,
) -> Result<PaymentStart, String> {
    if !is_method_key(&method) {
        return Err("روش پرداخت را انتخاب کن.".into());
    }
    let p = purchase(plan_id, coupon, renews);
    let start = call(&app, &auth, |api, t| {
        let (p, method) = (p.clone(), method.clone());
        async move { api.checkout(&t, &p, &method).await }
    })
    .await?;
    follow(&app, &shop, start).await
}

/// The server's own pattern for a method key (`^[a-z0-9_]+$`, 2–32).
fn is_method_key(k: &str) -> bool {
    (2..=32).contains(&k.len())
        && k.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[tauri::command]
pub async fn shop_open_gateway(app: AppHandle, shop: State<'_, ShopState>) -> Result<(), String> {
    let url = shop
        .gateway
        .lock()
        .await
        .clone()
        .ok_or("پرداختی در جریان نیست.")?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn shop_pending(
    app: AppHandle,
    auth: State<'_, AuthState>,
) -> Result<Vec<PaymentView>, String> {
    call(&app, &auth, |api, t| async move {
        api.pending_payments(&t).await
    })
    .await
}

/// The receipt photo the customer picked in the file dialog.
#[tauri::command]
pub async fn shop_receipt(
    app: AppHandle,
    auth: State<'_, AuthState>,
    payment_id: String,
    path: String,
) -> Result<(), String> {
    let meta = std::fs::metadata(&path).map_err(|_| "فایل رسید پیدا نشد.".to_string())?;
    if meta.len() > MAX_RECEIPT_BYTES as u64 {
        return Err("عکس رسید بزرگ‌تر از ۸ مگابایت است.".into());
    }
    let image = std::fs::read(&path).map_err(|_| "فایل رسید خوانده نشد.".to_string())?;
    call(&app, &auth, |api, t| {
        let (id, image) = (payment_id.clone(), image.clone());
        async move { api.upload_receipt(&t, &id, image).await }
    })
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn shop_txid(
    app: AppHandle,
    auth: State<'_, AuthState>,
    payment_id: String,
    txid: String,
) -> Result<(), String> {
    let txid = txid.trim().to_string();
    if !(8..=256).contains(&txid.len()) {
        return Err("شناسه‌ی تراکنش (TXID) را کامل وارد کن.".into());
    }
    call(&app, &auth, |api, t| {
        let (id, txid) = (payment_id.clone(), txid.clone());
        async move { api.attach_txid(&t, &id, &txid).await }
    })
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn trial_claim(app: AppHandle, auth: State<'_, AuthState>) -> Result<TrialClaim, String> {
    call(
        &app,
        &auth,
        |api, t| async move { api.trial_claim(&t).await },
    )
    .await
}

#[tauri::command]
pub async fn wallet_load(app: AppHandle, auth: State<'_, AuthState>) -> Result<Wallet, String> {
    call(&app, &auth, |api, t| async move { api.wallet(&t).await }).await
}

#[tauri::command]
pub async fn wallet_transactions(
    app: AppHandle,
    auth: State<'_, AuthState>,
    page: u32,
) -> Result<WalletTransactions, String> {
    let page = page.max(1);
    call(&app, &auth, |api, t| async move {
        api.wallet_transactions(&t, page, 20).await
    })
    .await
}

/// The Mini App's floor for a top-up; the server enforces its own.
const MIN_TOPUP: i64 = 10_000;

#[tauri::command]
pub async fn wallet_topup(
    app: AppHandle,
    auth: State<'_, AuthState>,
    shop: State<'_, ShopState>,
    amount: i64,
    method: String,
) -> Result<PaymentStart, String> {
    if amount < MIN_TOPUP {
        return Err("حداقل مبلغ افزایش موجودی ۱۰٬۰۰۰ تومان است.".into());
    }
    if !is_method_key(&method) || method == "wallet" {
        return Err("روش پرداخت را انتخاب کن.".into());
    }
    let start = call(&app, &auth, |api, t| {
        let method = method.clone();
        async move { api.topup(&t, amount, &method).await }
    })
    .await?;
    follow(&app, &shop, start).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralView {
    summary: Referral,
    /// `https://t.me/<bot>?start=ref_<code>`; None in a build without the
    /// bot's name.
    link: Option<String>,
    qr_svg: Option<String>,
}

pub fn bot_username() -> Option<&'static str> {
    Some(env!("GEEK_BOT_USERNAME")).filter(|b| !b.is_empty())
}

fn invite_link(bot: &str, code: &str) -> String {
    format!("https://t.me/{bot}?start=ref_{code}")
}

#[tauri::command]
pub async fn referral_load(
    app: AppHandle,
    auth: State<'_, AuthState>,
) -> Result<ReferralView, String> {
    let summary = call(&app, &auth, |api, t| async move { api.referral(&t).await }).await?;
    let link = bot_username()
        .filter(|_| !summary.code.is_empty())
        .map(|b| invite_link(b, &summary.code));
    let qr_svg = link.as_deref().map(crate::auth::qr_svg).transpose()?;
    Ok(ReferralView {
        summary,
        link,
        qr_svg,
    })
}

#[tauri::command]
pub async fn usage_service(
    app: AppHandle,
    auth: State<'_, AuthState>,
    subscription_id: String,
    days: u32,
) -> Result<Vec<UsageDay>, String> {
    call(&app, &auth, |api, t| {
        let id = subscription_id.clone();
        async move { api.usage_days(&t, &id, days).await }
    })
    .await
}

#[cfg(test)]
mod tests {
    #[test]
    fn keys_links_and_urls() {
        assert!(super::is_method_key("card") && super::is_method_key("zarinpal_2"));
        assert!(
            !super::is_method_key("../x")
                && !super::is_method_key("A")
                && !super::is_method_key("")
        );
        assert_eq!(
            super::invite_link("GeekVpnBot", "AB12"),
            "https://t.me/GeekVpnBot?start=ref_AB12"
        );
        assert!(super::safe_url("https://pay.example.com/x"));
        assert!(!super::safe_url("file:///etc/passwd") && !super::safe_url("tg://resolve"));
    }
}
