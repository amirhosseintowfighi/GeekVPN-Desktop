# Serialises GeekVPNBot's own read models exactly as the Mini App router does.
import json, sys, uuid
from datetime import datetime, timezone
from fastapi.encoders import jsonable_encoder
from geekvpn.presentation.api.routers import miniapp as M
from geekvpn.application.bot import read_models as R
from geekvpn.application.catalog.dto import QuoteView, PriceLineView, CouponPreview
from geekvpn.domain.payments.enums import PaymentMethod, PaymentState, TransactionKind
from geekvpn.domain.catalog.rewards import LoyaltyTier
from geekvpn.application.bot.read_models import TicketState as CardTicketState
import dataclasses

out = sys.argv[1]
T = datetime(2026, 9, 27, 10, 30, tzinfo=timezone.utc)
U = lambda n: uuid.UUID(int=n)
def camel(obj): return M._camelize(jsonable_encoder(obj))
def model(m): return m.model_dump(mode="json", by_alias=True)
def w(name, v):
    with open(f"{out}/{name}", "w") as f:
        json.dump(v, f, ensure_ascii=False, indent=2); f.write("\n")

plan = lambda n, days, gib, price, cmp=None: M.PlanCard(plan_id=U(n), product_id=U(100), name_fa=f"{days} روزه", plan_type="volume", duration_days=days, price=price, compare_at_price=cmp, quota_gib=gib, daily_quota_gib=None, device_limit=2, badge_fa="پرفروش" if n == 2 else None, is_featured=n == 2, description_fa=None)
w("storefront.json", model(M.StorefrontResponse(categories=[M.CategoryCard(category_id=U(10), name_fa="سرویس‌ها", icon=None, products=[
    M.ProductCard(product_id=U(100), category_id=U(10), tier="direct", name_fa="مستقیم", tagline_fa="برای اسکنر کلادفلر", description_fa=None, features_fa=["IP تمیز"], icon=None, badge_fa=None, is_featured=True,
                  plans=[plan(1, 30, 20, 90000), plan(2, 30, 40, 150000, 180000), plan(3, 90, None, 400000)]),
    M.ProductCard(product_id=U(101), category_id=U(10), tier="tunnel", name_fa="تونل", tagline_fa=None, description_fa=None, features_fa=[], icon=None, badge_fa=None, is_featured=False, plans=[]),
])], wallet_balance=50000, loyalty_tier="bronze", is_first_purchase=True)))
w("quote.json", camel(QuoteView(plan_id=U(2), product_id=U(100), base_price=180000, total=150000, total_discount=30000, discount_percent=16, cashback=1500,
    lines=(PriceLineView(kind="base", label="قیمت پایه", amount=180000, is_deduction=False), PriceLineView(kind="campaign", label="تخفیف", amount=30000, is_deduction=True)),
    compare_at_price=180000, campaign_label=None, coupon_code=None)))
w("coupon_rejected.json", camel(CouponPreview.rejected(code="NOPE", message_fa="این کد معتبر نیست.")))
w("payment_methods.json", camel([{"key": k, "label_fa": l} for k, l in [("card", "کارت به کارت"), ("zarinpal", "درگاه زرین‌پال")]]))
w("checkout_wallet.json", camel({"subscription_id": str(U(9))}))
pend = R.PendingPayment(payment_id=U(77), reference="P-1405-000077", amount=150000, method=PaymentMethod.CARD, state=PaymentState.AWAITING_PROOF, created_at=T)
w("checkout_card.json", camel(R.CardPaymentDetails(card_number="6037991234567890", card_holder_fa="گیک", bank_fa="ملی", review_sla_fa="تا یک ساعت", payment=pend)))
w("checkout_gateway.json", camel(R.GatewayScreen(url="https://pay.example.com/start/abc", body_fa="", payment_id=U(78))))
w("wallet.json", camel(R.WalletSnapshot(balance=50000, lifetime_spend=300000, pending_credit=0, tier=LoyaltyTier.BRONZE)))
w("wallet_transactions.json", camel({"items": [R.WalletTransaction(transaction_id=U(5), kind=TransactionKind.TOPUP, amount=100000, created_at=T, description_fa="افزایش موجودی", balance_after=100000)], "page": 1, "page_size": 10, "total": 1}))
w("trial.json", model(M.TrialOfferResponse(available=True, traffic_mib=50, duration_days=2)))
w("referral.json", camel(dataclasses.asdict(R.ReferralSummary(code="K7Q2MX", invited_count=4, converted_count=1, total_earned=0, pending_earned=15000)) | {"invitee_bonus": 10000, "first_purchase_bps": 1000, "recurring_bps": 500}))
w("usage_days.json", [model(M.UsageDayView(day="2026-09-26", used_mib=1536)), model(M.UsageDayView(day="2026-09-27", used_mib=0))])
w("tickets.json", camel([R.TicketCard(ticket_id=U(40), reference="SUP-1405-000040", topic_fa="مشکل اتصال", state=CardTicketState.WAITING, created_at=T, unread_count=2, last_reply_at=T)]))
w("ticket_messages.json", camel([
    {"message_id": U(41).hex, "ticket_id": U(40).hex, "kind": "customer", "from_support": False, "body_fa": "روی ایرانسل وصل نمی‌شود", "created_at": T, "attachment_count": 0, "is_read": True},
    {"message_id": U(42).hex, "ticket_id": U(40).hex, "kind": "support", "from_support": True, "body_fa": "یک اسکن بزنید.", "created_at": T, "attachment_count": 0, "is_read": False}]))
