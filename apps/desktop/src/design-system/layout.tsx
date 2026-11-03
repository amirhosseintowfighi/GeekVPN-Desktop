import type { ReactNode } from "react";
import { Icon, type IconName } from "./Icon";

/** Frosted-glass panel, the design's side cards. */
export function Card({ children, className = "", padding = 20 }: { children: ReactNode; className?: string; padding?: number }) {
  return (
    <section style={{ padding }} className={`glass-milk flex flex-col gap-3 rounded-[26px] ${className}`}>
      {children}
    </section>
  );
}

export function CardTitle({ title, subtitle, actions, micro }: { title: string; subtitle?: string; actions?: ReactNode; micro?: string }) {
  return (
    <div className="flex items-center gap-2">
      <span className="flex flex-1 flex-col gap-0.5">
        {micro && <span className="micro">{micro}</span>}
        <span className="text-[17px] font-extrabold">{title}</span>
        {subtitle && <span className="text-xs text-ink-2">{subtitle}</span>}
      </span>
      {actions}
    </div>
  );
}

/**
 * The top of every page: a Latin micro-label, the Persian title at 40px, a
 * line under it, and a hairline across (Desktop-Report and the rest).
 */
export function PageHeader({ title, subtitle, actions, micro }: { title: ReactNode; subtitle?: ReactNode; actions?: ReactNode; micro?: string }) {
  return (
    <header className="flex shrink-0 flex-col gap-[18px]">
      <div className="flex flex-wrap items-end justify-between gap-x-5 gap-y-3">
        <div className="flex min-w-[220px] flex-1 flex-col gap-1.5">
          {micro && <span className="micro">{micro}</span>}
          <h1 className="m-0 whitespace-nowrap text-[40px] font-extrabold leading-[1.2] max-[1150px]:text-[34px]">{title}</h1>
          {subtitle && <span className="text-sm leading-[1.7] text-ink-2">{subtitle}</span>}
        </div>
        {actions && <div className="flex flex-wrap items-center justify-end gap-2.5">{actions}</div>}
      </div>
      <div aria-hidden="true" className="h-px bg-hair" />
    </header>
  );
}

type RowTone = "link" | "ok" | "warn" | "bad";

/**
 * A ruled row (Desktop-Settings, Desktop-Account): the title, a hint under
 * it and the control at the end, between hairlines. `icon` and `tone` are
 * kept for callers; the design draws no icon tile.
 */
export function Row({
  title,
  hint,
  trailing,
  onClick,
}: {
  icon?: IconName;
  title: string;
  hint?: ReactNode;
  trailing?: ReactNode;
  tone?: RowTone;
  onClick?: () => void;
}) {
  const body = (
    <>
      <span className="flex min-w-0 flex-1 flex-col gap-0.5 text-start">
        <span className="text-sm font-bold text-ink">{title}</span>
        {hint && <span className="text-[11px] leading-[1.7] text-ink-2">{hint}</span>}
      </span>
      {trailing ?? (onClick && <span className="flex text-ink-2"><Icon name="chev" size={16} /></span>)}
    </>
  );
  const cls = "flex w-full items-center gap-3 border-b border-hair py-2.5 last:border-b-0";
  return onClick ? (
    <button type="button" onClick={onClick} className={`${cls} transition-colors hover:bg-white/5`}>
      {body}
    </button>
  ) : (
    <div className={cls}>{body}</div>
  );
}

/** A section of ruled rows under a micro-label. */
export function Group({ label, micro, children }: { label?: string; micro?: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      {micro ? <span className="micro text-[9px] tracking-[0.22em]">{micro}</span> : label && <span className="text-xs font-bold text-ink-2">{label}</span>}
      <div className="flex flex-col border-y border-hair">{children}</div>
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
      <span className="glass-milk flex h-16 w-16 items-center justify-center rounded-full">
        <Icon name={icon} size={30} stroke={1.8} />
      </span>
      <span className="text-lg font-extrabold">{title}</span>
      <span className="max-w-[420px] text-[13px] leading-[1.9] text-ink-2">{text}</span>
      {action}
    </div>
  );
}
