/**
 * The blue backdrop: two glows and the hairline rings around the upper one
 * (Foundations). Percent coordinates keep the composition as the window resizes.
 */
export function Backdrop() {
  const rings = [120, 200, 280, 360, 440, 520];
  return (
    <svg aria-hidden="true" className="pointer-events-none absolute inset-0 h-full w-full">
      <circle cx="10%" cy="92%" r="260" style={{ fill: "var(--gv-glow)" }} />
      <circle cx="85%" cy="160" r="190" style={{ fill: "var(--gv-glow-strong)" }} />
      {rings.map((r, i) => (
        <circle key={r} cx="85%" cy="160" r={r} fill="none" stroke="#FFFFFF" strokeOpacity={0.1 - i * 0.014} strokeWidth="1" />
      ))}
    </svg>
  );
}
