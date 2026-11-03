import { useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router";
import { Button, Segmented } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { EmptyState, PageHeader } from "../design-system/layout";
import { support, TICKET_STATE, TOPICS, type Ticket, type TicketMessage, type TicketTopic } from "../lib/account";
import { errorText } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";
import { field } from "./Payment";

/** The page refreshes this often while it is open (the design's «هر ۲۰ ثانیه»). */
const POLL_MS = 20_000;
const MIN_MESSAGE = 10;

function when(iso: string): string {
  const d = new Date(iso);
  const today = new Date().toDateString() === d.toDateString();
  const time = d.toLocaleTimeString("fa-IR", { hour: "2-digit", minute: "2-digit" });
  return today ? `امروز ${time}` : `${d.toLocaleDateString("fa-IR", { month: "long", day: "numeric" })} ${time}`;
}

function Bubble({ m }: { m: TicketMessage }) {
  // RTL: the customer's own messages on the start side, support's on the end.
  return m.fromSupport ? (
    <div className="flex max-w-[70%] flex-col items-end gap-1 self-end">
      <div className="glass-milk whitespace-pre-line rounded-[20px_20px_6px_20px] px-4 py-3 text-[13px] leading-[1.9]">{m.bodyFa}</div>
      <span className="text-[11px] text-ink-2">پشتیبانی · {when(m.createdAt)}</span>
    </div>
  ) : (
    <div className="flex max-w-[70%] flex-col gap-1 self-start">
      <div className="whitespace-pre-line rounded-[20px_20px_20px_6px] bg-white px-4 py-3 text-[13px] leading-[1.9] text-navy">{m.bodyFa}</div>
      <span className="text-[11px] text-ink-2">تو · {when(m.createdAt)}</span>
    </div>
  );
}

function NewTicket({ onOpened, onCancel }: { onOpened: (t: Ticket) => void; onCancel: () => void }) {
  const [topic, setTopic] = useState<TicketTopic>("connection");
  const [subject, setSubject] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const send = async () => {
    setBusy(true);
    setError(null);
    try {
      onOpened(await support.open(topic, subject, message));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="flex flex-1 flex-col gap-3 overflow-y-auto">
      <span className="micro text-[9px]">Topic</span>
      <Segmented label="موضوع" options={TOPICS} value={topic} onChange={setTopic} />
      <label className="flex flex-col gap-1.5 text-xs font-bold text-ink-2">
        عنوان (اختیاری)
        <input value={subject} maxLength={128} onChange={(e) => setSubject(e.target.value)} className={field} />
      </label>
      <label className="flex flex-1 flex-col gap-1.5 text-xs font-bold text-ink-2">
        پیام
        <textarea
          value={message}
          maxLength={4000}
          onChange={(e) => setMessage(e.target.value)}
          placeholder="مشکلت را بنویس… (حداقل ۱۰ حرف)"
          className="min-h-[140px] flex-1 resize-none rounded-[18px] border border-white/25 bg-white/7 p-3 text-[13px] leading-[1.9] text-ink outline-none focus:border-link"
        />
      </label>
      {error && (
        <span role="alert" className="text-xs text-bad">
          {error}
        </span>
      )}
      <div className="flex gap-2">
        <Button kind="soft" onClick={onCancel}>
          انصراف
        </Button>
        <Button grow icon="send" disabled={busy || message.trim().length < MIN_MESSAGE} onClick={() => void send()}>
          {busy ? "در حال ارسال…" : "ارسال تیکت"}
        </Button>
      </div>
    </div>
  );
}

/** Desktop-Support: the tickets on the start side, one thread beside them. */
export function Support() {
  const { view, showSignIn } = useAuth();
  const [tickets, setTickets] = useState<Ticket[] | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const [composing, setComposing] = useState(false);
  const [thread, setThread] = useState<TicketMessage[] | null>(null);
  const [reply, setReply] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const end = useRef<HTMLDivElement>(null);
  const openRef = useRef<string | null>(null);
  openRef.current = openId;
  const signedIn = Boolean(view?.user);

  const loadList = useCallback(() => {
    support.tickets().then(
      (t) => {
        setTickets(t);
        setOpenId((id) => id ?? t[0]?.ticketId ?? null);
      },
      (e) => setError(errorText(e)),
    );
  }, []);
  const loadThread = useCallback((id: string) => {
    support.thread(id).then(
      (m) => {
        setThread(m);
        // Reading marks the answers read: the badge follows.
        loadList();
      },
      (e) => setError(errorText(e)),
    );
  }, [loadList]);

  useEffect(() => {
    if (!signedIn) return;
    loadList();
    const t = setInterval(() => {
      loadList();
      const id = openRef.current;
      if (id) void support.thread(id).then(setThread, () => {});
    }, POLL_MS);
    return () => clearInterval(t);
  }, [signedIn, loadList]);
  useEffect(() => {
    setThread(null);
    if (openId) loadThread(openId);
  }, [openId, loadThread]);
  useEffect(() => end.current?.scrollIntoView({ block: "end" }), [thread]);

  if (!signedIn) {
    return (
      <div className="flex min-w-0 flex-1 flex-col gap-3.5">
        <PageHeader micro="Support" title="پشتیبانی" />
        <EmptyState
          icon="chat"
          title="تیکت‌های من"
          text="برای دیدن تیکت‌ها و گفتگو با پشتیبانی وارد حسابت شو."
          action={
            <Button kind="white" icon="plane" onClick={showSignIn}>
              ورود با تلگرام
            </Button>
          }
        />
      </div>
    );
  }

  const current = tickets?.find((t) => t.ticketId === openId) ?? null;
  const closed = current?.state === "closed";
  const send = async () => {
    if (!openId) return;
    setBusy(true);
    setError(null);
    try {
      const m = await support.reply(openId, reply);
      setThread((t) => [...(t ?? []), m]);
      setReply("");
      loadList();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const st = current ? TICKET_STATE[current.state] ?? { label: current.state, tone: "link" as const } : null;
  return (
    <>
      <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
        <PageHeader
          micro={composing || !current ? "Support" : `Support · ${current.reference}`}
          title={composing ? "تیکت جدید" : current ? current.topicFa || "تیکت" : "پشتیبانی"}
          subtitle={composing ? "جواب را همین‌جا و در ربات تلگرام می‌گیری" : st ? `${st.label} · هر ۲۰ ثانیه تازه می‌شود` : "هر ۲۰ ثانیه تازه می‌شود"}
          actions={
            <>
            <Link to="/help" className="glass-milk flex h-10 items-center gap-2 rounded-full px-4 text-[13px] font-bold text-white no-underline">
              راهنما و وضعیت سرورها
            </Link>
            <Link to="/report" className="glass-milk flex h-10 items-center gap-2 rounded-full px-4 text-[13px] font-bold text-white no-underline">
              <Icon name="flag" size={16} />
              گزارش مشکل اتصال
            </Link>
            </>
          }
        />
        {composing ? (
          <NewTicket
            onCancel={() => setComposing(false)}
            onOpened={(t) => {
              setComposing(false);
              setTickets((l) => [t, ...(l ?? []).filter((x) => x.ticketId !== t.ticketId)]);
              setOpenId(t.ticketId);
            }}
          />
        ) : !current ? (
          <p className="m-auto p-6 text-center text-[13px] leading-[1.9] text-ink-2">{tickets?.length === 0 ? "هنوز تیکتی نداری؛ «تیکت جدید» بساز." : "یک تیکت را انتخاب کن یا تیکت جدید بساز."}</p>
        ) : (
          <>
            <div className="flex min-h-0 flex-1 flex-col gap-3.5 overflow-y-auto pe-1 [scrollbar-width:thin]">
              <div className="flex-1" />
              {thread === null ? <span className="text-[13px] text-ink-2">در حال خواندن…</span> : thread.map((m) => <Bubble key={m.messageId} m={m} />)}
              <div ref={end} />
            </div>
            {error && (
              <span role="alert" className="text-xs text-bad">
                {error}
              </span>
            )}
            {closed ? (
              <p className="m-0 text-center text-xs text-ink-2">این تیکت بسته شده؛ برای موضوع تازه، تیکت جدید بساز.</p>
            ) : (
              <div className="flex items-center gap-2.5">
                <textarea
                  aria-label="جوابت"
                  value={reply}
                  maxLength={4000}
                  onChange={(e) => setReply(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && (e.ctrlKey || e.metaKey) && reply.trim().length >= MIN_MESSAGE) void send();
                  }}
                  placeholder="جوابت را بنویس… (حداقل ۱۰ حرف)"
                  className="h-12 flex-1 resize-none rounded-full border border-white/25 bg-white/7 px-5 py-3 text-[13px] text-ink outline-none focus:border-white/60"
                />
                <Button icon="send" height={48} disabled={busy || reply.trim().length < MIN_MESSAGE} onClick={() => void send()}>
                  ارسال
                </Button>
              </div>
            )}
          </>
        )}
      </div>

      <section className="glass-milk flex w-[300px] shrink-0 flex-col gap-3 rounded-[30px] p-5 max-[1250px]:w-[260px]">
        <span className="text-lg font-extrabold">تیکت‌های من</span>
        <span className="text-[11px] text-ink-2">جواب‌ها در ربات تلگرام هم می‌آیند</span>
        <Button kind="soft" icon="plus" height={40} onClick={() => setComposing(true)}>
          تیکت جدید
        </Button>
        <div className="-mx-1 flex min-h-0 flex-1 flex-col overflow-y-auto px-1 [scrollbar-width:thin]">
          {tickets === null && !error && <span className="p-3 text-[13px] text-ink-2">در حال خواندن…</span>}
          {tickets?.length === 0 && <span className="p-3 text-center text-[13px] leading-[1.9] text-ink-2">هنوز تیکتی نداری.</span>}
          {tickets?.map((t) => {
            const sel = t.ticketId === openId && !composing;
            const tst = TICKET_STATE[t.state] ?? { label: t.state, tone: "link" as const };
            return (
              <button
                key={t.ticketId}
                type="button"
                aria-pressed={sel}
                onClick={() => {
                  setComposing(false);
                  setOpenId(t.ticketId);
                }}
                className={`flex flex-col gap-1 py-3 text-start ${sel ? "-mx-2 rounded-[18px] bg-white px-3.5 text-navy" : "border-b border-hair hover:bg-white/5"}`}
              >
                <span className="flex items-center gap-2">
                  <span className="flex-1 truncate text-sm font-extrabold">{t.topicFa || "تیکت"}</span>
                  {t.unreadCount > 0 && <span className={`text-[11px] font-extrabold ${sel ? "" : "text-blue-text"}`}>{faDigits(t.unreadCount)} جدید</span>}
                </span>
                <span className={`truncate text-[11px] ${sel ? "text-navy/70" : "text-ink-2"}`}>
                  {tst.label} · <span dir="ltr" className="font-num">{t.reference}</span>
                </span>
              </button>
            );
          })}
        </div>
      </section>
    </>
  );
}
