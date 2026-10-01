import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { Button } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { Logo } from "../design-system/Logo";
import { errorText } from "../lib/auth";
import { faDigits } from "../lib/fa";
import { noteLines, updates, type UpdateView } from "../lib/update";

interface UpdateContextValue {
  view: UpdateView | null;
  /** Asks the server again and opens the dialog. */
  openDialog: () => void;
}

const Ctx = createContext<UpdateContextValue>({ view: null, openDialog: () => {} });

export function useUpdate(): UpdateContextValue {
  return useContext(Ctx);
}

function megabytes(n: number): string {
  return faDigits((n / 1024 / 1024).toFixed(1));
}

/** Desktop-Update: the new version, its notes, the download, and the restart. */
function Dialog({ view, checking, error, onClose }: { view: UpdateView | null; checking: boolean; error: string | null; onClose: () => void }) {
  const [progress, setProgress] = useState<{ downloaded: number; total: number | null } | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  useEffect(() => {
    const off = updates.onProgress(setProgress);
    return () => void off.then((f) => f());
  }, []);

  const a = view?.available ?? null;
  const install = async () => {
    setBusy(true);
    setFailed(null);
    try {
      await updates.install();
    } catch (e) {
      setFailed(errorText(e));
      setBusy(false);
    }
  };
  const pct = progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null;
  const canClose = !a?.required && !busy;

  return (
    <div className="absolute inset-0 z-40 flex items-start justify-center bg-[rgba(3,27,51,0.6)] pt-[150px]" onClick={() => canClose && onClose()}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label="به‌روزرسانی برنامه"
        onClick={(e) => e.stopPropagation()}
        className="glass-milk flex w-[480px] flex-col gap-3.5 rounded-[28px] p-6 text-ink"
      >
        <div className="flex items-center gap-3">
          <span className="flex h-[52px] w-[52px] items-center justify-center rounded-2xl bg-logo">
            <Logo size={34} color="#062845" title="" />
          </span>
          <span className="flex flex-1 flex-col">
            <span className="text-[19px] font-extrabold">
              {checking ? "در حال بررسی…" : a ? `نسخه‌ی ${faDigits(a.version)} آماده است` : view?.configured === false ? "به‌روزرسانی خودکار در این نسخه نیست" : "برنامه به‌روز است"}
            </span>
            <span className="text-xs text-ink-2">
              نسخه‌ی فعلی {faDigits(view?.current ?? "")}
              {a ? " · امضاشده" : ""}
              {a?.required ? " · این به‌روزرسانی لازم است" : ""}
            </span>
          </span>
          {canClose && (
            <button type="button" aria-label="بستن" onClick={onClose} className="flex h-9 w-9 items-center justify-center rounded-xl text-muted hover:bg-soft">
              <Icon name="x" size={18} />
            </button>
          )}
        </div>
        {error && (
          <span role="alert" className="text-xs text-bad">
            {error}
          </span>
        )}
        {view?.configured === false && !checking && (
          <span className="text-[13px] leading-[1.9] text-ink-2">این build بدون کلید امضای به‌روزرسانی ساخته شده؛ نسخه‌ی جدید را از سایت GeekVPN بگیر.</span>
        )}
        {a && (
          <>
            {noteLines(a.notes).length > 0 && (
              <>
                <span className="text-[13px] font-extrabold">تغییرات</span>
                <ul className="m-0 max-h-[180px] overflow-y-auto pe-0 ps-[18px] text-[13px] leading-[1.9] text-ink-2">
                  {noteLines(a.notes).map((n) => (
                    <li key={n} className="mb-1.5">
                      {n}
                    </li>
                  ))}
                </ul>
              </>
            )}
            {busy && (
              <div className="flex flex-col gap-1.5">
                <span className="flex justify-between text-xs text-ink-2">
                  <span>{pct === 100 ? "در حال نصب…" : `در حال دانلود${progress ? ` · ${megabytes(progress.downloaded)} مگابایت` : "…"}`}</span>
                  {pct !== null && <span>٪{faDigits(pct)}</span>}
                </span>
                <span className="h-2 overflow-hidden rounded bg-track">
                  <span className="block h-full bg-link transition-[width]" style={{ width: `${pct ?? 8}%` }} />
                </span>
              </div>
            )}
            {failed && (
              <span role="alert" className="text-xs text-bad">
                {failed}
              </span>
            )}
            <div className="flex gap-2.5">
              <Button grow icon="dl" height={48} disabled={busy} onClick={() => void install()}>
                {busy ? "در حال به‌روزرسانی…" : "نصب و راه‌اندازی دوباره"}
              </Button>
              {canClose && (
                <Button kind="soft" height={48} onClick={onClose}>
                  بعداً
                </Button>
              )}
            </div>
            <span className="text-[11px] leading-[1.8] text-ink-2">
              امضای فایل قبل از نصب بررسی می‌شود. اتصال VPN حین نصب قطع و بعد دوباره وصل می‌شود.
            </span>
          </>
        )}
      </div>
    </div>
  );
}

/** Holds what the updater found; a required update opens the dialog itself. */
export function UpdateProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<UpdateView | null>(null);
  const [open, setOpen] = useState(false);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void updates.state().then((v) => {
      setView(v);
      if (v.available?.required) setOpen(true);
    }, () => {});
    const off = updates.onAvailable((a) => {
      setView((v) => ({ configured: true, current: v?.current ?? "", available: a }));
      if (a.required) setOpen(true);
    });
    return () => void off.then((f) => f());
  }, []);

  const openDialog = useCallback(() => {
    setOpen(true);
    setError(null);
    setChecking(true);
    updates
      .check()
      .then(setView, (e) => setError(errorText(e)))
      .finally(() => setChecking(false));
  }, []);

  return (
    <Ctx.Provider value={{ view, openDialog }}>
      {children}
      {open && <Dialog view={view} checking={checking} error={error} onClose={() => setOpen(false)} />}
    </Ctx.Provider>
  );
}
