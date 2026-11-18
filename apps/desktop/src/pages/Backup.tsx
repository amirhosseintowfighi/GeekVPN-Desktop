import { open, save } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { Button } from "../design-system/controls";
import { PageHeader } from "../design-system/layout";
import { field } from "./Payment";
import { errorText } from "../lib/auth";
import { backup, NEEDS_PASSWORD } from "../lib/backup";
import { faDigits } from "../lib/fa";

function Password({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  return (
    <label className="flex min-w-0 flex-1 flex-col gap-1.5 text-xs text-ink-2">
      {label}
      <input type="password" autoComplete="new-password" dir="ltr" value={value} onChange={(e) => onChange(e.target.value)} className={field} />
    </label>
  );
}

/** «پشتیبان‌گیری»: settings and hand-added links to a file, and back. */
export function Backup() {
  const [pw, setPw] = useState("");
  const [pw2, setPw2] = useState("");
  const [saveNote, setSaveNote] = useState<{ ok: boolean; text: string } | null>(null);

  const [file, setFile] = useState<string | null>(null);
  const [askPw, setAskPw] = useState(false);
  const [openPw, setOpenPw] = useState("");
  const [restoreNote, setRestoreNote] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  const doSave = async () => {
    setSaveNote(null);
    const day = new Date().toISOString().slice(0, 10);
    const path = await save({ title: "ذخیره‌ی پشتیبان", defaultPath: `geekvpn-${day}.geekvpn`, filters: [{ name: "GeekVPN", extensions: ["geekvpn"] }] });
    if (typeof path !== "string") return;
    setBusy(true);
    try {
      await backup.save(path, pw || null);
      setSaveNote({ ok: true, text: pw ? "پشتیبان با رمز ذخیره شد." : "پشتیبان ذخیره شد. این فایل لینک‌هایت را دارد؛ جای امنی نگهش دار." });
      setPw("");
      setPw2("");
    } catch (e) {
      setSaveNote({ ok: false, text: errorText(e) });
    }
    setBusy(false);
  };

  const doRestore = async (path: string, password: string | null) => {
    setBusy(true);
    setRestoreNote(null);
    try {
      const s = await backup.restore(path, password);
      setRestoreNote({ ok: true, text: `بازگردانی شد: ${faDigits(s.linksAdded)} لینک تازه و ${faDigits(s.rules)} قانون. تنظیمات از اتصال بعدی اعمال می‌شوند.` });
      setAskPw(false);
      setOpenPw("");
      setFile(null);
    } catch (e) {
      const msg = errorText(e);
      if (msg === NEEDS_PASSWORD) {
        setFile(path);
        setAskPw(true);
      } else {
        setRestoreNote({ ok: false, text: msg });
      }
    }
    setBusy(false);
  };

  const pick = async () => {
    const path = await open({ title: "انتخاب فایل پشتیبان", multiple: false, filters: [{ name: "GeekVPN", extensions: ["geekvpn"] }] });
    if (typeof path === "string") await doRestore(path, null);
  };

  const pwMismatch = pw2 !== "" && pw !== pw2;

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
      <PageHeader micro="Backup" title="پشتیبان‌گیری و بازگردانی" subtitle="تنظیمات ← پشتیبان · برای کامپیوتر تازه یا بعد از نصب دوباره" />
      <div className="flex min-h-0 flex-1 gap-10 overflow-y-auto pb-2 pe-1 [scrollbar-width:thin] max-[1100px]:flex-col max-[1100px]:gap-5">
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-3">
          <span className="micro text-[9px] tracking-[0.22em]">Export</span>
          <span className="text-sm font-bold">ذخیره‌ی پشتیبان</span>
          <span className="text-[11px] leading-[1.9] text-ink-2">
            لینک‌ها و کانفیگ‌هایی که دستی اضافه کرده‌ای، علاقه‌مندی‌ها، قانون‌ها، تونل برنامه‌ای، تنظیمات پیشرفته، شبکه‌ها و بقیه‌ی تنظیمات. سرویس‌های حساب در فایل نیستند (با ورود برمی‌گردند) و رمز قفل برنامه هم نه.
          </span>
          <div className="flex gap-2.5">
            <Password label="رمز فایل (اختیاری، پیشنهادی)" value={pw} onChange={setPw} />
            <Password label="تکرار رمز" value={pw2} onChange={setPw2} />
          </div>
          {pwMismatch && <span className="text-[11px] text-bad">دو رمز یکی نیستند.</span>}
          <span className="text-[11px] leading-[1.8] text-ink-2">با رمز، فایل رمزنگاری می‌شود و بدون آن باز نمی‌شود. رمز را فراموش نکن؛ راه بازیابی ندارد.</span>
          <Button kind="white" icon="dl" height={42} className="self-start" disabled={busy || pwMismatch || (pw !== "" && pw2 === "")} onClick={() => void doSave()}>
            ذخیره در فایل
          </Button>
          {saveNote && <span role="status" className={`text-[12px] ${saveNote.ok ? "text-ok" : "text-bad"}`}>{saveNote.text}</span>}
        </div>

        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-3">
          <span className="micro text-[9px] tracking-[0.22em]">Restore</span>
          <span className="text-sm font-bold">بازگردانی از فایل</span>
          <span className="text-[11px] leading-[1.9] text-ink-2">تنظیمات با تنظیمات فایل جایگزین می‌شوند و لینک‌های فایل کنار لینک‌های فعلی اضافه می‌شوند (تکراری‌ها یک بار).</span>
          <Button kind="soft" icon="file" height={42} className="self-start" disabled={busy} onClick={() => void pick()}>
            انتخاب فایل پشتیبان
          </Button>
          {askPw && file && (
            <div className="flex items-end gap-2.5">
              <Password label="این پشتیبان رمز دارد" value={openPw} onChange={setOpenPw} />
              <Button kind="soft" height={46} disabled={busy || !openPw} onClick={() => void doRestore(file, openPw)}>
                باز کن
              </Button>
            </div>
          )}
          {restoreNote && <span role="status" className={`text-[12px] leading-[1.8] ${restoreNote.ok ? "text-ok" : "text-bad"}`}>{restoreNote.text}</span>}
        </div>
      </div>
    </div>
  );
}
