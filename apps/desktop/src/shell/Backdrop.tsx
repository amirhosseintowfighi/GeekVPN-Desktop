/**
 * The ground: almost black, one field of blue light, a vignette and grain
 * (Foundations: «یک میدان نور آبی در هر صفحه»). Each screen of the design has
 * its own field; the shapes are the design's, drawn on its 1280×800 frame and
 * scaled to cover the window.
 */
export type Light = "beam" | "arc" | "bars" | "horizon" | "blob" | "stack";

const BAR_STRIPS: [number, number, number][] = [
  [0, 53, 0.95], [49, 99, 0.89], [140, 94, 0.78], [226, 61, 0.68], [283, 77, 0.61], [353, 74, 0.52],
  [421, 87, 0.44], [500, 95, 0.34], [588, 51, 0.24], [635, 47, 0.18], [678, 98, 0.13], [768, 72, 0.02],
];

const STACK_OPACITY = [0.06, 0.061, 0.065, 0.073, 0.084, 0.099, 0.118, 0.141, 0.169, 0.201, 0.238, 0.279, 0.326, 0.377, 0.433, 0.494, 0.56];

function Field({ light }: { light: Light }) {
  switch (light) {
    case "beam":
      return (
        <>
          <path d="M0 -40 C704 200 -128 496 576 840" fill="none" stroke="#1F6FE0" strokeWidth="256" opacity="0.75" filter="url(#gv-f1)" />
          <path d="M0 -40 C704 200 -128 496 576 840" fill="none" stroke="#4FA2FF" strokeWidth="64" opacity="0.45" filter="url(#gv-f2)" />
        </>
      );
    case "arc":
      return (
        <>
          <ellipse cx="128" cy="200" rx="896" ry="360" fill="#0B2FD0" opacity="0.55" filter="url(#gv-f1)" />
          <circle cx="-154" cy="-40" r="649" fill="#1E5BFF" opacity="0.55" filter="url(#gv-f2)" />
          <circle cx="-154" cy="-40" r="624" fill="url(#gv-arc)" />
          <circle cx="-154" cy="-40" r="624" fill="none" stroke="#BFD8FF" strokeOpacity="0.85" strokeWidth="2.4" filter="url(#gv-f4)" />
        </>
      );
    case "bars":
      return (
        <>
          {BAR_STRIPS.map(([x, w, o]) => (
            <rect key={x} x={x} y="0" width={w} height="800" fill="url(#gv-bar)" opacity={o} filter="url(#gv-fx)" />
          ))}
          <ellipse cx="256" cy="800" rx="576" ry="176" fill="#BFE4FF" opacity="0.45" filter="url(#gv-f2)" />
        </>
      );
    case "horizon":
      return (
        <>
          <ellipse cx="64" cy="760" rx="704" ry="240" fill="#1E5FD6" opacity="0.9" filter="url(#gv-f1)" />
          <ellipse cx="0" cy="800" rx="384" ry="128" fill="#9CD8FF" opacity="0.85" filter="url(#gv-f2)" />
          <ellipse cx="704" cy="120" rx="768" ry="200" fill="#0A1F7A" opacity="0.55" filter="url(#gv-f1)" />
          <path d="M-64 816 C320 640 704 440 1408 288 L1408 960 L-64 960Z" fill="#061650" opacity="0.65" filter="url(#gv-f2)" />
          <path d="M-64 816 C320 640 704 440 1408 288" fill="none" stroke="#3E8BFF" strokeOpacity="0.55" strokeWidth="48" filter="url(#gv-f3)" />
          <path d="M-64 816 C320 640 704 440 1408 288" fill="none" stroke="#D6F1FF" strokeOpacity="0.9" strokeWidth="2.4" filter="url(#gv-f4)" />
        </>
      );
    case "blob":
      return (
        <>
          <path
            d="M553 496 C553 535 609 594 523 624 C438 654 185 683 40 678 C-106 672 -310 634 -350 594 C-389 554 -256 482 -195 436 C-134 390 -103 324 17 316 C136 309 434 361 523 391 C612 421 553 457 553 496Z"
            fill="#3FB8FF"
            opacity="0.65"
            filter="url(#gv-f1)"
          />
          <path
            d="M551 480 C555 519 459 606 380 623 C300 639 148 594 73 581 C-1 568 -13 571 -68 544 C-123 518 -287 455 -258 422 C-229 389 7 352 108 346 C210 340 277 364 351 386 C425 409 546 441 551 480Z"
            fill="#1424D8"
            opacity="0.95"
            filter="url(#gv-f2)"
          />
          <ellipse cx="64" cy="440" rx="154" ry="64" fill="#06106A" opacity="0.9" filter="url(#gv-f2)" />
        </>
      );
    case "stack":
      return (
        <>
          {STACK_OPACITY.map((o, i) => (
            <circle key={o} cx="192" cy={1200 - i * 47.5} r="216" fill="#1A2FFF" opacity={o} />
          ))}
          <ellipse cx="192" cy="480" rx="448" ry="144" fill="#1A2FFF" opacity="0.35" filter="url(#gv-f1)" />
        </>
      );
  }
}

export function Backdrop({ light = "beam" }: { light?: Light }) {
  return (
    <svg aria-hidden="true" viewBox="0 0 1280 800" preserveAspectRatio="xMinYMid slice" className="pointer-events-none absolute inset-0 h-full w-full">
      <defs>
        <radialGradient id="gv-vg" cx="0.5" cy="0.5" r="0.75">
          <stop offset="0.55" stopColor="#000" stopOpacity="0" />
          <stop offset="1" stopColor="#000" stopOpacity="0.55" />
        </radialGradient>
        <filter id="gv-f1" x="-80%" y="-80%" width="260%" height="260%">
          <feGaussianBlur stdDeviation="80" />
        </filter>
        <filter id="gv-f2" x="-80%" y="-80%" width="260%" height="260%">
          <feGaussianBlur stdDeviation="36" />
        </filter>
        <filter id="gv-f3" x="-30%" y="-30%" width="160%" height="160%">
          <feGaussianBlur stdDeviation="10" />
        </filter>
        <filter id="gv-f4" x="-10%" y="-10%" width="120%" height="120%">
          <feGaussianBlur stdDeviation="1.8" />
        </filter>
        <filter id="gv-fx" x="-10%" y="0" width="120%" height="100%">
          <feGaussianBlur stdDeviation="10 0" />
        </filter>
        <filter id="gv-gr" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="5" stitchTiles="stitch" />
          <feColorMatrix type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 1.8 -0.62" />
        </filter>
        <linearGradient id="gv-bar" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#0B2A7A" stopOpacity="0" />
          <stop offset="0.45" stopColor="#2F7FD8" stopOpacity="0.55" />
          <stop offset="1" stopColor="#CDEBFF" />
        </linearGradient>
        <radialGradient id="gv-arc" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stopColor="#0A1A8C" />
          <stop offset="0.82" stopColor="#1238E8" />
          <stop offset="1" stopColor="#3D7BFF" />
        </radialGradient>
      </defs>
      <rect x="-2000" y="-2000" width="6000" height="6000" fill="#02040E" />
      <Field light={light} />
      <rect width="1280" height="800" fill="url(#gv-vg)" />
      <rect width="1280" height="800" filter="url(#gv-gr)" opacity="0.2" />
    </svg>
  );
}

/** Which field each screen uses in the design. */
export function lightFor(path: string, tunnel: "on" | "off" | "busy"): Light {
  if (path === "/") return tunnel === "on" ? "beam" : tunnel === "busy" ? "bars" : "arc";
  if (path.startsWith("/servers") || path.startsWith("/connections") || path.startsWith("/split")) return "horizon";
  if (path.startsWith("/services") || path.startsWith("/report")) return "blob";
  if (path.startsWith("/shop") || path.startsWith("/usage")) return "arc";
  if (path.startsWith("/account") || path.startsWith("/tools")) return "bars";
  if (path.startsWith("/referral")) return "stack";
  return "beam";
}
