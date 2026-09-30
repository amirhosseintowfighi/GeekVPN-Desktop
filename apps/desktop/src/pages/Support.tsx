import { useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router";
import { Badge, Button, Segmented } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { CardTitle, EmptyState, PageHeader } from "../design-system/layout";
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
      <div className="whitespace-pre-line rounded-[18px_18px_6px_18px] bg-soft-button px-3.5 py-3 text-[13px] leading-[1.9] text-ink">{m.bodyFa}</div>
      <span className="text-[11px] text-ink-2">پشتیبانی · {when(m.createdAt)}</span>
    </div>
  ) : (
    <div className="flex max-w-[70%] flex-col gap-1 self-start">
      <div className="whitespace-pre-line rounded-[18px_18px_18px_6px] bg-action px-3.5 py-3 text-[13px] leading-[1.9] text-on-action">{m.bodyFa}</div>
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
    <div className="flex flex-1 flex-col gap-3 overflow-y-auto p-[18px]">
      <CardTitle title="تیکت جدید" subtitle="جواب را همین‌جا و در ربات تلگرام می‌گیری" />
      <span className="text-xs font-bold text-ink-2">موضوع</span>
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
          className="min-h-[140px] flex-1 resize-none rounded-[14px] border border-chip-border bg-chip p-3 text-[13px] leading-[1.9] text-ink outline-none focus:border-link"
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
        <PageHeader title="پشتیبانی" />
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

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader title="پشتیبانی" subtitle="هر ۲۰ ثانیه تازه می‌شود" />
      <div className="flex min-h-0 flex-1 gap-4">
        <section className="glass-milk flex w-[330px] shrink-0 flex-col gap-2 rounded-[26px] p-3.5">
          <CardTitle title="تیکت‌های من" subtitle="جواب‌ها در ربات تلگرام هم می‌آیند" />
          <Button icon="plus" onClick={() => setComposing(true)}>
            تیکت جدید
          </Button>
          <div className="-mx-1 flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto px-1">
            {tickets === null && !error && <span className="p-3 text-[13px] text-ink-2">در حال خواندن…</span>}
            {tickets?.length === 0 && <span className="p-3 text-center text-[13px] leading-[1.9] text-ink-2">هنوز تیکتی نداری.</span>}
            {tickets?.map((t) => {
              const sel = t.ticketId === openId && !composing;
              const st = TICKET_STATE[t.state] ?? { label: t.state, tone: "link" as const };
              return (
                <button
                  key={t.ticketId}
                  type="button"
                  aria-pressed={sel}
                  onClick={() => {
                    setComposing(false);
                    setOpenId(t.ticketId);
                  }}
                  className={`flex flex-col gap-1.5 rounded-2xl px-3.5 py-3 text-start text-ink ${sel ? "border-2 border-link bg-soft" : "border-2 border-transparent hover:bg-soft/60"}`}
                >
                  <span className="flex items-center gap-2">
                    <span className="flex-1 truncate text-sm font-extrabold">{t.topicFa || "تیکت"}</span>
                    {t.unreadCount > 0 && <Badge tone="bad">{faDigits(t.unreadCount)} جدید</Badge>}
                  </span>
                  <span className="flex items-center gap-2">
                    <span dir="ltr" className="flex-1 text-right font-num text-[11px] text-muted">
                      {t.reference}
                    </span>
                    <Badge tone={st.tone}>{st.label}</Badge>
                  </span>
                </button>
              );
            })}
          </div>
          <Link to="/report" className="flex items-center gap-2.5 rounded-2xl bg-soft-button p-3 text-ink no-underline">
            <span className="flex h-9 w-9 items-center justify-center rounded-[11px] bg-warn-soft text-warn">
              <Icon name="flag" size={18} />
            </span>
            <span className="flex flex-col">
              <span className="text-[13px] font-extrabold">گزارش مشکل اتصال</span>
              <span className="text-[11px] text-ink-2">با گزارش فنی حذف‌شده از اطلاعات حساس</span>
            </span>
          </Link>
        </section>

        <section className="glass-milk flex min-w-0 flex-1 flex-col overflow-hidden rounded-[26px] text-ink">
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
            <p className="m-auto p-6 text-center text-[13px] leading-[1.9] text-ink-2">یک تیکت را انتخاب کن یا تیکت جدید بساز.</p>
          ) : (
            <>
              <div className="flex items-center gap-2.5 border-b border-hair px-[18px] py-3.5">
                <span className="flex flex-1 flex-col">
                  <span className="text-base font-extrabold">{current.topicFa || "تیکت"}</span>
                  <span dir="ltr" className="text-right font-num text-[11px] text-muted">
                    {current.reference}
                  </span>
                </span>
                <Badge tone={(TICKET_STATE[current.state] ?? { tone: "link" }).tone}>{TICKET_STATE[current.state]?.label ?? current.state}</Badge>
              </div>
              <div className="flex flex-1 flex-col gap-3.5 overflow-y-auto p-[18px]">
                {thread === null ? <span className="text-[13px] text-ink-2">در حال خواندن…</span> : thread.map((m) => <Bubble key={m.messageId} m={m} />)}
                <div ref={end} />
              </div>
              {error && (
                <span role="alert" className="px-4 pb-1 text-xs text-bad">
                  {error}
                </span>
              )}
              {closed ? (
                <p className="m-0 border-t border-hair p-3 text-center text-xs text-ink-2">این تیکت بسته شده؛ برای موضوع تازه، تیکت جدید بساز.</p>
              ) : (
                <div className="flex items-end gap-2 border-t border-hair p-3">
                  <textarea
                    aria-label="جوابت"
                    value={reply}
                    maxLength={4000}
                    onChange={(e) => setReply(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && (e.ctrlKey || e.metaKey) && reply.trim().length >= MIN_MESSAGE) void send();
                    }}
                    placeholder="جوابت را بنویس… (حداقل ۱۰ حرف)"
                    className="h-12 flex-1 resize-none rounded-[14px] border border-chip-border bg-chip p-3 text-[13px] text-ink outline-none focus:border-link"
                  />
                  <Button icon="send" height={48} disabled={busy || reply.trim().length < MIN_MESSAGE} onClick={() => void send()}>
                    ارسال
                  </Button>
                </div>
              )}
            </>
          )}
        </section>
      </div>
    </div>
  );
}
