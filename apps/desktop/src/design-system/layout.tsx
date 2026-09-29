import type { ReactNode } from "react";
import { Icon, type IconName } from "./Icon";

/** Milk-glass card, the design's main surface. */
export function Card({ children, className = "", padding = 16 }: { children: ReactNode; className?: string; padding?: number }) {
  return (
    <section style={{ padding }} className={`glass-milk flex flex-col gap-3 rounded-3xl ${className}`}>
      {children}
    </section>
  );
}

export function CardTitle({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  return (
    <div className="flex items-center gap-2">
      <span className="flex flex-1 flex-col">
        <span className="text-base font-extrabold">{title}</span>
        {subtitle && <span className="text-xs text-ink-2">{subtitle}</span>}
      </span>
      {actions}
    </div>
  );
}

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  return (
    <header className="flex items-center gap-3">
      <div className="flex flex-1 flex-col gap-0.5">
        <h1 className="m-0 text-[26px] font-extrabold tracking-[-0.3px]">{title}</h1>
        {subtitle && <span className="text-[13px] opacity-90">{subtitle}</span>}
      </div>
      {actions}
    </header>
  );
}

type RowTone = "link" | "ok" | "warn" | "bad";
const ROW_TONE: Record<RowTone, string> = {
  link: "bg-soft text-link",
  ok: "bg-ok-soft text-ok",
  warn: "bg-warn-soft text-warn",
  bad: "bg-bad-soft text-bad",
};

/** A settings/list row inside a `Group`: icon tile, title, hint, trailing control. */
export function Row({
  icon,
  title,
  hint,
  trailing,
  tone = "link",
  onClick,
}: {
  icon: IconName;
  title: string;
  hint?: ReactNode;
  trailing?: ReactNode;
  tone?: RowTone;
  onClick?: () => void;
}) {
  const body = (
    <>
      <span className={`flex h-[38px] w-[38px] shrink-0 items-center justify-center rounded-xl ${ROW_TONE[tone]}`}>
        <Icon name={icon} size={19} />
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-px text-start">
        <span className="text-sm font-bold text-ink">{title}</span>
        {hint && <span className="text-xs text-ink-2">{hint}</span>}
      </span>
      {trailing ?? (onClick && <span className="flex text-muted"><Icon name="chev" size={18} /></span>)}
    </>
  );
  const cls = "flex w-full items-center gap-3 border-b border-hair px-3.5 py-[11px] last:border-b-0";
  return onClick ? (
    <button type="button" onClick={onClick} className={cls}>
      {body}
    </button>
  ) : (
    <div className={cls}>{body}</div>
  );
}

export function Group({ label, children }: { label?: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      {label && <span className="text-[13px] font-bold text-white/90">{label}</span>}
      <div className="glass-milk overflow-hidden rounded-[22px]">{children}</div>
    </div>
  );
}

/**
 * What a screen shows when it has nothing to show yet: said plainly, with the
 * one thing the user can do about it.
 */
export function EmptyState({ icon, title, text, action }: { icon: IconName; title: string; text: string; action?: ReactNode }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
      <span className="glass-clear flex h-16 w-16 items-center justify-center rounded-[21px]">
        <Icon name={icon} size={30} stroke={1.8} />
      </span>
      <span className="text-lg font-extrabold">{title}</span>
      <span className="max-w-[420px] text-[13px] leading-[1.9] opacity-90">{text}</span>
      {action}
    </div>
  );
}
