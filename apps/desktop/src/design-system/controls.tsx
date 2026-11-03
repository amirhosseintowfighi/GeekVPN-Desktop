import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Icon, type IconName } from "./Icon";

type ButtonKind = "action" | "soft" | "accent" | "danger" | "white" | "clear";

// Foundations: solid white for the action, frosted glass for the rest.
const BUTTON_KIND: Record<ButtonKind, string> = {
  action: "bg-action text-on-action font-extrabold shadow-[var(--gv-action-shadow)]",
  soft: "glass-milk font-bold",
  accent: "bg-light text-white font-extrabold",
  danger: "bg-bad-soft text-bad border border-bad-border font-bold",
  white: "bg-white text-navy font-extrabold",
  clear: "glass-clear font-bold",
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
      className={`flex items-center justify-center gap-2 whitespace-nowrap rounded-full px-[18px] text-sm transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-55 ${
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
  const look = kind === "soft" ? "glass-milk" : kind === "clear" ? "glass-clear" : "bg-action text-on-action";
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      {...rest}
      style={{ width: size, height: size }}
      className={`flex shrink-0 items-center justify-center rounded-full disabled:opacity-55 ${look}`}
    >
      <Icon name={icon} size={19} />
    </button>
  );
}

/** Foundations' pill switch: white with a navy knob when on. `onNavy` is kept for callers; the look is one. */
export function Switch({
  checked,
  onChange,
  label,
  onNavy,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  onNavy?: boolean;
}) {
  void onNavy;
  const track = checked ? "justify-start bg-white" : "justify-end bg-white/16 border border-white/30";
  const knob = checked ? "bg-navy" : "bg-white";
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`box-border flex h-[26px] w-11 shrink-0 items-center rounded-full p-[3px] transition-colors ${track}`}
    >
      <span className={`h-5 w-5 rounded-full ${knob}`} />
    </button>
  );
}

/** One-of-n picker in a glass pill (route mode, failover threshold). */
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
    <div role="radiogroup" aria-label={label} style={{ height }} className="glass-milk box-border flex items-center gap-1 rounded-full px-1">
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
            className={`flex-1 basis-0 whitespace-nowrap rounded-full text-[13px] transition-colors ${
              on ? "bg-white font-extrabold text-navy" : "font-bold text-white/78 hover:text-white"
            }`}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

/** Filter chips (white when selected, quiet glass otherwise). */
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
          className={`h-9 whitespace-nowrap rounded-full px-4 text-[13px] ${
            o.value === value ? "bg-white font-extrabold text-navy" : "glass-clear font-bold"
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
    <span className={`flex h-[26px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full px-2.5 text-xs font-extrabold ${TONE[tone]}`}>
      <span className="h-[7px] w-[7px] rounded-full bg-current" />
      {children}
    </span>
  );
}

export function SearchField({ id, placeholder, value, onChange }: { id: string; placeholder: string; value: string; onChange: (v: string) => void }) {
  return (
    <div className="flex h-10 items-center gap-2 rounded-full border border-hair bg-white/8 px-3.5 text-ink-2">
      <label htmlFor={id} className="sr-only">
        {placeholder}
      </label>
      <Icon name="search" size={17} />
      <input
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="min-w-0 flex-1 border-none bg-transparent text-[13px] text-ink outline-none placeholder:text-ink-2"
      />
    </div>
  );
}

/** Space Grotesk micro-label over a section ("Servers", "Home"…). */
export function Micro({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <span className={`micro ${className}`}>{children}</span>;
}

/** The design's hairline rule. */
export function Hair({ vertical }: { vertical?: boolean }) {
  return vertical ? <span aria-hidden="true" className="w-px shrink-0 self-stretch bg-hair" /> : <div aria-hidden="true" className="h-px shrink-0 bg-hair" />;
}

/**
 * A ruled list row (Desktop-Home «DE · آلمان»): a short Latin code, the
 * title with a hint under it, and a value or control at the end.
 */
export function RuleRow({
  code,
  title,
  hint,
  end,
  onClick,
  selected,
  size = 16,
}: {
  code?: string;
  title: ReactNode;
  hint?: ReactNode;
  end?: ReactNode;
  onClick?: () => void;
  selected?: boolean;
  size?: number;
}) {
  const body = (
    <>
      {code !== undefined && (
        <span dir="ltr" className="w-[30px] shrink-0 text-right font-num text-[10px] tracking-[0.18em] text-ink-2">
          {code}
        </span>
      )}
      <span className="flex min-w-0 flex-1 flex-col gap-0.5 text-start">
        <span className="truncate font-bold" style={{ fontSize: size }}>
          {title}
        </span>
        {hint && <span className="truncate text-xs text-ink-2">{hint}</span>}
      </span>
      {end !== undefined && <span className="shrink-0 whitespace-nowrap text-[13px] text-ink-2">{end}</span>}
    </>
  );
  const cls = `flex min-h-6 w-full items-center gap-3 border-b border-hair py-3 last:border-b-0 ${selected ? "-mx-3 rounded-2xl border-transparent bg-white px-3 text-navy [&_span]:text-navy" : ""}`;
  return onClick ? (
    <button type="button" onClick={onClick} aria-pressed={selected} className={`${cls} transition-colors hover:bg-white/5`}>
      {body}
    </button>
  ) : (
    <div className={cls}>{body}</div>
  );
}
