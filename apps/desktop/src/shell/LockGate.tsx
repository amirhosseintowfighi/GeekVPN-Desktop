import { useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { Button } from "../design-system/controls";
import { Logo } from "../design-system/Logo";
import { errorText } from "../lib/auth";
import { faDigits } from "../lib/fa";
import { lock, pinOk, type LockView } from "../lib/lock";
import { currentOs, inTauri } from "../lib/platform";
import { Backdrop } from "./Backdrop";
import { TitleBar } from "./TitleBar";

const ACTIVITY = ["pointerdown", "keydown", "wheel", "pointermove"] as const;

function LockScreen({ view }: { view: LockView }) {
  const [pin, setPin] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [forgot, setForgot] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => input.current?.focus(), []);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!pinOk(pin)) return;
    setBusy(true);
    try {
      await lock.unlock(pin);
    } catch (err) {
      setError(errorText(err));
      setPin("");
    }
    setBusy(false);
  };

  return (
    <div dir="rtl" className="relative flex h-full w-full items-center justify-center overflow-hidden bg-bg text-on-bg">
      <Backdrop light="arc" />
      <TitleBar os={currentOs()} />
      <form onSubmit={(e) => void submit(e)} className="glass-milk relative z-10 flex w-[340px] flex-col items-center gap-4 rounded-[28px] px-7 py-8">
        <Logo />
        <span className="micro text-[9px] tracking-[0.22em]">Locked</span>
        <span className="text-xl font-extrabold">برنامه قفل است</span>
        <span className="text-center text-[12px] leading-[1.8] text-ink-2">VPN سر جایش است؛ برای دیدن برنامه رمز را بزن.</span>
        <label className="sr-only" htmlFor="pin">
          رمز
        </label>
        <input
          id="pin"
          ref={input}
          type="password"
          inputMode="numeric"
          autoComplete="off"
          dir="ltr"
          maxLength={8}
          value={pin}
          onChange={(e) => {
            setPin(e.target.value);
            setError(null);
          }}
          className="h-12 w-full rounded-full border border-white/25 bg-white/7 px-4 text-center font-num text-xl tracking-[0.5em] text-white outline-none focus:border-white/60"
        />
        {(error || view.waitSecs > 0) && (
          <span role="alert" className="text-center text-[12px] text-bad">
            {view.waitSecs > 0 ? `چند بار اشتباه زدی؛ ${faDigits(view.waitSecs)} ثانیه صبر کن.` : error}
          </span>
        )}
        <Button type="submit" kind="white" height={44} className="w-full" disabled={busy || !pinOk(pin)}>
          باز کردن
        </Button>
        {!forgot ? (
          <button type="button" onClick={() => setForgot(true)} className="text-[12px] text-ink-2 underline underline-offset-4">
            رمز را فراموش کرده‌ام
          </button>
        ) : (
          <div className="flex flex-col gap-2 border-t border-hair pt-3 text-center">
            <span className="text-[12px] leading-[1.8] text-ink-2">
              قفل برداشته می‌شود و برای امنیت، از حساب بیرون می‌روی و همه‌ی سرویس‌ها و لینک‌های این کامپیوتر پاک می‌شوند. با ورود دوباره، سرویس‌های حسابت برمی‌گردند.
            </span>
            <Button kind="danger" height={38} onClick={() => void lock.forgot().catch((e) => setError(errorText(e)))}>
              پاک کن و باز کن
            </Button>
          </div>
        )}
      </form>
    </div>
  );
}

/**
 * «قفل برنامه»: the app behind a PIN at start, and again after the chosen
 * idle time (the window hidden by the clock counts as idle).
 */
export function LockGate({ children }: { children: ReactNode }) {
  const [view, setView] = useState<LockView | null>(inTauri ? null : { enabled: false, locked: false, idleMinutes: 0, waitSecs: 0 });
  const last = useRef(Date.now());

  useEffect(() => {
    void lock.state().then(setView, () => setView({ enabled: false, locked: false, idleMinutes: 0, waitSecs: 0 }));
    const off = lock.onChange(setView);
    return () => void off.then((f) => f());
  }, []);

  const watching = Boolean(view?.enabled && !view.locked && view.idleMinutes > 0);
  useEffect(() => {
    if (!watching || !view) return;
    last.current = Date.now();
    const seen = () => (last.current = Date.now());
    for (const ev of ACTIVITY) window.addEventListener(ev, seen, { passive: true });
    const t = setInterval(() => {
      if (Date.now() - last.current >= view.idleMinutes * 60_000) void lock.now();
    }, 15_000);
    return () => {
      clearInterval(t);
      for (const ev of ACTIVITY) window.removeEventListener(ev, seen);
    };
  }, [watching, view?.idleMinutes]);

  // A wait after wrong tries counts down on screen.
  useEffect(() => {
    if (!view?.locked || view.waitSecs <= 0) return;
    const t = setTimeout(() => void lock.state().then(setView), 1000);
    return () => clearTimeout(t);
  }, [view]);

  if (view === null) return <div className="h-full w-full bg-bg" />;
  if (view.locked) return <LockScreen view={view} />;
  return <>{children}</>;
}
