import { useEffect, useState } from "react";
import { Link } from "react-router";
import { Icon } from "../design-system/Icon";
import { Card, EmptyState, PageHeader } from "../design-system/layout";
import { bot, type FaqSection, type ServerStatus } from "../lib/account";
import { errorText } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";

const HEALTH: Record<string, { label: string; dot: string }> = {
  healthy: { label: "سالم", dot: "bg-ok" },
  degraded: { label: "کند", dot: "bg-warn" },
  down: { label: "قطع", dot: "bg-bad" },
  maintenance: { label: "در حال تعمیر", dot: "bg-white/50" },
};

/** «وضعیت سرورها» from the bot: the platform's own servers, as the operator sees them. */
function Status() {
  const [rows, setRows] = useState<ServerStatus[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    bot.servers().then(setRows, (e) => setError(errorText(e)));
  }, []);
  return (
    <Card className="w-[330px] shrink-0 self-start max-[1250px]:w-[270px]">
      <span className="micro text-[9px]">Server status</span>
      <span className="text-[17px] font-extrabold">وضعیت سرورها</span>
      {error && <span className="text-xs text-bad">{error}</span>}
      {rows === null && !error && <span className="text-xs text-ink-2">در حال خواندن…</span>}
      {rows?.length === 0 && <span className="text-xs text-ink-2">سروری گزارش نشده.</span>}
      <div className="flex flex-col border-t border-hair">
        {rows?.map((r) => {
          const h = HEALTH[r.health] ?? { label: r.health, dot: "bg-white/50" };
          return (
            <div key={r.name_fa} className="flex items-center gap-3 border-b border-hair py-2.5 last:border-b-0">
              <span className={`h-2 w-2 shrink-0 rounded-full ${h.dot}`} aria-hidden="true" />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-sm font-bold">{r.name_fa}</span>
                {r.location_fa && <span className="truncate text-[11px] text-ink-2">{r.location_fa}</span>}
              </span>
              <span className="text-xs text-ink-2">
                {h.label}
                {r.load_percent !== null ? ` · ${faDigits(r.load_percent)}٪ بار` : ""}
              </span>
            </div>
          );
        })}
      </div>
    </Card>
  );
}

/** «راهنما»: the bot's FAQ and server status (bot handlers faq.py and server_status.py). */
export function Help() {
  const { view, showSignIn } = useAuth();
  const [faq, setFaq] = useState<FaqSection[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const signedIn = Boolean(view?.user);

  useEffect(() => {
    if (signedIn) bot.faq().then(setFaq, (e) => setError(errorText(e)));
  }, [signedIn]);

  if (!signedIn) {
    return (
      <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
        <PageHeader micro="Help" title="راهنما" />
        <EmptyState icon="chat" title="سوالات متداول" text="برای دیدن راهنما و وضعیت سرورها وارد حسابت شو." action={<button type="button" onClick={showSignIn} className="h-10 rounded-full bg-white px-[18px] text-[13px] font-extrabold text-navy">ورود با تلگرام</button>} />
      </div>
    );
  }

  return (
    <>
      <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
        <PageHeader
          micro="Help"
          title="راهنما"
          subtitle="سوالات متداول؛ همان جواب‌هایی که ربات می‌دهد"
          actions={
            <Link to="/support" className="glass-milk flex h-10 items-center gap-2 rounded-full px-4 text-[13px] font-bold text-white no-underline">
              <Icon name="chat" size={16} />
              جوابم را پیدا نکردم
            </Link>
          }
        />
        {error && <span role="alert" className="text-[13px] text-bad">{error}</span>}
        <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto pe-1 [scrollbar-width:thin]">
          {faq === null && !error && <span className="text-[13px] text-ink-2">در حال خواندن…</span>}
          {faq?.map((sec) => (
            <div key={sec.key} className="flex flex-col gap-1">
              <span className="text-lg font-extrabold">{sec.title_fa}</span>
              <div className="flex flex-col border-t border-hair">
                {sec.entries.map((e) => {
                  const id = `${sec.key}/${e.key}`;
                  const shown = open === id;
                  return (
                    <div key={id} className="border-b border-hair">
                      <button type="button" aria-expanded={shown} onClick={() => setOpen(shown ? null : id)} className="flex w-full items-center gap-3 py-3 text-start hover:bg-white/5">
                        <span className="flex-1 text-sm font-bold">{e.question_fa}</span>
                        <span className={`flex text-ink-2 transition-transform ${shown ? "-rotate-90" : ""}`}>
                          <Icon name="chev" size={16} />
                        </span>
                      </button>
                      {shown && <p className="m-0 whitespace-pre-line pb-3.5 text-[13px] leading-[1.9] text-ink-2 select-text">{e.answer_fa}</p>}
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>
      <Status />
    </>
  );
}
