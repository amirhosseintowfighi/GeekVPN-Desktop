import { useEffect, useState } from "react";
import { Button, Segmented, Switch } from "../design-system/controls";
import { Group, PageHeader, Row } from "../design-system/layout";
import { field } from "./Payment";
import { errorText } from "../lib/auth";
import { networks, OFFLINE, type NetAction, type NetworksView } from "../lib/networks";

type Choice = NetAction | "none";

const CHOICES: readonly { value: Choice; label: string }[] = [
  { value: "none", label: "کاری نکن" },
  { value: "connect", label: "وصل شو" },
  { value: "trusted", label: "شبکه‌ی امن" },
];

const ACTION_TEXT: Record<NetAction, string> = {
  connect: "با وصل شدن به این شبکه، VPN روشن می‌شود",
  trusted: "شبکه‌ی امن: با وصل شدن به آن، VPN قطع می‌شود",
};

/** «شبکه‌ها»: connect on cafés and airports, stay off at home. */
export function Networks() {
  const [v, setV] = useState<NetworksView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");

  const run = (p: Promise<NetworksView>) =>
    p.then(
      (x) => {
        setV(x);
        setError(null);
      },
      (e) => setError(errorText(e)),
    );

  useEffect(() => {
    void run(networks.view());
    // The computer can move to another network while the page is open.
    const t = setInterval(() => void networks.view().then(setV, () => {}), 8000);
    return () => clearInterval(t);
  }, []);

  const current = v?.current;
  const rule = v?.rules.find((r) => r.key === current?.key);
  const known = current && current.key !== OFFLINE;
  useEffect(() => {
    setName(rule?.name ?? (current ? `${current.label}` : ""));
  }, [rule?.name, current?.key, current?.label]);

  const choose = (c: Choice) => {
    if (!current) return;
    void run(networks.setRule(current.key, name, c === "none" ? null : c));
  };

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
      <PageHeader micro="Networks" title="شبکه‌ها" subtitle="تنظیمات ← شبکه‌ها · VPN با عوض شدن شبکه خودش روشن یا خاموش شود" />
      {error && <span role="alert" className="text-[13px] text-bad">{error}</span>}
      <div className="flex min-h-0 flex-1 gap-10 overflow-y-auto pb-2 pe-1 [scrollbar-width:thin] max-[1100px]:flex-col max-[1100px]:gap-5">
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <div className="flex flex-col gap-2">
            <span className="micro text-[9px] tracking-[0.22em]">This network</span>
            <span className="text-sm font-bold">{known ? `الان روی: ${rule?.name ?? current.label}` : "به شبکه‌ای وصل نیستی"}</span>
            <span className="text-[11px] leading-[1.8] text-ink-2">
              شبکه از روی مودم (روتر) و DNSی که می‌دهد شناخته می‌شود؛ نام Wi-Fi خوانده نمی‌شود چون ویندوز و مک برایش اجازه‌ی موقعیت مکانی می‌خواهند. برای همین یک اسم برایش بگذار.
            </span>
            {known && (
              <>
                <label className="flex flex-col gap-1.5 text-xs text-ink-2">
                  اسم این شبکه
                  <input dir="auto" value={name} maxLength={40} onChange={(e) => setName(e.target.value)} placeholder="مثلاً Wi-Fi خانه" className={field} />
                </label>
                <Segmented label="کار روی این شبکه" height={42} options={CHOICES} value={rule?.action ?? "none"} onChange={choose} />
                {rule && name.trim() && name.trim() !== rule.name && (
                  <Button kind="soft" height={40} className="self-start" onClick={() => choose(rule.action)}>
                    ذخیره‌ی اسم
                  </Button>
                )}
                <span className="text-[11px] leading-[1.8] text-ink-2">
                  فقط وقتی شبکه عوض می‌شود کاری انجام می‌شود؛ اگر روی همین شبکه خودت وصل یا قطع کنی، برنامه دست نمی‌زند. «اتصال خودکار بعد از اجرا» هم روی شبکه‌ی امن وصل نمی‌شود.
                </span>
              </>
            )}
          </div>
          <Group micro="New networks">
            <Row
              title="وصل شدن در شبکه‌های جدید"
              hint="روی هر شبکه‌ای که هنوز برایش کاری انتخاب نکرده‌ای (Wi-Fi کافه، فرودگاه، هتل) VPN خودکار روشن شود"
              trailing={<Switch label="وصل شدن در شبکه‌های جدید" checked={v?.connectOnNew ?? false} onChange={(on) => void run(networks.connectOnNew(on))} />}
            />
          </Group>
        </div>

        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <Group micro="Saved networks">
            {v?.rules.length === 0 && <span className="py-3 text-xs text-ink-2">هنوز شبکه‌ای ذخیره نشده.</span>}
            {v?.rules.map((r) => (
              <Row
                key={r.key}
                title={r.key === current?.key ? `${r.name} · همین الان` : r.name}
                hint={ACTION_TEXT[r.action]}
                trailing={
                  <Button kind="soft" height={34} onClick={() => void run(networks.setRule(r.key, r.name, null))}>
                    حذف
                  </Button>
                }
              />
            ))}
          </Group>
        </div>
      </div>
    </div>
  );
}
