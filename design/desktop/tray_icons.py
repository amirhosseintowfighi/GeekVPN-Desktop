"""Tray icons (Desktop-TrayIcons board): the logo on a rounded square, one
colour per state. Writes apps/desktop/src-tauri/icons/tray-*.png through
ImageMagick; run from the repository root."""
import re, subprocess, pathlib

logo = pathlib.Path("apps/desktop/src/design-system/Logo.tsx").read_text()
paths = re.findall(r"'(M[^']+)'", logo)
STATES = {"on": ("#00ACFE", "#062845"), "off": ("#8A9BB0", "#FFFFFF"), "busy": ("#F59E0B", "#062845")}
out = pathlib.Path("apps/desktop/src-tauri/icons")
for name, (bg, fg) in STATES.items():
    body = "".join(f'<path d="{d}"/>' for d in paths)
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">'
           f'<rect width="64" height="64" rx="18" fill="{bg}"/>'
           f'<svg x="9" y="10" width="46" height="45" viewBox="0 0 340 330">'
           f'<g transform="translate(0,330) scale(0.1,-0.1)" fill="{fg}">{body}</g></svg></svg>')
    subprocess.run(["convert", "-background", "none", "-density", "288", "svg:-", "-resize", "64x64", str(out / f"tray-{name}.png")],
                   input=svg.encode(), check=True)
    print("wrote", out / f"tray-{name}.png")
