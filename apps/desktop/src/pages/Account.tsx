import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { Badge, Button, Switch } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { Card, EmptyState, Group, PageHeader, Row } from "../design-system/layout";
import { bot, referral, shop, support, toman, wallet, type Preferences, type Profile, type PaymentMethod, type PaymentStart, type ReferralView, type Wallet, type WalletTransactions } from "../lib/account";
import { errorText } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";
import { useUnread } from "../lib/useUnread";
import { useUpdate } from "../shell/UpdateDialog";
import { field, PaymentResult } from "./Payment";

const TX_KIND: Record<string, string> = {
  topup: "افزایش موجودی",
  purchase: "خرید",
  cashback: "بازگشت وجه",
  referral: "درآمد دعوت",
  refund: "بازپرداخت",
  adjustment: "اصلاح",
};
const PRESETS = [50_000, 100_000, 200_000, 500_000];

function Topup({ methods, onDone }: { methods: PaymentMethod[]; onDone: () => void }) {
  const [amount, setAmount] = useState("100000");
  const [method, setMethod] = useState(methods[0]?.key ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [started, setStarted] = useState<PaymentStart | null>(null);
  const value = Number(amount.replace(/[^\d]/g, ""));

  if (started) return <PaymentResult start={started} amount={value} onDone={onDone} onClose={() => setStarted(null)} />;
  const go = async () => {
    setBusy(true);
    setError(null);
    try {
      setStarted(await wallet.topup(value, method));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="glass-milk flex flex-col gap-3 rounded-[26px] p-4">
      <span className="text-base font-extrabold text-ink">افزایش موجودی</span>
      <div className="grid grid-cols-4 gap-2">
        {PRESETS.map((p) => (
          <button
            key={p}
            type="button"
            onClick={() => setAmount(String(p))}
            className={`h-10 rounded-full text-xs font-bold text-ink ${value === p ? "bg-white text-navy" : "border border-white/25 bg-white/7"}`}
          >
            {toman(p)}
          </button>
        ))}
      </div>
      <label className="flex flex-col gap-1.5 text-xs font-bold text-ink-2">
        مبلغ (تومان)
        <input dir="ltr" inputMode="numeric" value={amount} onChange={(e) => setAmount(e.target.value)} className={`${field} font-num`} />
      </label>
      {methods.length === 0 ? (
        <span className="text-xs text-warn">الان روشی برای پرداخت فعال نیست.</span>
      ) : (
        <div role="radiogroup" aria-label="روش پرداخت" className="flex flex-wrap gap-2">
          {methods.map((m) => (
            <button
              key={m.key}
              type="button"
              role="radio"
              aria-checked={m.key === method}
              onClick={() => setMethod(m.key)}
              className={`h-9 rounded-full px-3 text-xs font-bold ${m.key === method ? "bg-action text-on-action" : "border border-white/25 bg-white/7 text-ink"}`}
            >
              {m.labelFa}
            </button>
          ))}
        </div>
      )}
      {error && (
        <span role="alert" className="text-xs text-bad">
          {error}
        </span>
      )}
      <Button icon="lock" disabled={busy || !method || value <= 0} onClick={() => void go()}>
        {busy ? "…" : `پرداخت ${value > 0 ? toman(value) : ""} تومان`}
      </Button>
    </section>
  );
}

function Transactions() {
  const [page, setPage] = useState(1);
  const [data, setData] = useState<WalletTransactions | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    wallet.transactions(page).then(setData, (e) => setError(errorText(e)));
  }, [page]);
  const pages = data ? Math.max(1, Math.ceil(data.total / data.pageSize)) : 1;
  return (
    <section className="glass-milk flex max-h-[420px] flex-col overflow-hidden rounded-[26px]">
      <div className="flex items-center gap-2 border-b border-hair px-4 py-3">
        <span className="flex-1 text-base font-extrabold text-ink">تراکنش‌ها</span>
        {pages > 1 && (
          <span className="flex items-center gap-1 text-xs text-ink-2">
            <button type="button" aria-label="صفحه‌ی قبل" disabled={page <= 1} onClick={() => setPage(page - 1)} className="p-1 disabled:opacity-40">
              <span className="flex rotate-180">
                <Icon name="chev" size={16} />
              </span>
            </button>
            {faDigits(page)} از {faDigits(pages)}
            <button type="button" aria-label="صفحه‌ی بعد" disabled={page >= pages} onClick={() => setPage(page + 1)} className="p-1 disabled:opacity-40">
              <Icon name="chev" size={16} />
            </button>
          </span>
        )}
      </div>
      <div className="overflow-y-auto">
        {error && <p className="m-0 p-4 text-[13px] text-bad">{error}</p>}
        {data?.items.length === 0 && <p className="m-0 p-6 text-center text-[13px] text-ink-2">هنوز تراکنشی نداری.</p>}
        {data?.items.map((t) => {
          const credit = t.amount > 0 && t.kind !== "purchase";
          return (
            <div key={t.transactionId} className="flex items-center gap-3 border-b border-hair px-4 py-2.5 last:border-b-0">
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="text-[13px] font-bold text-ink">{t.descriptionFa || TX_KIND[t.kind] || t.kind}</span>
                <span className="text-[11px] text-muted">{new Date(t.createdAt).toLocaleDateString("fa-IR", { dateStyle: "medium" })}</span>
              </span>
              <span className={`text-[13px] font-extrabold ${credit ? "text-ok" : "text-ink"}`}>
                {credit ? "+" : "−"}
                {toman(Math.abs(t.amount))}
              </span>
            </div>
          );
        })}
      </div>
    </section>
  );
}

const PREFS: readonly [keyof Preferences, string, string][] = [
  ["expiry", "یادآوری انقضا", "چند روز قبل از تمام شدن زمان سرویس"],
  ["traffic", "هشدار اتمام حجم", "وقتی بیشتر حجم سرویس مصرف شد"],
  ["promos", "اطلاع از تخفیف‌ها", "کد تخفیف و پیشنهادهای ویژه"],
  ["news", "اخبار و به‌روزرسانی‌ها", "خبرهای سرویس و نسخه‌های تازه"],
  ["quiet_hours", "ساعات سکوت شبانه", "پیام‌های غیرضروری شب‌ها فرستاده نمی‌شوند"],
];

/** The bot's «تنظیمات اعلان»: what the bot sends to Telegram. */
function Notifications() {
  const [prefs, setPrefs] = useState<Preferences | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    bot.prefs().then(setPrefs, (e) => setError(errorText(e)));
  }, []);
  const flip = (key: keyof Preferences, on: boolean) => {
    if (!prefs) return;
    const next = { ...prefs, [key]: on };
    setPrefs(next);
    setError(null);
    bot.savePrefs(next).then(setPrefs, (e) => {
      setPrefs(prefs);
      setError(errorText(e));
    });
  };
  return (
    <Group micro="Telegram notifications">
      {PREFS.map(([key, title, hint]) => (
        <Row
          key={key}
          title={title}
          hint={hint}
          trailing={<Switch label={title} checked={prefs?.[key] ?? false} onChange={(v) => prefs && flip(key, v)} />}
        />
      ))}
      {error && <span role="alert" className="py-2 text-xs text-bad">{error}</span>}
    </Group>
  );
}

/** «نام نمایشی»: the name the bot and support see. */
function NameRow({ profile, onSaved }: { profile: Profile | null; onSaved: (p: Profile) => void }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      onSaved(await bot.rename(name));
      setEditing(false);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  if (editing) {
    return (
      <div className="flex flex-col gap-2 border-b border-hair py-2.5">
        <div className="flex items-center gap-2">
          <input
            aria-label="نام نمایشی"
            value={name}
            maxLength={64}
            autoFocus
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && name.trim() && void save()}
            className={`${field} h-10 min-w-0 flex-1`}
          />
          <Button height={38} disabled={busy || !name.trim()} onClick={() => void save()}>
            ذخیره
          </Button>
          <Button kind="soft" height={38} onClick={() => setEditing(false)}>
            انصراف
          </Button>
        </div>
        {error && <span role="alert" className="text-xs text-bad">{error}</span>}
      </div>
    );
  }
  return (
    <Row
      title="نام نمایشی"
      hint={profile?.display_name || "تنظیم نشده"}
      onClick={() => {
        setName(profile?.display_name ?? "");
        setEditing(true);
      }}
    />
  );
}

/** Desktop-Account: profile, wallet, referral, and the ways to support. */
export function Account() {
  const { view, signOut, showSignIn } = useAuth();
  const navigate = useNavigate();
  const unread = useUnread();
  const update = useUpdate();
  const newVersion = update.view?.available?.version ?? null;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [money, setMoney] = useState<Wallet | null>(null);
  const [ref, setRef] = useState<ReferralView | null>(null);
  const [methods, setMethods] = useState<PaymentMethod[]>([]);
  const [profile, setProfile] = useState<Profile | null>(null);
  const [panel, setPanel] = useState<"none" | "topup" | "history">("none");
  const user = view?.user;

  const load = useCallback(() => {
    wallet.load().then(setMoney, (e) => setError(errorText(e)));
    referral.load().then(setRef, () => {});
    bot.profile().then(setProfile, () => {});
    shop.load().then((s) => setMethods(s.methods), () => {});
  }, []);
  useEffect(() => {
    if (user) load();
  }, [user, load]);

  if (!user) {
    return (
      <div className="flex min-w-0 flex-1 flex-col gap-3.5">
        <PageHeader micro="Account" title="حساب" />
        <EmptyState
          icon="user"
          title="وارد نشده‌ای"
          text="با ورود از طریق ربات تلگرام، سرویس‌ها و کیف پولت روی این کامپیوتر می‌آیند."
          action={
            <Button kind="white" icon="plane" onClick={showSignIn}>
              ورود با تلگرام
            </Button>
          }
        />
      </div>
    );
  }

  const leave = async () => {
    setBusy(true);
    setError(null);
    try {
      await signOut();
    } catch (e) {
      setError(errorText(e));
      setBusy(false);
    }
  };

  const r = ref?.summary;
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
      <PageHeader
        micro="Account"
        title={user.displayName || `کاربر ${faDigits(user.telegramId)}`}
        subtitle={
          <>
            متصل به تلگرام{user.username ? ` · @${user.username}` : ""} · شناسه <span dir="ltr">{faDigits(user.telegramId)}</span>
          </>
        }
        actions={
          <button type="button" disabled={busy} onClick={() => void leave()} className="text-[13px] text-bad underline underline-offset-4 disabled:opacity-60">
            {busy ? "در حال خروج…" : "خروج از حساب"}
          </button>
        }
      />
      {error && (
        <span role="alert" className="text-[13px] text-bad">
          {error}
        </span>
      )}
      {view?.storeError && (
        <div role="status" className="flex items-start gap-2 text-[13px] leading-[1.8] text-warn">
          <Icon name="alert" size={18} />
          <span>{view.storeError}</span>
        </div>
      )}
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto pb-2 pe-1 [scrollbar-width:thin]">
        <div className="flex gap-4 max-[1100px]:flex-col">
          <Card className="flex-1 basis-0">
            <span className="micro text-[9px]">Wallet</span>
            <span className="flex items-baseline gap-1.5">
              <span className="text-[34px] font-extrabold leading-none">{money ? toman(money.balance) : "…"}</span>
              <span className="text-sm text-ink-2">تومان</span>
            </span>
            {money && money.pendingCredit > 0 && <span className="text-xs text-ink-2">{toman(money.pendingCredit)} تومان در راه</span>}
            <div className="flex gap-2">
              <Button icon="plus" height={40} aria-expanded={panel === "topup"} onClick={() => setPanel(panel === "topup" ? "none" : "topup")}>
                افزایش موجودی
              </Button>
              <Button kind="soft" height={40} aria-expanded={panel === "history"} onClick={() => setPanel(panel === "history" ? "none" : "history")}>
                تراکنش‌ها
              </Button>
            </div>
          </Card>
          <Card className="flex-1 basis-0">
            <span className="micro text-[9px]">Referral</span>
            <span className="text-lg font-extrabold">دعوت از دوستان</span>
            <span className="text-xs text-ink-2">
              {r ? `${faDigits(r.invitedCount)} دعوت · ${faDigits(r.convertedCount)} خرید · درآمد ${toman(r.totalEarned)} تومان` : "لینک دعوت و درآمدت"}
            </span>
            <div className="flex">
              <Button kind="soft" icon="gift" height={40} onClick={() => navigate("/referral")}>
                لینک دعوت
              </Button>
            </div>
          </Card>
        </div>
          {panel === "topup" && (
            <Topup
              methods={methods}
              onDone={() => {
                setPanel("none");
                load();
              }}
            />
          )}
          {panel === "history" && <Transactions />}
        <div className="flex items-start gap-10 max-[1100px]:flex-col max-[1100px]:gap-5">
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <Group micro="Tools">
            <Row icon="chart" title="مصرف روزانه" hint="ترافیک VPN همین کامپیوتر و هر سرویس" onClick={() => navigate("/usage")} />
            <Row icon="gauge" title="تست سرعت" hint="پینگ، دانلود و آپلود" onClick={() => navigate("/tools")} />
          </Group>
        </div>
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <Group micro="Support">
            <Row
              icon="chat"
              title="تیکت‌های من"
              hint="جواب‌های پشتیبانی، داخل برنامه"
              onClick={() => navigate("/support")}
              trailing={
                <span className="flex items-center gap-2">
                  {unread > 0 && <Badge tone="bad">{faDigits(unread)} جدید</Badge>}
                  <span className="flex text-ink-2">
                    <Icon name="chev" size={16} />
                  </span>
                </span>
              }
            />
            <Row icon="chat" title="راهنما و وضعیت سرورها" hint="سوالات متداول ربات و سلامت سرورها" onClick={() => navigate("/help")} />
            <Row icon="flag" title="گزارش مشکل" hint="یک گزارش فنی برای پشتیبانی می‌فرستد" onClick={() => navigate("/report")} />
            <Row icon="plane" title="ربات پشتیبانی" hint="گفتگو در تلگرام" onClick={() => void support.openBot().catch((e) => setError(errorText(e)))} />
          </Group>
          <Group micro="App">
            <Row
              icon="dl"
              title="به‌روزرسانی برنامه"
              hint={newVersion ? `نسخه‌ی ${faDigits(newVersion)} آماده است` : `نسخه‌ی ${faDigits(update.view?.current ?? "")} · بررسی نسخه‌ی جدید`}
              onClick={update.openDialog}
              trailing={
                <span className="flex items-center gap-2">
                  {newVersion && <Badge tone="link">جدید</Badge>}
                  <span className="flex text-ink-2">
                    <Icon name="chev" size={16} />
                  </span>
                </span>
              }
            />
          </Group>
          <Group micro="Profile">
            <NameRow profile={profile} onSaved={setProfile} />
            <Row
              title="عضویت"
              hint={profile?.joined_at ? `از ${new Date(profile.joined_at).toLocaleDateString("fa-IR", { dateStyle: "medium" })} · ${faDigits(profile.order_count)} خرید` : "…"}
              trailing={<span />}
            />
            <Row title="کد معرف" hint={<span dir="ltr" className="font-num">{user.referralCode}</span>} trailing={<span />} />
          </Group>
          <Notifications />
        </div>
        </div>
      </div>
    </div>
  );
}
