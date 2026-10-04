//! GeekVPN's backend (GeekVPNBot) as the desktop app sees it.
//!
//! Sign-in follows `presentation/api/routers/app_auth.py`: the bot approves
//! the device, then the app is an ordinary Bearer client that rotates its
//! tokens at `/api/v1/auth/refresh` and calls the Mini App's routes.

mod bootstrap;
mod client;
mod error;
mod login;
mod miniapp;
mod session;
mod types;

pub use client::ApiClient;
pub use error::ApiError;
pub use miniapp::{
    receipt_type, CardInfo, CouponPreview, PaymentMethod, PaymentStart, PaymentView, PendingPayment, Purchase, Quote, QuoteLine,
    Referral, StoreCategory, StorePlan, StoreProduct, Storefront, Ticket, TicketMessage, TrialClaim, TrialOffer, UsageDay, Wallet,
    WalletTransaction, WalletTransactions, MAX_RECEIPT_BYTES,
};
pub use login::{telegram_app_link, wait_for_approval, LinkOutcome};
pub use session::{Session, SessionError, TokenStore};
pub use types::{AppUser, DeviceInfo, LinkPoll, LinkStart, LinkStatus, SignedIn, SubscriptionCard, Tokens};
