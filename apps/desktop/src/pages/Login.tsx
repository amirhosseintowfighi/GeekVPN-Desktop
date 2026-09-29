import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { Icon, type IconName } from "../design-system/Icon";
import { Logo } from "../design-system/Logo";
import { Button } from "../design-system/controls";
import { auth, errorText, type LoginStatus, type LoginView } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";
import { currentOs } from "../lib/platform";
import { Backdrop } from "../shell/Backdrop";
import { TitleBar } from "../shell/TitleBar";

const FEATURES: readonly [IconName, string][] = [
  ["bolt", "اتصال هوشمند"],
  ["wall", "Kill Switch واقعی"],
  ["split", "سایت‌های ایرانی مستقیم"],
  ["plane", "پشتیبانی تلگرام"],
];

type Mode = { kind: "choose" } | { kind: "password" } | { kind: "wait"; login: LoginView; startedAt: number };

/** Desktop-Login and Desktop-Login-Wait. */
export function Login() {
  const [mode, setMode] = useState<Mode>({ kind: "choose" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { continueAsGuest } = useAuth();

  const startTelegram = async () => {
    setBusy(true);
    setError(null);
    try {
      const login = await auth.startTelegram();
      setMode({ kind: "wait", login, startedAt: Date.now() });
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div dir="rtl" className="relative h-full w-full overflow-hidden bg-bg text-on-bg">
      <Backdrop />
      <TitleBar os={currentOs()} />
      {mode.kind === "wait" ? (
        <Waiting
          login={mode.login}
          startedAt={mode.startedAt}
          onRestart={startTelegram}
          onBack={() => {
            void auth.cancel();
            setMode({ kind: "choose" });
          }}
        />
      ) : (
        <div className="absolute inset-x-[72px] bottom-10 top-[70px] flex items-center gap-12">
          <div className="flex flex-1 flex-col gap-6">
            <div className="relative flex h-[240px] w-[240px] items-center justify-center">
              <svg aria-hidden="true" width="240" height="240" viewBox="0 0 240 240" className="absolute inset-0">
                <rect x="28" y="28" width="184" height="184" rx="56" fill="rgba(255,255,255,0.10)" stroke="rgba(255,255,255,0.35)" />
                <rect x="9" y="9" width="222" height="222" rx="67" fill="none" stroke="rgba(255,255,255,0.18)" strokeDasharray="3 8" />
              </svg>
              <span className="relative">
                <Logo size={140} color="#FFFFFF" title="GeekVPN" />
              </span>
            </div>
            <h1 dir="ltr" className="m-0 text-right font-num text-[56px] font-bold leading-none tracking-[-1.5px]">
              GeekVPN
            </h1>
            <p className="m-0 text-lg leading-[1.8] text-white/90">اینترنت آزاد، سریع و امن — روی ویندوز، مک و لینوکس.</p>
            <div className="flex flex-wrap gap-2">
              {FEATURES.map(([icon, label]) => (
                <span key={label} className="glass-clear flex h-9 items-center gap-1.5 rounded-xl px-3.5 text-[13px] font-semibold">
                  <Icon name={icon} size={16} />
                  {label}
                </span>
              ))}
            </div>
          </div>

          <section className="glass-milk flex w-[420px] shrink-0 flex-col gap-3.5 rounded-[30px] p-7">
            {mode.kind === "choose" ? (
              <>
                <span className="text-[22px] font-extrabold">ورود به حساب</span>
                <span className="text-[13px] leading-[1.9] text-ink-2">
                  با تأیید در ربات تلگرام GeekVPN وارد شو؛ سرویس‌هایت خودکار روی این کامپیوتر می‌آیند.
                </span>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void startTelegram()}
                  className="flex h-[58px] items-center gap-2.5 rounded-[18px] bg-action pe-[7px] ps-4 text-[15px] font-bold text-on-action shadow-[var(--gv-action-shadow)] disabled:opacity-60"
                >
                  <span className="flex-1 text-start">{busy ? "در حال ساخت لینک ورود…" : "ورود با تلگرام"}</span>
                  <span className="flex h-11 w-11 items-center justify-center rounded-[15px] bg-white text-[#062845]">
                    <Icon name="plane" size={20} stroke={2.2} />
                  </span>
                </button>
                <div className="flex gap-2.5">
                  <Button kind="soft" icon="user" height={52} grow onClick={() => setMode({ kind: "password" })}>
                    نام کاربری
                  </Button>
                  {/* The bot creates the account on /start, so this is the same flow. */}
                  <Button kind="soft" icon="plus" height={52} grow disabled={busy} onClick={() => void startTelegram()}>
                    ساخت حساب
                  </Button>
                </div>
                {error && <ErrorLine text={error} />}
                <div className="flex items-center gap-2.5 text-xs text-muted">
                  <span className="h-px flex-1 bg-hair" />
                  یا
                  <span className="h-px flex-1 bg-hair" />
                </div>
                <button type="button" onClick={continueAsGuest} className="self-center p-1.5 text-[13px] font-bold text-link">
                  شروع سریع بدون ثبت‌نام
                </button>
                <span className="text-center text-[11px] leading-[1.8] text-ink-2">
                  توکن ورود فقط در کلیدساز امن سیستم‌عامل نگه داشته می‌شود.
                </span>
              </>
            ) : (
              <PasswordForm onBack={() => setMode({ kind: "choose" })} />
            )}
          </section>
        </div>
      )}
    </div>
  );
}

function ErrorLine({ text }: { text: string }) {
  return (
    <div role="alert" className="flex gap-2 rounded-[14px] bg-bad-soft px-3 py-2.5 text-xs leading-[1.8] text-bad">
      <Icon name="alert" size={18} />
      <span>{text}</span>
    </div>
  );
}

function PasswordForm({ onBack }: { onBack: () => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      // Success arrives as auth://changed, which swaps this screen for Home.
      await auth.password(username, password);
    } catch (err) {
      setError(errorText(err));
      setBusy(false);
    }
  };

  const field = "h-11 rounded-[13px] border border-chip-border bg-chip px-3 text-[13px] text-ink outline-none focus:border-link";
  return (
    <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-3.5">
      <span className="text-[22px] font-extrabold">ورود با نام کاربری</span>
      <span className="text-[13px] leading-[1.9] text-ink-2">
        نام کاربری و رمز را در ربات می‌سازی: پروفایل ← «ورود به اپ با نام کاربری».
      </span>
      <label className="flex flex-col gap-1.5 text-xs font-bold text-ink-2">
        نام کاربری
        <input dir="ltr" autoComplete="username" value={username} onChange={(e) => setUsername(e.target.value)} className={field} />
      </label>
      <label className="flex flex-col gap-1.5 text-xs font-bold text-ink-2">
        رمز
        <input
          dir="ltr"
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          className={field}
        />
      </label>
      {error && <ErrorLine text={error} />}
      <Button type="submit" kind="action" icon="key" height={48} disabled={busy || !username.trim() || !password}>
        {busy ? "در حال ورود…" : "ورود"}
      </Button>
      <button type="button" onClick={onBack} className="self-center p-1.5 text-[13px] font-bold text-link">
        بازگشت
      </button>
    </form>
  );
}

function useCountdown(startedAt: number, seconds: number): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);
  return Math.max(0, seconds - Math.floor((now - startedAt) / 1000));
}

function Waiting({
  login,
  startedAt,
  onRestart,
  onBack,
}: {
  login: LoginView;
  startedAt: number;
  onRestart: () => void;
  onBack: () => void;
}) {
  const left = useCountdown(startedAt, login.expiresIn);
  const [ended, setEnded] = useState<LoginStatus | null>(null);
  const [copied, setCopied] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const copiedTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    setEnded(null);
    const off = auth.onLoginStatus((s) => {
      // "approved" needs nothing here: auth://changed replaces this screen.
      if (s.status !== "approved") setEnded(s);
    });
    return () => {
      void off.then((f) => f());
      clearTimeout(copiedTimer.current);
    };
  }, [login]);

  const copy = async () => {
    try {
      await writeText(login.deepLink);
      setCopied(true);
      clearTimeout(copiedTimer.current);
      copiedTimer.current = setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      setNote(errorText(e));
    }
  };

  const mm = String(Math.floor(left / 60)).padStart(2, "0");
  const ss = String(left % 60).padStart(2, "0");
  const steps = [
    "تلگرام باز شد؛ اگر نشد، کد QR را با گوشی اسکن کن.",
    "در ربات GeekVPN دکمه‌ی «تأیید و اتصال» را بزن.",
    "این پنجره خودش وارد می‌شود و سرویس‌ها را می‌آورد.",
  ];

  return (
    <section className="glass-milk absolute left-1/2 top-[120px] flex w-[800px] -translate-x-1/2 gap-8 rounded-[30px] p-8">
      <div className="flex flex-1 flex-col gap-4">
        {ended ? (
          <span className="flex items-center gap-2 text-[13px] font-bold text-bad">
            <span className="h-[9px] w-[9px] rounded-[2px] bg-bad" />
            ورود انجام نشد
          </span>
        ) : (
          <span className="flex items-center gap-2 text-[13px] font-bold text-warn">
            <span className="h-[9px] w-[9px] rounded-[2px] bg-warn" />
            منتظر تأیید در تلگرام
          </span>
        )}
        <span className="text-2xl font-extrabold">{ended ? ended.message : "درخواست ورود فرستاده شد"}</span>
        {!ended &&
          steps.map((s, i) => (
            <div key={s} className="flex items-center gap-2.5">
              <span className="flex h-[26px] w-[26px] shrink-0 items-center justify-center rounded-[9px] bg-soft font-num text-[13px] font-bold text-link">
                {faDigits(i + 1)}
              </span>
              <span className="text-[13px] leading-[1.7]">{s}</span>
            </div>
          ))}
        {!ended && (
          <div className="mt-1.5 flex items-center gap-2.5">
            <span className="text-xs text-ink-2">اعتبار لینک</span>
            <span dir="ltr" className="font-num text-[22px] font-bold" aria-live="off">
              {mm}:{ss}
            </span>
            <span className="h-1.5 flex-1 overflow-hidden rounded-[3px] bg-track">
              <span className="block h-full bg-link transition-[width]" style={{ width: `${(left / login.expiresIn) * 100}%` }} />
            </span>
          </div>
        )}
        {note && <ErrorLine text={note} />}
        <div className="mt-1 flex gap-2.5">
          {ended ? (
            <Button kind="action" icon="refresh" height={48} grow onClick={onRestart}>
              دوباره امتحان کن
            </Button>
          ) : (
            <Button kind="action" icon="plane" height={48} grow onClick={() => void auth.reopenTelegram().catch((e) => setNote(errorText(e)))}>
              باز کردن دوباره‌ی تلگرام
            </Button>
          )}
          <Button kind="soft" icon={copied ? "check" : "copy"} height={48} onClick={() => void copy()}>
            {copied ? "کپی شد" : "کپی لینک"}
          </Button>
          <Button kind="danger" height={48} onClick={onBack}>
            {ended ? "بازگشت" : "انصراف"}
          </Button>
        </div>
      </div>
      <div className="flex w-[236px] shrink-0 flex-col items-center gap-2.5">
        <div className={`rounded-[22px] border border-chip-border bg-white p-3.5 ${ended ? "opacity-30" : ""}`}>
          <img src={`data:image/svg+xml;utf8,${encodeURIComponent(login.qrSvg)}`} alt="کد QR لینک ورود" width={196} height={196} />
        </div>
        <span className="text-center text-xs leading-[1.8] text-ink-2">با دوربین گوشی یا اسکنر تلگرام اسکن کن</span>
      </div>
    </section>
  );
}
