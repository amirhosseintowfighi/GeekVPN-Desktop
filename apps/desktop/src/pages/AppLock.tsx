import { useEffect, useState } from "react";
import { Button, Segmented } from "../design-system/controls";
import { Group, PageHeader, Row } from "../design-system/layout";
import { field } from "./Payment";
import { errorText } from "../lib/auth";
import { lock, pinOk, type LockView } from "../lib/lock";

const IDLE = [
  { value: "0", label: "فقط هنگام اجرا" },
  { value: "5", label: "۵ دقیقه" },
  { value: "15", label: "۱۵ دقیقه" },
  { value: "60", label: "۱ ساعت" },
] as const;

function PinInput({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  return (
    <label className="flex min-w-0 flex-1 flex-col gap-1.5 text-xs text-ink-2">
      {label}
      <input type="password" inputMode="numeric" autoComplete="off" dir="ltr" maxLength={8} value={value} onChange={(e) => onChange(e.target.value)} className={`${field} font-num tracking-[0.3em]`} />
    </label>
  );
}

/** «قفل برنامه»: set, change or remove the PIN, and when it locks again. */
export function AppLock() {
  const [v, setV] = useState<LockView | null>(null);
  const [current, setCurrent] = useState("");
  const [pin, setPin] = useState("");
  const [again, setAgain] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);

  useEffect(() => {
    void lock.state().then(setV, (e) => setError(errorText(e)));
  }, []);

  const run = (p: Promise<LockView>, ok: string) =>
    p.then(
      (x) => {
        setV(x);
        setError(null);
        setDone(ok);
        setCurrent("");
        setPin("");
        setAgain("");
      },
      (e) => {
        setError(errorText(e));
        setDone(null);
      },
    );

  const enabled = v?.enabled ?? false;
  const match = pin === again;
  const canSave = pinOk(pin) && match && (!enabled || pinOk(current));

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
      <PageHeader micro="App lock" title="قفل برنامه" subtitle="تنظیمات ← قفل · کسی که پشت این کامپیوتر می‌نشیند حساب و لینک‌هایت را نبیند" />
      {error && <span role="alert" className="text-[13px] text-bad">{error}</span>}
      {done && <span role="status" className="text-[13px] text-ok">{done}</span>}
      <div className="flex min-h-0 flex-1 gap-10 overflow-y-auto pb-2 pe-1 [scrollbar-width:thin] max-[1100px]:flex-col max-[1100px]:gap-5">
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <div className="flex flex-col gap-2">
            <span className="micro text-[9px] tracking-[0.22em]">PIN</span>
            <span className="text-sm font-bold">{enabled ? "تغییر رمز" : "گذاشتن رمز"}</span>
            <span className="text-[11px] leading-[1.8] text-ink-2">
              ۴ تا ۸ رقم. با اجرای برنامه و بعد از مدت بیکاری انتخاب‌شده، برنامه قفل می‌شود. VPN و منوی کنار ساعت در حالت قفل هم کار می‌کنند.
            </span>
            {enabled && <PinInput label="رمز فعلی" value={current} onChange={setCurrent} />}
            <div className="flex gap-2.5">
              <PinInput label="رمز تازه" value={pin} onChange={setPin} />
              <PinInput label="تکرار رمز" value={again} onChange={setAgain} />
            </div>
            {again && !match && <span className="text-[11px] text-bad">دو رمز یکی نیستند.</span>}
            <Button kind="soft" height={42} className="self-start" disabled={!canSave} onClick={() => void run(lock.setPin(pin, enabled ? current : undefined), enabled ? "رمز عوض شد." : "قفل روشن شد.")}>
              {enabled ? "ذخیره‌ی رمز تازه" : "روشن کردن قفل"}
            </Button>
          </div>
        </div>
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <div className="flex flex-col gap-2">
            <span className="micro text-[9px] tracking-[0.22em]">Auto lock</span>
            <span className="text-sm font-bold">قفل دوباره بعد از بیکاری</span>
            <Segmented label="قفل بعد از بیکاری" height={40} options={IDLE} value={String(v?.idleMinutes ?? 0) as (typeof IDLE)[number]["value"]} onChange={(m) => void run(lock.setIdle(+m), "ذخیره شد.")} />
            <span className="text-[11px] leading-[1.8] text-ink-2">وقتی برنامه کنار ساعت است هم بیکاری حساب می‌شود.</span>
          </div>
          {enabled && (
            <Group micro="Lock">
              <Row title="قفل کن" hint="همین الان برنامه را قفل کن" trailing={<Button kind="soft" height={34} onClick={() => void lock.now()}>قفل</Button>} />
              <Row
                title="برداشتن قفل"
                hint="رمز فعلی را در خانه‌ی «رمز فعلی» بنویس"
                trailing={
                  <Button kind="danger" height={34} disabled={!pinOk(current)} onClick={() => void run(lock.disable(current), "قفل برداشته شد.")}>
                    برداشتن
                  </Button>
                }
              />
            </Group>
          )}
        </div>
      </div>
    </div>
  );
}
