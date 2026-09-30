import { ICON_PATHS, type IconName } from "./icon-paths";

export type { IconName };

export function Icon({
  name,
  size = 20,
  stroke = 2,
  color = "currentColor",
}: {
  name: IconName;
  size?: number;
  stroke?: number;
  color?: string;
}) {
  // The paths are our own constant markup (icon-paths.ts), never user data.
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={stroke}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: ICON_PATHS[name] }}
    />
  );
}
