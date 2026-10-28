import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { inTauri } from "./platform";

/*
 * The shop, the wallet, referral, usage and support, as the Rust side hands
 * them over (GeekVPNBot's Mini App shapes, camelCase). Amounts are tomans.
 */

export type Tier = "direct" | "tunnel" | "elite";

export interface StorePlan {
  planId: string;
  nameFa: string;
  planType: string;
  durationDays: number;
  price: number;
  compareAtPrice: number | null;
  /** null: unlimited. */
  quotaGib: number | null;
  dailyQuotaGib: number | null;
  deviceLimit: number;
  badgeFa: string | null;
  isFeatured: boolean;
  descriptionFa: string | null;
}

export interface StoreProduct {
  productId: string;
  tier: Tier | string;
  nameFa: string;
  taglineFa: string | null;
  descriptionFa: string | null;
  featuresFa: string[];
  badgeFa: string | null;
  isFeatured: boolean;
  plans: StorePlan[];
}

export interface Storefront {
  categories: { categoryId: string; nameFa: string; icon: string | null; products: StoreProduct[] }[];
  walletBalance: number;
  loyaltyTier: string;
  isFirstPurchase: boolean;
}

export interface PaymentMethod {
  /** `card`, `crypto` or an online gateway's key. */
  key: string;
  labelFa: string;
}

export interface TrialOffer {
  available: boolean;
  trafficMib: number;
  durationDays: number;
}

export interface CardInfo {
  cardNumber: string;
  cardHolderFa: string;
  bankFa: string;
  reviewSlaFa: string;
}

export interface PendingPayment {
  paymentId: string;
  reference: string;
  amount: number;
  method: string;
  /** awaiting_proof | pending_review | … */
  state: string;
  createdAt: string | null;
}

export interface PaymentView extends PendingPayment {
  expiresAt: string | null;
  card: CardInfo | null;
}

export interface ShopView {
  store: Storefront;
  methods: PaymentMethod[];
  trial: TrialOffer | null;
  pending: PaymentView[];
}

export interface Quote {
  planId: string;
  basePrice: number;
  total: number;
  totalDiscount: number;
  discountPercent: number;
  cashback: number;
  lines: { kind: string; label: string; amount: number; isDeduction: boolean }[];
  compareAtPrice: number | null;
  campaignLabel: string | null;
  couponCode: string | null;
}

export interface CouponPreview {
  code: string;
  isValid: boolean;
  discount: number;
  totalAfter: number;
  messageFa: string;
}

export type PaymentStart =
  | { kind: "done"; subscriptionId: string }
  | { kind: "card"; card: CardInfo; payment: PendingPayment | null }
  | { kind: "gateway"; url: string; bodyFa: string; paymentId: string | null }
  | { kind: "crypto"; network: string; asset: string; amountDisplay: string; address: string; payment: PendingPayment | null };

export interface Wallet {
  balance: number;
  lifetimeSpend: number;
  pendingCredit: number;
  tier: string;
}

export interface WalletTransaction {
  transactionId: string;
  kind: string;
  amount: number;
  createdAt: string;
  descriptionFa: string;
  balanceAfter: number | null;
}

export interface WalletTransactions {
  items: WalletTransaction[];
  page: number;
  pageSize: number;
  total: number;
}

export interface Referral {
  code: string;
  invitedCount: number;
  convertedCount: number;
  totalEarned: number;
  pendingEarned: number;
  inviteeBonus: number;
  firstPurchaseBps: number;
  recurringBps: number;
}

export interface ReferralView {
  summary: Referral;
  link: string | null;
  qrSvg: string | null;
}

export interface Ticket {
  ticketId: string;
  reference: string;
  topicFa: string;
  /** open | waiting | answered | closed */
  state: string;
  createdAt: string;
  lastMessageFa: string;
  unreadCount: number;
  lastReplyAt: string | null;
}

export interface TicketMessage {
  messageId: string;
  fromSupport: boolean;
  bodyFa: string;
  createdAt: string;
}

export type TicketTopic = "connection" | "payment" | "account" | "speed" | "other";

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return Promise.reject("این کار فقط داخل برنامه‌ی GeekVPN انجام می‌شود.");
  return invoke<T>(cmd, args);
}

interface Buy {
  planId: string;
  coupon?: string | null;
  renews?: string | null;
}

export const shop = {
  load: () => call<ShopView>("shop_load"),
  quote: (b: Buy) => call<Quote>("shop_quote", { planId: b.planId, coupon: b.coupon ?? null, renews: b.renews ?? null }),
  coupon: (planId: string, code: string) => call<CouponPreview>("shop_coupon", { planId, code }),
  checkout: (b: Buy, method: string) =>
    call<PaymentStart>("shop_checkout", { planId: b.planId, coupon: b.coupon ?? null, renews: b.renews ?? null, method }),
  openGateway: () => call<void>("shop_open_gateway"),
  pending: () => call<PaymentView[]>("shop_pending"),
  receipt: (paymentId: string, path: string) => call<void>("shop_receipt", { paymentId, path }),
  txid: (paymentId: string, txid: string) => call<void>("shop_txid", { paymentId, txid }),
  claimTrial: () => call<{ subscriptionIds: string[]; pending: number }>("trial_claim"),
};

export const wallet = {
  load: () => call<Wallet>("wallet_load"),
  transactions: (page: number) => call<WalletTransactions>("wallet_transactions", { page }),
  topup: (amount: number, method: string) => call<PaymentStart>("wallet_topup", { amount, method }),
};

export const referral = {
  load: () => call<ReferralView>("referral_load"),
};

export const usage = {
  local: (days: number) => call<{ day: string; bytes: number }[]>("usage_local", { days }),
  service: (subscriptionId: string, days: number) => call<{ day: string; usedMib: number }[]>("usage_service", { subscriptionId, days }),
};

export const support = {
  tickets: () => call<Ticket[]>("tickets_list"),
  unread: () => (inTauri ? call<number>("support_unread") : Promise.resolve(0)),
  onUnread: (f: (count: number) => void): Promise<UnlistenFn> =>
    inTauri ? listen<{ count: number }>("support://unread", (e) => f(e.payload.count)) : Promise.resolve(() => {}),
  thread: (ticketId: string) => call<TicketMessage[]>("ticket_thread", { ticketId }),
  reply: (ticketId: string, message: string) => call<TicketMessage>("ticket_reply", { ticketId, message }),
  open: (topic: TicketTopic, subject: string, message: string) => call<Ticket>("ticket_open", { topic, subject, message }),
  reportPreview: (description: string) => call<string>("report_preview", { description }),
  reportSend: (description: string) => call<string>("report_send", { description }),
  openBot: () => call<void>("support_open_bot"),
};

/** Tomans with Persian digits and the thousands separator: «۱۵۰٬۰۰۰». */
export function toman(n: number): string {
  return new Intl.NumberFormat("fa-IR").format(Math.round(n));
}

/** «۱ ماهه» from 30 days, «۴۵ روزه» when it is not whole months. */
export function durationLabel(days: number): string {
  const fa = new Intl.NumberFormat("fa-IR");
  return days % 30 === 0 ? `${fa.format(days / 30)} ماهه` : `${fa.format(days)} روزه`;
}

export const TICKET_STATE: Record<string, { label: string; tone: "ok" | "warn" | "link" | "bad" }> = {
  open: { label: "در انتظار پشتیبانی", tone: "link" },
  waiting: { label: "منتظر پاسخ تو", tone: "warn" },
  answered: { label: "پاسخ داده شد", tone: "ok" },
  closed: { label: "بسته شده", tone: "link" },
};

export const TOPICS: readonly { value: TicketTopic; label: string }[] = [
  { value: "connection", label: "مشکل اتصال" },
  { value: "payment", label: "پرداخت و مالی" },
  { value: "account", label: "حساب کاربری" },
  { value: "speed", label: "سرعت و کیفیت" },
  { value: "other", label: "سایر" },
];

/**
 * The shop's three pickers (service type, then duration, then volume) over the
 * storefront's plans. Pure, so the picking is tested.
 *
 * The service type is the bot's category - its name is what the shop calls
 * it - and every product in it contributes its plans. It used to be the
 * product's tier, labelled with the model name (مستقیم/تونل/ویژه), and a shop
 * whose volumes are separate products ("10GB", "20GB", "100GB") of one tier
 * then showed every one of them under the same switch.
 */
export function planGrid(store: Storefront | null) {
  const categories = (store?.categories ?? [])
    .map((c) => ({
      id: c.categoryId,
      label: c.nameFa,
      tier: c.products.find((p) => p.plans.length)?.tier ?? null,
      plans: c.products.flatMap((p) => p.plans),
    }))
    .filter((c) => c.plans.length);
  const kinds = categories.map((c) => c.id);
  const find = (kind: string) => categories.find((c) => c.id === kind);
  const label = (kind: string) => find(kind)?.label ?? kind;
  const tierOf = (kind: string) => find(kind)?.tier ?? null;
  const plansOf = (kind: string) => find(kind)?.plans ?? [];
  const durations = (kind: string) => [...new Set(plansOf(kind).map((p) => p.durationDays))].sort((a, b) => a - b);
  const volumes = (kind: string, days: number) =>
    plansOf(kind)
      .filter((p) => p.durationDays === days)
      .sort((a, b) => (a.quotaGib ?? Infinity) - (b.quotaGib ?? Infinity));
  /** The category a renewed service belongs to: by its name, else by its tier. */
  const kindFor = (name: string | null | undefined, tier: string | null | undefined) =>
    categories.find((c) => name && c.label === name)?.id ?? categories.find((c) => tier && c.tier === tier)?.id ?? null;
  return { kinds, label, tierOf, durations, volumes, kindFor };
}

export interface DayUsage {
  /** `YYYY-MM-DD`. */
  day: string;
  bytes: number;
}

/** Total, daily average and the busiest day of a range. */
export function usageSummary(days: DayUsage[]) {
  const total = days.reduce((s, d) => s + d.bytes, 0);
  const peak = days.reduce<DayUsage | null>((p, d) => (d.bytes > 0 && (!p || d.bytes > p.bytes) ? d : p), null);
  return { total, average: days.length ? total / days.length : 0, peak };
}

/** A byte count in Persian: «۴۶٫۶ گیگ», «۳۱۲ مگ». */
export function sizeFa(n: number): { value: string; unit: string } {
  const fa = (v: number, digits: number) => new Intl.NumberFormat("fa-IR", { maximumFractionDigits: digits }).format(v);
  if (n >= 1024 ** 3) {
    const g = n / 1024 ** 3;
    return { value: fa(g, g < 10 ? 2 : 1), unit: "گیگ" };
  }
  if (n >= 1024 ** 2) return { value: fa(n / 1024 ** 2, 0), unit: "مگ" };
  if (n > 0) return { value: fa(n / 1024, 0), unit: "کیلوبایت" };
  return { value: "۰", unit: "مگ" };
}
