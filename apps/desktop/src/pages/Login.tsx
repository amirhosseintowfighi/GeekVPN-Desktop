import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { Icon } from "../design-system/Icon";
import { Lockup } from "../design-system/Logo";
import { Button, Hair } from "../design-system/controls";
import { auth, errorText, type LoginStatus, type LoginView } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { currentOs } from "../lib/platform";
import { Backdrop } from "../shell/Backdrop";
import { TitleBar } from "../shell/TitleBar";

const FEATURES = ["اتصال هوشمند", "Kill Switch واقعی", "سایت‌های ایرانی مستقیم", "پشتیبانی تلگرام"];

const WAIT_STEPS = [
  "تلگرام باز شد؛ اگر نشد، کد QR را با گوشی اسکن کن.",
  "در ربات GeekVPN دکمه‌ی «تأیید و اتصال» را بزن.",
  "این پنجره خودش وارد می‌شود و سرویس‌ها را می‌آورد.",
];

type Mode = { kind: "choose" } | { kind: "wait"; login: LoginView; startedAt: number };

/** A numbered ruled list, as on the left of Desktop-Login. */
function Numbered({ items, size = 16 }: { items: string[]; size?: number }) {
  return (
    <div className="flex shrink-0 flex-col">
      <Hair />
      {items.map((t, i) => (
        <div key={t} className="flex min-h-6 items-center gap-3 border-b border-hair py-3">
          <span dir="ltr" className="w-[30px] shrink-0 text-right font-num text-[10px] tracking-[0.18em] text-ink-2">
            {String(i + 1).padStart(2, "0")}
          </span>
          <span className="font-bold" style={{ fontSize: size }}>
            {t}
          </span>
        </div>
      ))}
    </div>
  );
}

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
  const waiting = mode.kind === "wait";

  return (
    <div dir="rtl" className="relative h-full w-full overflow-hidden bg-bg text-on-bg">
      <Backdrop light={waiting ? "bars" : "horizon"} />
      <TitleBar os={currentOs()} />
      <div className="absolute inset-x-14 bottom-10 top-10 flex gap-[60px] max-[1100px]:inset-x-6 max-[1100px]:gap-8">
        <div className="flex min-w-0 flex-[1.1] flex-col gap-[22px] px-5 py-[30px]">
          <Lockup size={52} text={24} />
          <div className="flex-1" />
          {waiting ? (
            <>
              <h1 className="m-0 text-[60px] font-extrabold leading-[1.15] max-[1100px]:text-[44px]">
                یه تأیید
                <br />
                <span className="text-blue-text">تا اتصال.</span>
              </h1>
              <Numbered items={WAIT_STEPS} size={15} />
            </>
          ) : (
            <>
              <h1 className="m-0 text-[68px] font-extrabold leading-[1.15] max-[1100px]:text-[48px]">
                اینترنت آزاد،
                <br />
                سریع و امن
                <br />
                <span className="text-blue-text">روی هر سیستمی.</span>
              </h1>
              <span className="micro text-[13px] tracking-[0.22em]">Windows · macOS · Linux</span>
              <Numbered items={FEATURES} />
            </>
          )}
        </div>

        <section className="glass-milk flex w-[440px] shrink-0 flex-col gap-4 self-center rounded-[32px] p-8 shadow-[inset_0_1px_0_rgba(255,255,255,0.45),0_30px_70px_rgba(0,4,30,0.45)]">
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
            <>
              <span className="micro">Sign in</span>
              <span className="text-[28px] font-extrabold">ورود به حساب</span>
              <p className="m-0 text-sm leading-[1.8] text-white/85">با تأیید در ربات تلگرام GeekVPN وارد شو؛ سرویس‌هایت خودکار روی این کامپیوتر می‌آیند.</p>
              <button
                type="button"
                disabled={busy}
                onClick={() => void startTelegram()}
                className="flex h-14 shrink-0 items-center justify-center gap-2.5 rounded-full bg-white text-navy shadow-[0_14px_30px_rgba(0,4,30,0.45)] disabled:opacity-60"
              >
                <span className="text-base font-extrabold">{busy ? "در حال ساخت لینک ورود…" : "ورود با تلگرام"}</span>
                <Icon name="plane" size={20} stroke={2.2} />
              </button>
              {error && <ErrorLine text={error} />}
              <div className="flex items-center gap-3 text-xs text-ink-2">
                <span className="h-px flex-1 bg-hair" />
                یا
                <span className="h-px flex-1 bg-hair" />
              </div>
              <PasswordForm />
              <div className="flex justify-between text-xs">
                {/* The bot creates the account on /start, so this is the same flow. */}
                <button type="button" disabled={busy} onClick={() => void startTelegram()} className="underline underline-offset-4">
                  ساخت حساب
                </button>
                <button type="button" onClick={continueAsGuest} className="text-ink-2 underline underline-offset-4">
                  شروع سریع بدون ثبت‌نام
                </button>
              </div>
              <span className="text-[11px] leading-[1.7] text-ink-2">توکن ورود در Credential Manager / Keychain سیستم‌عامل نگه داشته می‌شود.</span>
            </>
          )}
        </section>
      </div>
    </div>
  );
}

function ErrorLine({ text }: { text: string }) {
  return (
    <div role="alert" className="flex gap-2 rounded-[18px] bg-bad-soft px-3.5 py-2.5 text-xs leading-[1.8] text-bad">
      <Icon name="alert" size={18} />
      <span>{text}</span>
    </div>
  );
}

/** «ورود با نام کاربری»: the username and password made in the bot. */
function PasswordForm() {
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

  const field = "h-[46px] rounded-full border border-white/25 bg-white/7 px-4 text-sm text-white outline-none focus:border-white/60";
  return (
    <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-4">
      <label className="flex flex-col gap-1.5 text-xs text-ink-2">
        نام کاربری
        <input dir="ltr" autoComplete="username" value={username} onChange={(e) => setUsername(e.target.value)} className={field} />
      </label>
      <label className="flex flex-col gap-1.5 text-xs text-ink-2">
        رمز
        <input dir="ltr" type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} className={field} />
      </label>
      {error && <ErrorLine text={error} />}
      <Button type="submit" kind="soft" height={48} disabled={busy || !username.trim() || !password}>
        {busy ? "در حال ورود…" : "ورود با نام کاربری"}
      </Button>
      <span className="text-[11px] leading-[1.7] text-ink-2">نام کاربری و رمز را در ربات می‌سازی: پروفایل ← «ورود به اپ با نام کاربری».</span>
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

  return (
    <>
      <span className="micro">{ended ? "Sign in · Failed" : "Waiting · Telegram"}</span>
      <span className="text-[26px] font-extrabold">{ended ? "ورود انجام نشد" : "منتظر تأیید در تلگرام"}</span>
      <span className="text-[13px] text-ink-2">{ended ? ended.message : "درخواست ورود فرستاده شد"}</span>
      <div className={`flex h-[200px] w-[200px] items-center justify-center self-center rounded-[26px] bg-white ${ended ? "opacity-30" : ""}`}>
        <img src={`data:image/svg+xml;utf8,${encodeURIComponent(login.qrSvg)}`} alt="کد QR لینک ورود" width={168} height={168} />
      </div>
      <span className="self-center text-xs text-ink-2">با دوربین گوشی یا اسکنر تلگرام اسکن کن</span>
      {!ended && (
        <>
          <div className="flex items-center justify-between">
            <span className="text-[13px] text-ink-2">اعتبار لینک</span>
            <span dir="ltr" className="font-num text-[22px]" aria-live="off">
              {mm}:{ss}
            </span>
          </div>
          <div className="h-[3px] overflow-hidden rounded-sm bg-white/16" aria-hidden="true">
            <div className="h-full bg-white transition-[width]" style={{ width: `${(left / login.expiresIn) * 100}%` }} />
          </div>
        </>
      )}
      {note && <ErrorLine text={note} />}
      <div className="flex gap-2.5">
        {ended ? (
          <Button icon="refresh" height={40} onClick={onRestart}>
            دوباره امتحان کن
          </Button>
        ) : (
          <Button kind="soft" icon="plane" height={40} onClick={() => void auth.reopenTelegram().catch((e) => setNote(errorText(e)))}>
            باز کردن دوباره‌ی تلگرام
          </Button>
        )}
        <Button kind="soft" icon={copied ? "check" : "copy"} height={40} onClick={() => void copy()}>
          {copied ? "کپی شد" : "کپی لینک"}
        </Button>
      </div>
      <button type="button" onClick={onBack} className="self-start text-[13px] text-ink-2 underline underline-offset-4">
        {ended ? "بازگشت" : "انصراف"}
      </button>
    </>
  );
}
