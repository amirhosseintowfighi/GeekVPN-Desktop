import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Icon, type IconName } from "./Icon";

type ButtonKind = "action" | "soft" | "accent" | "danger" | "white" | "clear";

const BUTTON_KIND: Record<ButtonKind, string> = {
  action: "bg-action text-on-action shadow-[var(--gv-action-shadow)]",
  soft: "bg-soft-button text-ink",
  accent: "bg-logo text-[#062845]",
  danger: "bg-bad-soft text-bad border border-bad-border",
  white: "bg-white text-[#062845]",
  clear: "glass-clear",
};

export function Button({
  kind = "action",
  icon,
  height = 44,
  grow,
  children,
  className = "",
  ...rest
}: {
  kind?: ButtonKind;
  icon?: IconName;
  height?: number;
  grow?: boolean;
  children: ReactNode;
} & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      {...rest}
      style={{ height }}
      className={`flex items-center justify-center gap-2 rounded-[14px] px-4 text-sm font-bold disabled:cursor-not-allowed disabled:opacity-55 ${
        grow ? "flex-1 basis-0" : ""
      } ${BUTTON_KIND[kind]} ${className}`}
    >
      {icon && <Icon name={icon} size={18} />}
      <span>{children}</span>
    </button>
  );
}

export function IconButton({
  icon,
  label,
  kind = "soft",
  size = 38,
  ...rest
}: {
  icon: IconName;
  label: string;
  kind?: "soft" | "clear" | "action";
  size?: number;
} & ButtonHTMLAttributes<HTMLButtonElement>) {
  const look = kind === "soft" ? "bg-soft text-link" : kind === "clear" ? "glass-clear" : "bg-action text-on-action";
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      {...rest}
      style={{ width: size, height: size, borderRadius: size < 44 ? 13 : 15 }}
      className={`flex shrink-0 items-center justify-center disabled:opacity-55 ${look}`}
    >
      <Icon name={icon} size={19} />
    </button>
  );
}

export function Switch({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`flex h-[30px] w-[50px] shrink-0 items-center rounded-[9px] p-[3px] transition-colors ${
        checked ? "justify-end bg-action" : "justify-start bg-track"
      }`}
    >
      <span
        className={`h-6 w-6 rounded-[7px] ${checked ? "bg-action-accent" : "bg-white shadow-[0_1px_3px_rgba(0,0,0,0.2)]"}`}
      />
    </button>
  );
}

/** One-of-n picker on milk glass (route mode, theme, failover threshold). */
export function Segmented<T extends string>({
  options,
  value,
  onChange,
  height = 40,
  label,
}: {
  options: readonly { value: T; label: string }[];
  value: T;
  onChange: (v: T) => void;
  height?: number;
  label: string;
}) {
  return (
    <div role="radiogroup" aria-label={label} className="flex gap-1 rounded-[14px] bg-soft-button p-1">
      {options.map((o) => {
        const on = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={on}
            onClick={() => onChange(o.value)}
            style={{ height: height - 8 }}
            className={`flex-1 basis-0 rounded-[11px] text-[13px] font-bold transition-colors ${
              on ? "bg-action text-on-action" : "text-ink"
            }`}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

/** Filter chips on the blue backdrop (white when selected, clear glass otherwise). */
export function ClearChips<T extends string>({
  options,
  value,
  onChange,
}: {
  options: readonly { value: T; label: string }[];
  value: T;
  onChange: (v: T) => void;
}) {
  return (
    <div className="flex gap-1.5">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          aria-pressed={o.value === value}
          onClick={() => onChange(o.value)}
          className={`h-9 rounded-xl px-3.5 text-[13px] font-bold ${
            o.value === value ? "bg-white text-[#062845]" : "glass-clear"
          }`}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

type Tone = "ok" | "warn" | "bad" | "link";
const TONE: Record<Tone, string> = {
  ok: "bg-ok-soft text-ok",
  warn: "bg-warn-soft text-warn",
  bad: "bg-bad-soft text-bad",
  link: "bg-soft text-link",
};

export function Badge({ tone = "ok", children }: { tone?: Tone; children: ReactNode }) {
  return (
    <span className={`flex h-[26px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-[9px] px-2.5 text-xs font-extrabold ${TONE[tone]}`}>
      <span className="h-[7px] w-[7px] rounded-[2px] bg-current" />
      {children}
    </span>
  );
}

/** Four signal bars for a real-delay result; `ms` 0 or less means no answer. */
export function PingBars({ ms }: { ms: number }) {
  const level = ms <= 0 ? 0 : ms < 150 ? 4 : ms < 250 ? 3 : ms < 450 ? 2 : 1;
  const color = level >= 3 ? "var(--gv-ok)" : level === 2 ? "var(--gv-warn)" : "var(--gv-bad)";
  return (
    <span className="flex items-center gap-2">
      <span dir="ltr" className="font-num text-xs font-semibold text-ink-2">
        {ms > 0 ? `${ms}ms` : "—"}
      </span>
      <span aria-hidden="true" dir="ltr" className="flex h-[17px] items-end gap-0.5">
        {[5, 9, 13, 17].map((h, i) => (
          <span key={h} className="w-1 rounded-sm" style={{ height: h, background: i < level ? color : "var(--gv-track)" }} />
        ))}
      </span>
    </span>
  );
}

export function CountryBadge({ code, selected, size = 40 }: { code: string; selected?: boolean; size?: number }) {
  return (
    <span
      style={{ width: size, height: size }}
      className={`flex shrink-0 items-center justify-center rounded-xl font-num text-xs font-bold tracking-[0.5px] ${
        selected ? "bg-action text-on-action" : "bg-soft text-ink"
      }`}
    >
      {code}
    </span>
  );
}

export function SearchField({ id, placeholder, value, onChange }: { id: string; placeholder: string; value: string; onChange: (v: string) => void }) {
  return (
    <div className="flex h-10 items-center gap-2 rounded-xl bg-soft-button px-3 text-muted">
      <label htmlFor={id} className="sr-only">
        {placeholder}
      </label>
      <Icon name="search" size={17} />
      <input
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="min-w-0 flex-1 border-none bg-transparent text-[13px] text-ink outline-none"
      />
    </div>
  );
}
