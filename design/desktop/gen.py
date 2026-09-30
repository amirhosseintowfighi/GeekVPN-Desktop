# Generates the desktop artboards of the GeekVPN canvas in the Sky Lens language.
# Every style value comes from Foundations / Desktop-Home, or the Android dark palette.
import json, os, pathlib

OUT = pathlib.Path(__file__).parent
OUT.mkdir(exist_ok=True)
LOGO = "/_blob/bd283d17aeac830200bcf609b68fa496"
GLASSES = "/_blob/9da278ee9b094f4c0052411b28d21a59"
FONT = "font-family:'Space Grotesk',Vazirmatn,sans-serif"

LIGHT = dict(
    bg="#0870D4", glow="rgba(0,172,254,0.55)", glow2="rgba(0,172,254,0.45)",
    milk="background:rgba(255,255,255,0.88);border:1px solid rgba(255,255,255,0.95);box-shadow:inset 0 1px 0 #FFFFFF,0 14px 34px rgba(2,36,84,0.28);backdrop-filter:blur(24px) saturate(180%);-webkit-backdrop-filter:blur(24px) saturate(180%);box-sizing:border-box;color:#062845",
    clear="background:rgba(255,255,255,0.13);border:1px solid rgba(255,255,255,0.34);box-shadow:inset 0 1px 0 rgba(255,255,255,0.45),0 10px 26px rgba(2,36,84,0.18);backdrop-filter:blur(18px) saturate(160%);-webkit-backdrop-filter:blur(18px) saturate(160%);box-sizing:border-box;color:#FFFFFF",
    text="#062845", text2="#3F5F7E", mute="#5E7A94", navy="#062845", onNavy="#FFFFFF",
    accent="#00ACFE", link="#0078C8", soft="#EAF5FD", softBtn="#EEF5FB", track="#D5E3EF",
    hair="#E4EEF6", chip="#FFFFFF", chipBorder="#D5E3EF", check="#C9D9E7",
    ok="#0E9F6E", okSoft="#E3F8EF", warn="#C77700", warnSoft="#FFF4E0",
    bad="#D93F48", badSoft="#FFF1F2", badBorder="#F7C9CD", okBright="#7DFFCB",
    knobOn="#00ACFE", scrim="rgba(3,27,51,0.6)", payBar="#062845",
    shadowNavy="0 10px 24px rgba(2,24,56,0.35)", ticket="#F4F9FD",
)
DARK = dict(LIGHT,
    bg="#041D38", glow="rgba(8,112,212,0.36)", glow2="rgba(0,172,254,0.30)",
    milk="background:rgba(11,42,74,0.88);border:1px solid rgba(255,255,255,0.15);box-shadow:inset 0 1px 0 rgba(255,255,255,0.2),0 14px 34px rgba(0,8,20,0.4);backdrop-filter:blur(24px) saturate(180%);-webkit-backdrop-filter:blur(24px) saturate(180%);box-sizing:border-box;color:#EAF5FD",
    clear="background:rgba(255,255,255,0.09);border:1px solid rgba(255,255,255,0.2);box-shadow:inset 0 1px 0 rgba(255,255,255,0.25),0 10px 26px rgba(0,8,20,0.3);backdrop-filter:blur(18px) saturate(160%);-webkit-backdrop-filter:blur(18px) saturate(160%);box-sizing:border-box;color:#FFFFFF",
    text="#EAF5FD", text2="#9DB7CF", mute="#9DB7CF", navy="#00ACFE", onNavy="#031B33",
    link="#00ACFE", soft="#11375C", softBtn="#123B61", track="#24496D", hair="#1C4166",
    chip="#0F3357", chipBorder="#24496D", check="#3B6187",
    ok="#3CC896", okSoft="#0E3B33", warn="#F0A63A", warnSoft="#3D2E12",
    bad="#F26B73", badSoft="#40202A", badBorder="#6B2E3A", knobOn="#031B33",
    payBar="#0B2A4A", shadowNavy="0 10px 24px rgba(0,172,254,0.3)", ticket="#0D3155",
)

P = {  # 24px stroke icon paths
 "home": '<path d="M3 10.5 12 3l9 7.5"/><path d="M5 9.5V20h14V9.5"/><path d="M10 20v-6h4v6"/>',
 "shield": '<path d="M12 3 4 6v6c0 5 3.5 8 8 9 4.5-1 8-4 8-9V6z"/><path d="m9 12 2 2 4-4"/>',
 "bag": '<path d="M6 8h12l-1 12H7z"/><path d="M9 8V6a3 3 0 0 1 6 0v2"/>',
 "user": '<circle cx="12" cy="8" r="4"/><path d="M4 21c1.5-4 4.5-6 8-6s6.5 2 8 6"/>',
 "globe": '<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3c2.5 2.7 3.8 5.7 3.8 9s-1.3 6.3-3.8 9c-2.5-2.7-3.8-5.7-3.8-9S9.5 5.7 12 3z"/>',
 "hub": '<circle cx="12" cy="12" r="2.5"/><circle cx="5" cy="5" r="2"/><circle cx="19" cy="5" r="2"/><circle cx="5" cy="19" r="2"/><circle cx="19" cy="19" r="2"/><path d="M7 7l3 3M17 7l-3 3M7 17l3-3M17 17l-3-3"/>',
 "tool": '<path d="M14.5 6.5a4 4 0 0 0-5.3 5.3L4 17l3 3 5.2-5.2a4 4 0 0 0 5.3-5.3l-2.5 2.5-2.5-.5-.5-2.5z"/>',
 "chat": '<path d="M4 5h16v11H9l-5 4z"/><path d="M8 9.5h8M8 12.5h5"/>',
 "gear": '<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M4.9 4.9 7 7M17 17l2.1 2.1M2 12h3M19 12h3M4.9 19.1 7 17M17 7l2.1-2.1"/>',
 "pin": '<path d="M9 4h6l-1 6 3 3H7l3-3z"/><path d="M12 13v7"/>',
 "min": '<path d="M5 12h14"/>', "max": '<rect x="5" y="5" width="14" height="14" rx="2"/>',
 "x": '<path d="M6 6l12 12M18 6 6 18"/>',
 "plane": '<path d="M21 4 3 11l7 2 2 7z"/><path d="m10 13 5-4"/>',
 "plus": '<path d="M12 5v14M5 12h14"/>',
 "wallet": '<rect x="3" y="6" width="18" height="14" rx="3"/><path d="M16 13h2"/><path d="M6 6V5a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1"/>',
 "chev": '<path d="m15 6-6 6 6 6"/>',
 "down": '<path d="M12 4v14"/><path d="m6 12 6 6 6-6"/>', "up": '<path d="M12 20V6"/><path d="m6 12 6-6 6 6"/>',
 "search": '<circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/>',
 "gauge": '<path d="M4 18a8 8 0 1 1 16 0"/><path d="m12 14 4-5"/>',
 "refresh": '<path d="M20 11a8 8 0 0 0-14.5-4.5L4 8"/><path d="M4 3v5h5"/><path d="M4 13a8 8 0 0 0 14.5 4.5L20 16"/><path d="M20 21v-5h-5"/>',
 "star": '<path d="m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1-4.4-4.3 6.1-.9z"/>',
 "sort": '<path d="M7 4v16"/><path d="m3 16 4 4 4-4"/><path d="M13 6h8M13 11h6M13 16h4"/>',
 "bolt": '<path d="M13 2 4 14h7l-1 8 9-12h-7z"/>',
 "cloud": '<path d="M7 18h10a4 4 0 0 0 .5-8A6 6 0 0 0 6 9.5 4.3 4.3 0 0 0 7 18z"/>',
 "lock": '<rect x="5" y="11" width="14" height="9" rx="2.5"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
 "copy": '<rect x="9" y="9" width="11" height="11" rx="2.5"/><path d="M5 15V6a2 2 0 0 1 2-2h8"/>',
 "qr": '<rect x="4" y="4" width="6" height="6" rx="1"/><rect x="14" y="4" width="6" height="6" rx="1"/><rect x="4" y="14" width="6" height="6" rx="1"/><path d="M14 14h2v2h-2zM18 18h2v2h-2zM18 14h2M14 18v2"/>',
 "link": '<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>',
 "unlink": '<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7"/><path d="M4 4l16 16"/>',
 "gift": '<rect x="3" y="8" width="18" height="5" rx="1"/><path d="M5 13v8h14v-8M12 8v13"/><path d="M12 8C10 4 7 4 7 6s3 2 5 2c2 0 5 0 5-2s-3-2-5 2z"/>',
 "chart": '<path d="M4 20V4"/><path d="M4 20h16"/><path d="M8 16v-4M12 16V8M16 16v-6"/>',
 "power": '<path d="M12 3v8"/><path d="M6.3 6.3a8 8 0 1 0 11.4 0"/>',
 "wifi": '<path d="M2 9a15 15 0 0 1 20 0"/><path d="M5.5 12.5a10 10 0 0 1 13 0"/><path d="M9 16a5 5 0 0 1 6 0"/><circle cx="12" cy="19" r="1"/>',
 "wall": '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18M3 15h18M9 4v6M15 10v5M9 15v5"/>',
 "split": '<path d="M4 6h5l6 12h5"/><path d="M4 18h5l2-4"/><path d="M15 6h5"/><path d="m18 3 3 3-3 3M18 15l3 3-3 3"/>',
 "moon": '<path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/>',
 "sun": '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
 "monitor": '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>',
 "apps": '<rect x="4" y="4" width="6" height="6" rx="1.5"/><rect x="14" y="4" width="6" height="6" rx="1.5"/><rect x="4" y="14" width="6" height="6" rx="1.5"/><rect x="14" y="14" width="6" height="6" rx="1.5"/>',
 "folder": '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
 "trash": '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>',
 "send": '<path d="M21 3 10 14"/><path d="m21 3-7 18-4-7-7-4z"/>',
 "alert": '<path d="M12 3 2 20h20z"/><path d="M12 10v4M12 17h.01"/>',
 "info": '<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/>',
 "clock": '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
 "check": '<path d="m5 12 5 5 9-10"/>',
 "dl": '<path d="M12 4v11"/><path d="m7 10 5 5 5-5"/><path d="M5 20h14"/>',
 "file": '<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v4h4M9 12h6M9 16h6"/>',
 "logout": '<path d="M15 4h4v16h-4"/><path d="M10 8l-4 4 4 4"/><path d="M6 12h10"/>',
 "key": '<circle cx="8" cy="15" r="4"/><path d="m11 12 9-9M17 6l3 3"/>',
 "radar": '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="5"/><path d="M12 12 19 5"/>',
 "flag": '<path d="M5 21V4h11l-2 4 2 4H5"/>',
 "eye": '<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
 "tray": '<path d="M3 13h5l2 3h4l2-3h5"/><path d="M5 5h14l2 8v6H3v-6z"/>',
 "keyboard": '<rect x="2" y="6" width="20" height="12" rx="2"/><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10"/>',
}

def ic(name, size=20, sw=2, color="currentColor"):
    return (f'<svg width="{size}" height="{size}" viewBox="0 0 24 24" fill="none" stroke="{color}" '
            f'stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{P[name]}</svg>')

def fa(n):
    return str(n).translate(str.maketrans("0123456789.", "۰۱۲۳۴۵۶۷۸۹٫"))

def backdrop(t, w, h, cx=None, cy=160):
    cx = cx if cx is not None else w * 0.85
    rings = "".join(
        f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="rgba(255,255,255,{op:.3f})" stroke-width="1"/>'
        for r, op in zip(range(120, 521, 80), (0.1, 0.086, 0.072, 0.058, 0.044, 0.03)))
    return (f'<svg aria-hidden="true" width="{w}" height="{h}" viewBox="0 0 {w} {h}" style="position:absolute;inset:0;pointer-events:none">'
            f'<circle cx="{w*0.1}" cy="{h*0.92}" r="260" fill="{t["glow"]}"/><circle cx="{cx}" cy="{cy}" r="190" fill="{t["glow2"]}"/>{rings}</svg>')

def titlebar():
    b = "".join(
        f'<button type="button" aria-label="{lbl}" style="width:44px;height:34px;border:none;background:none;color:#FFFFFF;display:flex;align-items:center;justify-content:center;cursor:pointer;opacity:0.85">{ic(n, 16, 2, "#FFFFFF")}</button>'
        for lbl, n in (("سنجاق", "pin"), ("کوچک‌سازی", "min"), ("بزرگ‌سازی", "max"), ("بستن", "x")))
    return (f'<div style="position:absolute;top:0;right:0;left:0;height:38px;display:flex;align-items:center;padding:0 16px 0 6px;box-sizing:border-box">'
            f'<span style="flex-grow:1"></span><div style="display:flex;direction:ltr">{b}</div></div>')

NAV = [("خانه", "home", "Desktop-Home.dc.html"), ("سرورها", "globe", "Desktop-Servers.dc.html"),
       ("سرویس‌ها", "shield", "Desktop-Services.dc.html"), ("فروشگاه", "bag", "Desktop-Shop.dc.html"),
       ("اتصالات", "hub", "Desktop-Connections.dc.html"), ("ابزارها", "tool", "Desktop-Tools.dc.html"),
       ("پشتیبانی", "chat", "Desktop-Support.dc.html"), ("حساب", "user", "Desktop-Account.dc.html"),
       ("تنظیمات", "gear", "Desktop-Settings.dc.html")]

def sidebar(t, current, badge=None):
    items = []
    for label, icon, href in NAV:
        if label == current:
            tile = (f'<span style="width:44px;height:44px;border-radius:14px;background:rgba(255,255,255,0.88);border:1px solid rgba(255,255,255,0.95);box-shadow:inset 0 1px 0 #FFFFFF,0 14px 34px rgba(2,36,84,0.28);box-sizing:border-box;color:#062845;display:flex;align-items:center;justify-content:center">{ic(icon, 21)}</span>'
                    f'<span style="font-size:10px;font-weight:800;color:#FFFFFF">{label}</span>')
            items.append(f'<a href="{href}" aria-current="page" style="display:flex;flex-direction:column;align-items:center;gap:4px;text-decoration:none;width:76px">{tile}</a>')
        else:
            dot = ""
            if badge and badge[0] == label:
                dot = f'<span style="position:absolute;top:-4px;left:-6px;min-width:18px;height:18px;padding:0 5px;box-sizing:border-box;border-radius:9px;background:{t["bad"]};color:#FFFFFF;font-size:10px;font-weight:800;display:flex;align-items:center;justify-content:center">{badge[1]}</span>'
            items.append(f'<a href="{href}" style="display:flex;flex-direction:column;align-items:center;gap:4px;text-decoration:none;width:76px">'
                         f'<span style="position:relative;width:40px;height:40px;border-radius:13px;color:#FFFFFF;background:rgba(255,255,255,0.08);display:flex;align-items:center;justify-content:center">{ic(icon, 21)}{dot}</span>'
                         f'<span style="font-size:10px;font-weight:500;color:#FFFFFF;opacity:0.8">{label}</span></a>')
    return (f'<aside aria-label="ناوبری اصلی" style="position:absolute;top:14px;right:14px;bottom:14px;width:92px;border-radius:26px;{t["clear"]};display:flex;flex-direction:column;align-items:center;gap:6px;padding:12px 0">'
            f'<img src="{LOGO}" alt="GeekVPN" style="width:40px;height:40px;object-fit:contain;margin-bottom:6px">{"".join(items)}</aside>')

def page(title, body, w=1280, h=800, t=LIGHT, lang="fa"):
    return f'''<!doctype html>
<html lang="{lang}" dir="rtl">
<head>
<meta charset="utf-8">
<title>{title}</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Vazirmatn:wght@300;400;500;600;700;800&amp;family=Space+Grotesk:wght@500;600;700&amp;display=swap">
<style>
body{{margin:0;background:{t["bg"]};font-family:Vazirmatn,Tahoma,sans-serif;color:#FFFFFF}}
a{{color:{t["link"]}}}a:hover{{color:{t["text"]}}}
</style>
</helmet>
<div dir="rtl" style="width:{w}px;height:{h}px;position:relative;overflow:hidden;background:{t["bg"]};font-family:Vazirmatn,Tahoma,sans-serif;color:#FFFFFF;box-sizing:border-box">
{body}
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":{w},"height":{h}}}}}'>
class Component extends DCLogic {{
renderVals() {{
return {{}};
}}
}}
</script>
</body>
</html>
'''

def shell(t, current, main, badge=("پشتیبانی", "۲")):
    return backdrop(t, 1280, 800) + titlebar() + sidebar(t, current, badge) + \
        f'<main style="position:absolute;top:38px;right:126px;left:20px;bottom:20px;display:flex;gap:20px">{main}</main>'

# --- small components -------------------------------------------------------
def header(title, sub="", actions=""):
    s = f'<span style="font-size:13px;opacity:0.9">{sub}</span>' if sub else ""
    return (f'<header style="display:flex;align-items:center;gap:12px"><div style="flex-grow:1;display:flex;flex-direction:column;gap:2px">'
            f'<h1 style="margin:0;font-size:26px;font-weight:800;letter-spacing:-0.3px">{title}</h1>{s}</div>{actions}</header>')

def card(t, inner, extra="", pad=16, r=24, tag="section"):
    return f'<{tag} style="border-radius:{r}px;{t["milk"]};padding:{pad}px;display:flex;flex-direction:column;gap:12px;{extra}">{inner}</{tag}>'

def ctitle(t, title, sub="", right=""):
    s = f'<span style="font-size:12px;color:{t["text2"]}">{sub}</span>' if sub else ""
    return (f'<div style="display:flex;align-items:center;gap:8px"><span style="flex-grow:1;display:flex;flex-direction:column">'
            f'<span style="font-size:16px;font-weight:800">{title}</span>{s}</span>{right}</div>')

def iconbtn(t, name, label, kind="soft", size=38):
    st = {"soft": f"background:{t['soft']};color:{t['link']};border:none",
          "clear": t["clear"], "milk": t["milk"],
          "navy": f"background:{t['navy']};color:{t['onNavy']};border:none"}[kind]
    return (f'<button type="button" aria-label="{label}" style="width:{size}px;height:{size}px;border-radius:{13 if size < 44 else 15}px;display:flex;align-items:center;justify-content:center;flex-shrink:0;cursor:pointer;padding:0;{st}">{ic(name, 19)}</button>')

def btn(t, label, icon=None, kind="navy", h=44, grow=False, extra=""):
    st = {"navy": f"background:{t['navy']};color:{t['onNavy']};border:none;box-shadow:{t['shadowNavy']}",
          "soft": f"background:{t['softBtn']};color:{t['text']};border:none",
          "accent": f"background:{t['accent']};color:#062845;border:none",
          "danger": f"background:{t['badSoft']};color:{t['bad']};border:1px solid {t['badBorder']}",
          "white": "background:#FFFFFF;color:#062845;border:none",
          "clear": t["clear"]}[kind]
    g = "flex-grow:1;flex-basis:0;" if grow else ""
    i = ic(icon, 18) if icon else ""
    return (f'<button type="button" style="{g}height:{h}px;padding:0 16px;border-radius:14px;display:flex;align-items:center;justify-content:center;gap:8px;font-family:inherit;font-size:14px;font-weight:700;cursor:pointer;box-sizing:border-box;{st};{extra}">{i}<span>{label}</span></button>')

def switch(t, on, label):
    if on:
        return (f'<button type="button" role="switch" aria-checked="true" aria-label="{label}" style="width:50px;height:30px;border-radius:9px;border:none;padding:3px;background:{t["navy"]};display:flex;justify-content:flex-end;align-items:center;flex-shrink:0;cursor:pointer"><span style="width:24px;height:24px;border-radius:7px;background:{t["knobOn"]}"></span></button>')
    return (f'<button type="button" role="switch" aria-checked="false" aria-label="{label}" style="width:50px;height:30px;border-radius:9px;border:none;padding:3px;background:{t["track"]};display:flex;justify-content:flex-start;align-items:center;flex-shrink:0;cursor:pointer"><span style="width:24px;height:24px;border-radius:7px;background:#FFFFFF;box-shadow:0 1px 3px rgba(0,0,0,0.2)"></span></button>')

def row(t, icon, title, sub="", right="chev", last=False, href=None, tone="link"):
    tones = {"link": (t["soft"], t["link"]), "ok": (t["okSoft"], t["ok"]), "warn": (t["warnSoft"], t["warn"]), "bad": (t["badSoft"], t["bad"])}
    bg, fg = tones[tone]
    r = f'<span style="color:{t["mute"]};display:flex">{ic("chev", 18)}</span>' if right == "chev" else right
    s = f'<span style="font-size:12px;color:{t["text2"]}">{sub}</span>' if sub else ""
    border = "" if last else f"border-bottom:1px solid {t['hair']};"
    tag, ha = ("a", f' href="{href}"') if href else ("div", "")
    return (f'<{tag}{ha} style="display:flex;align-items:center;gap:12px;padding:11px 14px;{border}text-decoration:none">'
            f'<span style="width:38px;height:38px;border-radius:12px;background:{bg};color:{fg};display:flex;align-items:center;justify-content:center;flex-shrink:0">{ic(icon, 19)}</span>'
            f'<span style="flex-grow:1;display:flex;flex-direction:column;gap:1px;min-width:0"><span style="font-size:14px;font-weight:700;color:{t["text"]}">{title}</span>{s}</span>{r}</{tag}>')

def group(t, rows, label=None):
    lab = f'<span style="font-size:13px;font-weight:700;color:rgba(255,255,255,0.9)">{label}</span>' if label else ""
    return lab + f'<div style="border-radius:22px;{t["milk"]};overflow:hidden">{"".join(rows)}</div>'

def badge(t, text, tone="ok"):
    c = {"ok": (t["okSoft"], t["ok"]), "warn": (t["warnSoft"], t["warn"]), "bad": (t["badSoft"], t["bad"]), "link": (t["soft"], t["link"])}[tone]
    return (f'<span style="height:26px;padding:0 10px;border-radius:9px;background:{c[0]};color:{c[1]};display:flex;align-items:center;gap:6px;font-size:12px;font-weight:800;flex-shrink:0;white-space:nowrap">'
            f'<span style="width:7px;height:7px;border-radius:2px;background:{c[1]}"></span>{text}</span>')

def chips(t, labels, sel=0, h=32):
    out = []
    for i, l in enumerate(labels):
        if i == sel:
            out.append(f'<button type="button" aria-pressed="true" style="height:{h}px;padding:0 12px;border-radius:10px;border:none;background:{t["navy"]};color:{t["onNavy"]};font-family:inherit;font-size:12px;font-weight:700;cursor:pointer">{l}</button>')
        else:
            out.append(f'<button type="button" aria-pressed="false" style="height:{h}px;padding:0 12px;border-radius:10px;border:1px solid {t["chipBorder"]};background:{t["chip"]};color:{t["text"]};font-family:inherit;font-size:12px;font-weight:700;cursor:pointer">{l}</button>')
    return f'<div style="display:flex;gap:6px;flex-wrap:wrap">{"".join(out)}</div>'

def clearchips(t, labels, sel=0):
    out = []
    for i, l in enumerate(labels):
        if i == sel:
            out.append(f'<button type="button" aria-pressed="true" style="height:36px;padding:0 14px;border-radius:12px;font-family:inherit;font-size:13px;font-weight:700;cursor:pointer;background:#FFFFFF;color:#062845;border:none">{l}</button>')
        else:
            out.append(f'<button type="button" aria-pressed="false" style="height:36px;padding:0 14px;border-radius:12px;font-family:inherit;font-size:13px;font-weight:700;cursor:pointer;{t["clear"]}">{l}</button>')
    return f'<div style="display:flex;gap:6px">{"".join(out)}</div>'

def segmented(t, labels, sel=0, h=40):
    out = []
    for i, l in enumerate(labels):
        on = i == sel
        st = f"background:{t['navy']};color:{t['onNavy']}" if on else f"background:none;color:{t['text']}"
        out.append(f'<button type="button" aria-pressed="{"true" if on else "false"}" style="flex-grow:1;flex-basis:0;height:{h-8}px;border-radius:11px;border:none;font-family:inherit;font-size:13px;font-weight:700;cursor:pointer;{st}">{l}</button>')
    return f'<div style="display:flex;gap:4px;padding:4px;border-radius:14px;background:{t["softBtn"]}">{"".join(out)}</div>'

def bars(t, level):
    col = {4: t["ok"], 3: t["ok"], 2: t["warn"], 1: t["bad"], 0: t["track"]}[level]
    return ('<span aria-hidden="true" style="display:flex;align-items:flex-end;gap:2px;height:17px;direction:ltr">' +
            "".join(f'<span style="width:4px;height:{hh}px;border-radius:2px;background:{col if i < level else t["track"]}"></span>' for i, hh in enumerate((5, 9, 13, 17))) + '</span>')

def ping(t, ms):
    lvl = 4 if ms < 150 else 3 if ms < 250 else 2 if ms < 450 else 1
    return (f'<span style="display:flex;align-items:center;gap:8px"><span dir="ltr" style="{FONT};font-size:12px;font-weight:600;color:{t["text2"]}">{ms}ms</span>{bars(t, lvl)}</span>')

def cc(t, code, dark=False, size=40):
    st = f"background:{t['navy']};color:{t['onNavy']}" if dark else f"background:{t['soft']};color:{t['text']}"
    return (f'<span style="width:{size}px;height:{size}px;border-radius:12px;{st};display:flex;align-items:center;justify-content:center;{FONT};font-size:12px;font-weight:700;letter-spacing:0.5px;flex-shrink:0">{code}</span>')

def search(t, pid, ph, width=None):
    w = f"width:{width}px;" if width else "flex-grow:1;"
    return (f'<label for="{pid}" style="position:absolute;width:1px;height:1px;overflow:hidden">{ph}</label>'
            f'<div style="{w}height:40px;border-radius:12px;background:{t["softBtn"]};display:flex;align-items:center;gap:8px;padding:0 12px;color:{t["mute"]};box-sizing:border-box">{ic("search", 17)}'
            f'<input id="{pid}" placeholder="{ph}" style="flex-grow:1;min-width:0;border:none;background:none;outline:none;font-family:inherit;font-size:13px;color:{t["text"]}"></div>')

def clearsearch(t, pid, ph, width=280):
    return (f'<label for="{pid}" style="position:absolute;width:1px;height:1px;overflow:hidden">{ph}</label>'
            f'<div style="width:{width}px;height:42px;border-radius:13px;{t["clear"]};display:flex;align-items:center;gap:8px;padding:0 12px">{ic("search", 17, 2, "#FFFFFF")}'
            f'<input id="{pid}" placeholder="{ph}" style="flex-grow:1;min-width:0;border:none;background:none;outline:none;font-family:inherit;font-size:13px;color:#FFFFFF"></div>')

def field(t, pid, label, value="", ph="", multiline=False, h=44, mono=False):
    f = f"{FONT};" if mono else "font-family:inherit;"
    inp = (f'<textarea id="{pid}" placeholder="{ph}" style="height:{h}px;resize:none;border-radius:13px;border:1px solid {t["chipBorder"]};background:{t["chip"]};padding:10px 12px;{f}font-size:13px;line-height:1.8;color:{t["text"]};box-sizing:border-box">{value}</textarea>'
           if multiline else
           f'<input id="{pid}" value="{value}" placeholder="{ph}" style="height:{h}px;border-radius:13px;border:1px solid {t["chipBorder"]};background:{t["chip"]};padding:0 12px;{f}font-size:13px;color:{t["text"]};box-sizing:border-box">')
    return (f'<div style="display:flex;flex-direction:column;gap:6px"><label for="{pid}" style="font-size:12px;font-weight:700;color:{t["text2"]}">{label}</label>{inp}</div>')

def stat(t, label, value, unit="", big=24):
    u = f' <span style="font-size:12px;color:{t["text2"]};font-weight:600">{unit}</span>' if unit else ""
    return (f'<div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;gap:4px;min-width:0"><span style="font-size:12px;color:{t["text2"]}">{label}</span>'
            f'<span style="font-size:{big}px;font-weight:800;line-height:1.15">{value}{u}</span></div>')

def qr(size=168, fg="#062845", seed=7):
    n = 25
    cell = size / n
    import random
    rnd = random.Random(seed)
    rects = []
    def finder(x, y):
        return (f'<rect x="{x*cell}" y="{y*cell}" width="{7*cell}" height="{7*cell}" rx="{cell*1.4}" fill="none" stroke="{fg}" stroke-width="{cell}"/>'
                f'<rect x="{(x+2)*cell}" y="{(y+2)*cell}" width="{3*cell}" height="{3*cell}" rx="{cell*0.6}" fill="{fg}"/>')
    for yy in range(n):
        for xx in range(n):
            if (xx < 8 and yy < 8) or (xx > n-9 and yy < 8) or (xx < 8 and yy > n-9):
                continue
            if rnd.random() < 0.46:
                rects.append(f'<rect x="{xx*cell:.2f}" y="{yy*cell:.2f}" width="{cell:.2f}" height="{cell:.2f}" rx="{cell*0.3:.2f}" fill="{fg}"/>')
    return (f'<svg role="img" aria-label="کد QR" width="{size}" height="{size}" viewBox="0 0 {size} {size}">'
            + finder(0.5, 0.5) + finder(n-7.5, 0.5) + finder(0.5, n-7.5) + "".join(rects) + '</svg>')

def dialog_scrim(t, w=1280, h=800):
    return f'<div aria-hidden="true" style="position:absolute;inset:0;background:{t["scrim"]}"></div>'

def write(name, html):
    (OUT / name).write_text(html, encoding="utf-8")

# ============================================================================
# Login
def login(t=LIGHT):
    hero = (f'<div style="position:absolute;top:80px;right:80px;width:520px;display:flex;flex-direction:column;gap:26px">'
            f'<div style="position:relative;width:260px;height:260px;display:flex;align-items:center;justify-content:center">'
            f'<svg aria-hidden="true" width="260" height="260" viewBox="0 0 260 260" style="position:absolute;inset:0"><rect x="30" y="30" width="200" height="200" rx="60" fill="rgba(255,255,255,0.10)" stroke="rgba(255,255,255,0.35)"/><rect x="10" y="10" width="240" height="240" rx="72" fill="none" stroke="rgba(255,255,255,0.18)" stroke-dasharray="3 8"/></svg>'
            f'<img src="{LOGO}" alt="GeekVPN" style="position:relative;width:150px;height:150px;object-fit:contain"></div>'
            f'<h1 dir="ltr" style="margin:0;{FONT};font-size:56px;font-weight:700;letter-spacing:-1.5px;text-align:right;line-height:1">GeekVPN</h1>'
            f'<p style="margin:0;font-size:18px;line-height:1.8;color:rgba(255,255,255,0.92)">اینترنت آزاد، سریع و امن — روی ویندوز، مک و لینوکس.</p>'
            f'<div style="display:flex;gap:8px;flex-wrap:wrap">' +
            "".join(f'<span style="height:36px;padding:0 14px;border-radius:12px;{t["clear"]};display:flex;align-items:center;gap:6px;font-size:13px;font-weight:600">{ic(i, 16, 2, "#FFFFFF")}{l}</span>'
                    for i, l in (("bolt", "اتصال هوشمند"), ("wall", "Kill Switch واقعی"), ("split", "سایت‌های ایرانی مستقیم"), ("plane", "پشتیبانی تلگرام"))) +
            '</div></div>')
    panel = (f'<section style="position:absolute;top:110px;left:90px;width:420px;border-radius:30px;{t["milk"]};padding:28px;display:flex;flex-direction:column;gap:14px">'
             f'<span style="font-size:22px;font-weight:800">ورود به حساب</span>'
             f'<span style="font-size:13px;line-height:1.9;color:{t["text2"]}">با تأیید در ربات تلگرام GeekVPN وارد شو؛ سرویس‌هایت خودکار روی این کامپیوتر می‌آیند.</span>'
             f'<a href="Desktop-Login-Wait.dc.html" style="height:58px;border-radius:18px;display:flex;align-items:center;gap:10px;padding:0 16px 0 7px;box-sizing:border-box;font-size:15px;font-weight:700;text-decoration:none;background:{t["navy"]};color:{t["onNavy"]};box-shadow:{t["shadowNavy"]}"><span style="flex-grow:1;padding-right:6px">ورود با تلگرام</span><span style="width:44px;height:44px;border-radius:15px;background:#FFFFFF;color:#062845;display:flex;align-items:center;justify-content:center">{ic("plane", 20, 2.2)}</span></a>'
             f'<div style="display:flex;gap:10px">{btn(t, "نام کاربری", "user", "soft", 52, True)}{btn(t, "ساخت حساب", "plus", "soft", 52, True)}</div>'
             f'<div style="display:flex;align-items:center;gap:10px;color:{t["mute"]};font-size:12px"><span style="flex-grow:1;height:1px;background:{t["hair"]}"></span>یا<span style="flex-grow:1;height:1px;background:{t["hair"]}"></span></div>'
             f'<a href="Desktop-Home.dc.html" style="align-self:center;padding:6px;font-size:13px;font-weight:700;text-decoration:none">شروع سریع بدون ثبت‌نام</a>'
             f'<span style="font-size:11px;line-height:1.8;color:{t["text2"]};text-align:center">توکن ورود در {"Credential Manager / Keychain"} سیستم‌عامل نگه داشته می‌شود.</span>'
             '</section>')
    # username sheet preview as a second small card
    user = (f'<section style="position:absolute;bottom:40px;left:90px;width:420px;border-radius:24px;{t["milk"]};padding:20px;display:flex;flex-direction:column;gap:12px">'
            f'<span style="font-size:15px;font-weight:800">ورود با نام کاربری</span>'
            f'<div style="display:flex;gap:10px"><div style="flex-grow:1;flex-basis:0">{field(t, "lu", "نام کاربری", "", "از ربات: پروفایل ← ورود به اپ")}</div><div style="flex-grow:1;flex-basis:0">{field(t, "lp", "رمز", "", "••••••••")}</div></div>'
            f'{btn(t, "ورود", "key", "navy", 46)}</section>')
    body = backdrop(t, 1280, 800, cx=380, cy=260) + titlebar() + hero + panel + user
    return page("ورود — GeekVPN", body, t=t)

def login_wait(t=LIGHT):
    steps = "".join(
        f'<div style="display:flex;align-items:center;gap:10px"><span style="width:26px;height:26px;border-radius:9px;background:{t["soft"]};color:{t["link"]};display:flex;align-items:center;justify-content:center;{FONT};font-size:13px;font-weight:700;flex-shrink:0">{fa(i)}</span><span style="font-size:13px;line-height:1.7">{s}</span></div>'
        for i, s in enumerate(("تلگرام باز شد؛ اگر نشد، کد QR را با گوشی اسکن کن.", "در ربات GeekVPN دکمه‌ی «تأیید و اتصال» را بزن.", "این پنجره خودش وارد می‌شود و سرویس‌ها را می‌آورد."), 1))
    panel = (f'<section style="position:absolute;top:120px;right:50%;margin-right:-400px;width:800px;border-radius:30px;{t["milk"]};padding:32px;display:flex;gap:32px;box-sizing:border-box">'
             f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:16px">'
             f'<span style="display:flex;align-items:center;gap:8px;font-size:13px;font-weight:700;color:{t["warn"]}"><span style="width:9px;height:9px;border-radius:2px;background:{t["warn"]}"></span>منتظر تأیید در تلگرام</span>'
             f'<span style="font-size:24px;font-weight:800">درخواست ورود فرستاده شد</span>{steps}'
             f'<div style="display:flex;align-items:center;gap:10px;margin-top:6px"><span style="font-size:12px;color:{t["text2"]}">اعتبار لینک</span><span dir="ltr" style="{FONT};font-size:22px;font-weight:700">04:21</span>'
             f'<span style="flex-grow:1;height:6px;border-radius:3px;background:{t["track"]};overflow:hidden"><span style="display:block;width:87%;height:100%;background:{t["link"]}"></span></span></div>'
             f'<div style="display:flex;gap:10px;margin-top:4px">{btn(t, "باز کردن دوباره‌ی تلگرام", "plane", "navy", 48, True)}{btn(t, "کپی لینک", "copy", "soft", 48)}{btn(t, "انصراف", None, "danger", 48)}</div>'
             '</div>'
             f'<div style="width:236px;flex-shrink:0;display:flex;flex-direction:column;align-items:center;gap:10px"><div style="padding:14px;border-radius:22px;background:#FFFFFF;border:1px solid {t["chipBorder"]}">{qr(196)}</div>'
             f'<span style="font-size:12px;color:{t["text2"]};text-align:center;line-height:1.8">با دوربین گوشی یا اسکنر تلگرام اسکن کن</span></div>'
             '</section>')
    body = backdrop(t, 1280, 800, cx=640, cy=300) + titlebar() + panel
    return page("منتظر تأیید — GeekVPN", body, t=t)

# ============================================================================
# Home: connecting (smart connect stage hub) and dark connected
def ring(size=280, prog=None):
    c = size / 2
    arc = ""
    if prog is not None:
        import math
        r = 128
        L = 2 * math.pi * r
        arc = (f'<circle cx="{c}" cy="{c}" r="{r}" fill="none" stroke="#00ACFE" stroke-width="6" stroke-linecap="round" '
               f'stroke-dasharray="{L*prog:.1f} {L:.1f}" transform="rotate(90 {c} {c})"/>')
    return (f'<svg aria-hidden="true" width="{size}" height="{size}" viewBox="0 0 {size} {size}" style="position:absolute;inset:0">'
            f'<circle cx="{c}" cy="{c}" r="128" fill="none" stroke="rgba(255,255,255,0.22)" stroke-width="6"/>{arc}</svg>')

def home_left(t, state):
    wallet = (f'<a href="Desktop-Account.dc.html" style="height:40px;padding:0 12px 0 6px;border-radius:13px;{t["clear"]};display:flex;align-items:center;gap:8px;text-decoration:none;font-size:13px;font-weight:700"><span style="width:28px;height:28px;border-radius:9px;background:#FFFFFF;color:#062845;display:flex;align-items:center;justify-content:center">{ic("wallet", 16, 2.2)}</span>۰ تومان</a>'
              f'<a href="Desktop-Shop.dc.html" style="height:40px;padding:0 6px 0 14px;border-radius:13px;background:{t["payBar"] if t is LIGHT else "#0B2A4A"};color:#FFFFFF;display:flex;align-items:center;gap:10px;font-size:13px;font-weight:700;text-decoration:none">خرید سرویس<span style="width:28px;height:28px;border-radius:9px;background:#FFFFFF;color:#062845;display:flex;align-items:center;justify-content:center">{ic("plus", 16, 2.6)}</span></a>')
    head = (f'<header style="display:flex;align-items:center;gap:12px"><div style="flex-grow:1;display:flex;flex-direction:column;gap:2px"><span dir="ltr" style="{FONT};font-size:22px;font-weight:700;letter-spacing:-0.5px;text-align:right">GeekVPN</span>'
            f'<span style="font-size:13px;opacity:0.9">شب بخیر، کاربر ۱۰۱۱۷۸۸۱۲۳</span></div>{wallet}</header>')
    btn_style = (f"position:relative;width:236px;height:236px;border-radius:50%;display:flex;align-items:center;justify-content:center;cursor:pointer;{t['milk']};"
                 "box-shadow:inset 0 2px 0 rgba(255,255,255,0.6),0 0 0 14px rgba(255,255,255,0.12),0 24px 50px rgba(2,36,84,0.35)")
    if state == "connecting":
        stages = [("پیدا کردن IP تمیز", "done", "۳ IP تازه برای این شبکه"), ("تست تأخیر سرورها", "done", "۱۰ سرور · بهترین ۹۶ms"),
                  ("تنظیم فایروال و Kill Switch", "done", ""), ("اتصال · تلاش ۲ از ۳", "now", "ترکیه · کلاد روی 104.18.x.x"), ("تست نهایی تأخیر", "todo", "")]
        items = []
        for label, st, sub in stages:
            if st == "done":
                mark = f'<span style="width:24px;height:24px;border-radius:8px;background:{t["navy"]};color:{t["accent"] if t is LIGHT else "#031B33"};display:flex;align-items:center;justify-content:center;flex-shrink:0">{ic("check", 15, 3)}</span>'
            elif st == "now":
                mark = f'<span style="width:24px;height:24px;border-radius:8px;background:#FFFFFF;display:flex;align-items:center;justify-content:center;flex-shrink:0"><span style="width:10px;height:10px;border-radius:3px;background:#C77700"></span></span>'
            else:
                mark = '<span style="width:24px;height:24px;border-radius:8px;border:2px solid rgba(255,255,255,0.45);box-sizing:border-box;flex-shrink:0"></span>'
            s = f'<span style="font-size:11px;opacity:0.8">{sub}</span>' if sub else ""
            items.append(f'<div style="display:flex;align-items:center;gap:10px;opacity:{1 if st != "todo" else 0.6}">{mark}<span style="display:flex;flex-direction:column"><span style="font-size:14px;font-weight:{800 if st == "now" else 600}">{label}</span>{s}</span></div>')
        right = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:12px">'
                 f'<span style="display:flex;align-items:center;gap:8px;font-size:15px;font-weight:700"><span style="width:9px;height:9px;border-radius:2px;background:#FFB547"></span>اتصال هوشمند</span>'
                 f'<div style="border-radius:20px;{t["clear"]};padding:14px 16px;display:flex;flex-direction:column;gap:12px;max-width:400px">{"".join(items)}</div>'
                 f'<span style="font-size:13px;opacity:0.9">برای لغو، دوباره روی عینک بزن</span></div>')
        center = (f'<div style="position:relative;width:280px;height:280px;flex-shrink:0;display:flex;align-items:center;justify-content:center">{ring(280, 0.62)}'
                  f'<button type="button" aria-label="لغو اتصال" style="{btn_style}"><img src="{GLASSES}" alt="" style="width:132px;height:132px;object-fit:contain;opacity:0.7"></button></div>')
        stats_vals = [("دانلود", "dl", "—", ""), ("آپلود", "up", "—", ""), ("IP خروجی", "globe", "—", ""), ("اتصالات فعال", "hub", "—", ""), ("زمان باقی‌مانده", "clock", "۲۸ روز", "")]
    else:
        right = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px">'
                 f'<span style="display:flex;align-items:center;gap:8px;font-size:15px;font-weight:700"><span style="width:9px;height:9px;border-radius:2px;background:#7DFFCB"></span>متصل و امن</span>'
                 f'<span dir="ltr" style="{FONT};font-size:56px;font-weight:700;letter-spacing:1px;line-height:1;text-align:right">00:12:48</span>'
                 f'<span style="font-size:13px;opacity:0.9">برای قطع اتصال، روی عینک بزن</span>'
                 f'<a href="Desktop-Servers.dc.html" style="margin-top:6px;border-radius:20px;{t["milk"]};display:flex;align-items:center;gap:12px;padding:10px 14px 10px 10px;text-decoration:none;max-width:380px">{cc(t, "DE", True, 44)}'
                 f'<span style="flex-grow:1;display:flex;flex-direction:column;gap:2px"><span style="font-size:12px;color:{t["text2"]}">سرور · مسیر هوشمند</span><span style="font-size:15px;font-weight:800;color:{t["text"]}">آلمان · کلاد</span></span>{ping(t, 121)}<span style="color:{t["mute"]};display:flex">{ic("chev", 18)}</span></a></div>')
        center = (f'<div style="position:relative;width:280px;height:280px;flex-shrink:0;display:flex;align-items:center;justify-content:center">{ring(280, 1.0)}'
                  f'<button type="button" aria-label="قطع اتصال" style="{btn_style}"><img src="{GLASSES}" alt="" style="width:132px;height:132px;object-fit:contain"></button></div>')
        stats_vals = [("دانلود", "dl", "2.48", "MB/s"), ("آپلود", "up", "312", "KB/s"), ("IP خروجی", "globe", "آلمان", ""), ("اتصالات فعال", "hub", "۷", ""), ("زمان باقی‌مانده", "clock", "۲۸ روز", "")]
    cells = []
    for i, (label, icon, v, u) in enumerate(stats_vals):
        if i:
            cells.append('<span aria-hidden="true" style="width:1px;align-self:stretch;background:rgba(255,255,255,0.25)"></span>')
        val = (f'<span dir="ltr" style="{FONT};font-size:18px;font-weight:700;text-align:right">{v} <span style="font-size:11px;opacity:0.8">{u}</span></span>' if u
               else f'<span style="font-size:17px;font-weight:800">{v}</span>')
        cells.append(f'<div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;gap:4px;padding:0 14px"><span style="display:flex;align-items:center;gap:6px;font-size:12px;opacity:0.85">{ic(icon, 14, 2.2, "#FFFFFF")}{label}</span>{val}</div>')
    strip = f'<div style="display:flex;padding:14px 4px;border-radius:20px;{t["clear"]}">{"".join(cells)}</div>'
    # live speed graph (desktop addition)
    import math
    pts_d = " ".join(f"{x*10},{56 - (22 + 16*math.sin(x/3.1) + 9*math.sin(x/1.3)) :.1f}" for x in range(0, 61))
    pts_u = " ".join(f"{x*10},{56 - (8 + 5*math.sin(x/2.2 + 1)) :.1f}" for x in range(0, 61))
    graph = ("" if state == "connecting" else
             f'<div style="display:flex;flex-direction:column;gap:6px;padding:0 4px"><div style="display:flex;justify-content:space-between;font-size:12px"><span style="font-weight:700">سرعت زنده · ۶۰ ثانیه‌ی اخیر</span>'
             f'<span style="display:flex;gap:12px;opacity:0.9"><span style="display:flex;align-items:center;gap:5px"><span style="width:10px;height:3px;border-radius:2px;background:#FFFFFF"></span>دانلود</span><span style="display:flex;align-items:center;gap:5px"><span style="width:10px;height:3px;border-radius:2px;background:#7DD8FF"></span>آپلود</span></span></div>'
             f'<svg role="img" aria-label="نمودار سرعت زنده" width="100%" height="56" viewBox="0 0 600 56" preserveAspectRatio="none" style="display:block"><polyline points="{pts_d}" fill="none" stroke="#FFFFFF" stroke-width="2"/><polyline points="{pts_u}" fill="none" stroke="#7DD8FF" stroke-width="2"/></svg></div>')
    usage = (f'<div style="display:flex;flex-direction:column;gap:8px;padding:0 4px"><div style="display:flex;justify-content:space-between;font-size:12px"><span style="font-weight:700">سرویس ۴۰ گیگ · یک‌ماهه — ۱۲٫۴ از ۴۰ گیگ</span><span style="opacity:0.9">۳۱٪</span></div>'
             f'<div style="display:flex;gap:3px;height:10px">' + "".join(f'<span style="flex-grow:1;border-radius:3px;background:{"#FFFFFF" if i < 12 else "rgba(255,255,255,0.22)"}"></span>' for i in range(40)) + '</div></div>')
    return (f'<div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;gap:16px;min-width:0">{head}'
            f'<div style="flex-grow:1;display:flex;align-items:center;gap:36px;padding:0 12px">{center}{right}</div>{strip}{graph}{usage}</div>')

def servers_panel(t, width=360):
    rows = [("DE", "آلمان", "کلاد", 121, True, True), ("TR", "ترکیه", "کلاد", 96, True, False), ("US", "آمریکا", "کلاد", 142, False, False),
            ("GB", "انگلیس", "کلاد", 168, False, False), ("FR", "فرانسه", "کلاد", 189, False, False), ("NL", "هلند", "کلاد", 233, False, False)]
    items = []
    for code, name, grp, ms, fav, sel in rows:
        star = ic("star", 17, 2, t["warn"]) if fav else ic("star", 17, 2, t["check"])
        st = f"background:{t['soft']};border:2px solid {t['link']}" if sel else "background:none;border:2px solid transparent"
        items.append(f'<button type="button" aria-pressed="{"true" if sel else "false"}" style="display:flex;align-items:center;gap:10px;padding:7px 8px;border-radius:14px;{st};font-family:inherit;color:{t["text"]};cursor:pointer;text-align:right">'
                     f'{cc(t, code, sel, 36)}<span style="flex-grow:1;display:flex;flex-direction:column"><span style="font-size:13px;font-weight:800">{name}</span><span style="font-size:11px;color:{t["text2"]}">{grp}</span></span>{ping(t, ms)}<span aria-label="{"ستاره‌دار" if fav else "ستاره"}" style="display:flex">{star}</span></button>')
    return (f'<section style="width:{width}px;flex-shrink:0;border-radius:26px;{t["milk"]};padding:16px;display:flex;flex-direction:column;gap:12px">'
            + ctitle(t, "سرورها", "۱۰ سرور · ستاره‌دارها بالا", iconbtn(t, "gauge", "تست پینگ همه") + iconbtn(t, "refresh", "بروزرسانی"))
            + search(t, "ds", "جستجوی کشور") + chips(t, ["همه", "ستاره‌دار", "کلاد", "ایرانسل"])
            + f'<div style="display:flex;gap:8px"><div style="flex-grow:1;flex-basis:0;border-radius:16px;background:{t["softBtn"]};padding:10px;display:flex;align-items:center;gap:8px"><span style="flex-grow:1;display:flex;flex-direction:column"><span style="font-size:12px;font-weight:800">انتخاب خودکار</span><span style="font-size:10px;color:{t["text2"]}">اتصال هوشمند + failover</span></span>{switch(t, True, "انتخاب خودکار")}</div>'
            + f'<a href="Desktop-Tools.dc.html" style="flex-grow:1;flex-basis:0;border-radius:16px;background:{t["navy"]};color:{t["onNavy"]};padding:10px;display:flex;align-items:center;gap:8px;text-decoration:none"><span style="color:{t["accent"] if t is LIGHT else "#031B33"};display:flex">{ic("radar", 18)}</span><span style="display:flex;flex-direction:column"><span style="font-size:12px;font-weight:800">بهینه‌ساز کلادفلر</span><span style="font-size:10px;opacity:0.75">اسکن IP تمیز</span></span></a></div>'
            + f'<div style="display:flex;flex-direction:column;gap:2px">{"".join(items)}</div></section>')

def home_connecting():
    t = LIGHT
    return page("خانه — در حال اتصال هوشمند", shell(t, "خانه", home_left(t, "connecting") + servers_panel(t)), t=t)

def home_dark():
    t = DARK
    return page("خانه — تم تیره", shell(t, "خانه", home_left(t, "on") + servers_panel(t)), t=t)

# ============================================================================
def servers_page():
    t = LIGHT
    groups = [("ستاره‌دار", [("DE", "آلمان", "کلاد · GeekVPN", 121, True, True, "vless · ws · tls", True), ("TR", "ترکیه", "کلاد · GeekVPN", 96, True, False, "vless · grpc · tls", True)]),
              ("سرویس ۴۰ گیگ · یک‌ماهه", [("US", "آمریکا", "کلاد", 142, False, False, "vless · ws · tls", True), ("GB", "انگلیس", "کلاد", 168, False, False, "vless · xhttp · tls", True),
                                          ("FR", "فرانسه", "کلاد", 189, False, False, "trojan · ws · tls", True), ("NL", "هلند", "کلاد", 233, False, False, "vless · reality", False)]),
              ("لینک دستی · My Sub", [("US", "آمریکا", "ایرانسل", 410, False, False, "vmess · tcp", False), ("GB", "انگلیس", "ایرانسل", 0, False, False, "vless · tcp", False)])]
    rows = []
    for gname, items in groups:
        rows.append(f'<div style="padding:10px 16px 4px;font-size:12px;font-weight:800;color:{t["text2"]}">{gname}</div>')
        for code, name, grp, ms, fav, sel, proto, cdn in items:
            st = f"background:{t['soft']}" if sel else ""
            p = ping(t, ms) if ms else badge(t, "بی‌پاسخ", "bad")
            clean = badge(t, "IP تمیز", "link") if cdn and ms and ms < 130 else ""
            rows.append(f'<div style="display:grid;grid-template-columns:44px minmax(0,1.6fr) minmax(0,1.3fr) 110px 120px 44px;align-items:center;gap:10px;padding:8px 16px;{st}">'
                        f'{cc(t, code, sel, 40)}<span style="display:flex;flex-direction:column"><span style="font-size:14px;font-weight:800">{name}</span><span style="font-size:11px;color:{t["text2"]}">{grp}</span></span>'
                        f'<span dir="ltr" style="{FONT};font-size:12px;color:{t["text2"]};text-align:right">{proto}</span><span style="display:flex">{clean}</span>{p}'
                        f'<button type="button" aria-label="{"حذف از ستاره‌دارها" if fav else "ستاره زدن"}" style="width:36px;height:36px;border:none;background:none;cursor:pointer;display:flex;align-items:center;justify-content:center">{ic("star", 19, 2, t["warn"] if fav else t["check"])}</button></div>')
    table = (f'<section style="flex-grow:1;border-radius:26px;{t["milk"]};display:flex;flex-direction:column;overflow:hidden">'
             f'<div style="display:grid;grid-template-columns:44px minmax(0,1.6fr) minmax(0,1.3fr) 110px 120px 44px;gap:10px;padding:12px 16px;border-bottom:1px solid {t["hair"]};font-size:12px;font-weight:700;color:{t["text2"]}"><span></span><span>سرور</span><span>پروتکل</span><span>آدرس</span><span>تأخیر واقعی</span><span></span></div>'
             f'{"".join(rows)}</section>')
    side = card(t,
                ctitle(t, "انتخاب خودکار", "اتصال هوشمند و failover") +
                f'<div style="display:flex;align-items:center;gap:10px;border-radius:14px;background:{t["softBtn"]};padding:10px 12px"><span style="flex-grow:1;font-size:13px;font-weight:700">سرور: خودکار</span>{switch(t, True, "سرور خودکار")}</div>'
                f'<span style="font-size:12px;font-weight:700;color:{t["text2"]}">جابه‌جایی وقتی تأخیر بیشتر شد از</span>'
                + segmented(t, ["هرگز", "قطعی", "۱ ث", "۲ ث", "۳ ث"], 3) +
                f'<span style="font-size:12px;line-height:1.8;color:{t["text2"]}">دو اندازه‌گیری بد پشت سر هم = رفتن به سرور بعدی، بدون قطع تونل.</span>', extra="width:300px;flex-shrink:0")
    side2 = card(t, ctitle(t, "بهینه‌ساز کلادفلر", "برای سرویس‌های مستقیم پشت CDN") +
                 f'<div style="display:flex;gap:10px">{stat(t, "IP تمیز این شبکه", "۵")}{stat(t, "بهترین", "88", "ms")}</div>'
                 f'<span style="font-size:12px;color:{t["text2"]}">شبکه: Wi-Fi · MikroTik-Home · ۲ ساعت پیش</span>'
                 + f'<a href="Desktop-Tools.dc.html" style="height:44px;border-radius:14px;background:{t["navy"]};color:{t["onNavy"]};display:flex;align-items:center;justify-content:center;gap:8px;font-size:14px;font-weight:700;text-decoration:none">{ic("radar", 18)}اسکن دوباره</a>', extra="width:300px;flex-shrink:0")
    actions = (f'<div style="display:flex;gap:8px;align-items:center">{clearsearch(t, "sq", "جستجوی سرور", 240)}'
               f'<button type="button" style="height:42px;padding:0 14px;border-radius:13px;{t["clear"]};display:flex;align-items:center;gap:8px;font-family:inherit;font-size:13px;font-weight:700;cursor:pointer">{ic("sort", 17, 2, "#FFFFFF")}کمترین پینگ</button>'
               f'<button type="button" style="height:42px;padding:0 6px 0 14px;border-radius:13px;border:none;background:#FFFFFF;color:#062845;display:flex;align-items:center;gap:10px;font-family:inherit;font-size:13px;font-weight:700;cursor:pointer">تست تأخیر همه<span style="width:30px;height:30px;border-radius:10px;background:#062845;color:#00ACFE;display:flex;align-items:center;justify-content:center">{ic("gauge", 16)}</span></button></div>')
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">'
            f'{header("سرورها", "۱۰ سرور از ۲ سرویس و ۱ لینک دستی · آخرین تست ۳ دقیقه پیش", actions)}'
            f'{clearchips(t, ["همه · ۱۰", "ستاره‌دار · ۲", "سرویس ۴۰ گیگ · ۶", "لینک‌های دستی · ۲"])}'
            f'<div style="flex-grow:1;display:flex;gap:16px;min-height:0">{table}<div style="display:flex;flex-direction:column;gap:16px">{side}{side2}</div></div></div>')
    return page("سرورها — دسکتاپ", shell(t, "سرورها", main), t=t)

# ============================================================================
def services_page():
    t = LIGHT
    def svc(title, uid, state, days, dayp, gb, gbp, tier, sel=False):
        st = badge(t, *state)
        border = f"outline:3px solid #FFFFFF;outline-offset:-1px;" if sel else ""
        return (f'<article style="flex-grow:1;flex-basis:0;border-radius:24px;{t["milk"]};overflow:hidden;{border}">'
                f'<div style="padding:16px;display:flex;flex-direction:column;gap:16px"><div style="display:flex;align-items:center;gap:12px">'
                f'<span style="width:48px;height:48px;border-radius:15px;background:#00ACFE;display:flex;align-items:center;justify-content:center"><img src="{LOGO}" alt="" style="width:32px;height:32px;object-fit:contain"></span>'
                f'<span style="flex-grow:1;display:flex;flex-direction:column;gap:2px"><span style="font-size:16px;font-weight:800">{title}</span><span style="display:flex;gap:6px;align-items:center"><span dir="ltr" style="{FONT};font-size:11px;color:{t["mute"]}">{uid}</span><span style="font-size:11px;color:{t["text2"]}">· {tier}</span></span></span>{st}</div>'
                f'<div style="display:flex;gap:18px"><div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;gap:4px"><span style="font-size:11px;color:{t["text2"]}">زمان باقی‌مانده</span><span style="font-size:28px;font-weight:800;line-height:1.1">{days} <span style="font-size:13px;color:{t["text2"]}">روز</span></span><div style="height:6px;border-radius:3px;background:{t["hair"]}"><div style="width:{dayp}%;height:100%;border-radius:3px;background:{t["warn"] if dayp < 20 else t["link"]}"></div></div></div>'
                f'<div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;gap:4px"><span style="font-size:11px;color:{t["text2"]}">حجم باقی‌مانده</span><span style="font-size:28px;font-weight:800;line-height:1.1">{gb} <span style="font-size:13px;color:{t["text2"]}">گیگ</span></span><div style="height:6px;border-radius:3px;background:{t["hair"]}"><div style="width:{gbp}%;height:100%;border-radius:3px;background:{t["warn"] if gbp < 20 else t["link"]}"></div></div></div></div></div>'
                f'<div style="position:relative;border-top:2px dashed {t["track"]};padding:12px;display:flex;gap:8px;background:{t["ticket"]}">'
                f'{btn(t, "تمدید", "refresh", "navy", 44, True)}{iconbtn(t, "qr", "لینک و QR سرویس", "soft", 44)}{iconbtn(t, "copy", "کپی لینک اشتراک", "soft", 44)}{iconbtn(t, "refresh", "بروزرسانی سرورها", "soft", 44)}</div></article>')
    cards = (f'<div style="display:flex;gap:16px">{svc("۴۰ گیگ · یک‌ماهه", "#fc34f6c7", ("متصل", "ok"), "۲۸", 93, "۲۷٫۶", 69, "مستقیم", True)}'
             f'{svc("۲۰ گیگ · تونل", "#a91c22e0", ("۳ روز مانده", "warn"), "۳", 10, "۱۸٫۲", 91, "تونل")}</div>')
    manual = (f'<div style="display:flex;align-items:center;justify-content:space-between"><span style="font-size:13px;font-weight:700;color:rgba(255,255,255,0.9)">لینک‌های دستی</span></div>'
              f'<div style="display:flex;gap:16px"><div style="flex-grow:1;flex-basis:0;border-radius:22px;{t["milk"]};overflow:hidden">'
              + row(t, "link", "My Sub", "۲ سرور · بروزرسانی خودکار روشن", f'<span style="display:flex;gap:6px">{iconbtn(t, "gear", "ویرایش لینک")}{iconbtn(t, "trash", "حذف لینک")}</span>', True) +
              f'</div><button type="button" style="flex-grow:1;flex-basis:0;border-radius:22px;border:1.5px dashed rgba(255,255,255,0.55);background:rgba(255,255,255,0.06);padding:14px 16px;display:flex;align-items:center;gap:14px;font-family:inherit;color:#FFFFFF;cursor:pointer;text-align:right"><span style="width:40px;height:40px;border-radius:13px;background:rgba(255,255,255,0.16);display:flex;align-items:center;justify-content:center;flex-shrink:0">{ic("plus", 20, 2, "#FFFFFF")}</span><span style="font-size:13px;line-height:1.8">لینک اشتراک یا کانفیگ دیگری داری؟ اینجا اضافه کن. <span dir="ltr" style="{FONT};opacity:0.8">Ctrl+V</span> هم کار می‌کند.</span></button></div>')
    qrpanel = (f'<section style="width:330px;flex-shrink:0;border-radius:26px;{t["milk"]};padding:20px;display:flex;flex-direction:column;gap:14px;align-items:stretch">'
               + ctitle(t, "لینک و QR سرویس", "۴۰ گیگ · یک‌ماهه", iconbtn(t, "x", "بستن")) +
               f'<div style="align-self:center;padding:14px;border-radius:22px;background:#FFFFFF;border:1px solid {t["chipBorder"]}">{qr(200, seed=11)}</div>'
               f'<div dir="ltr" style="border-radius:12px;background:{t["softBtn"]};padding:10px 12px;{FONT};font-size:11px;color:{t["text2"]};overflow:hidden;white-space:nowrap;text-overflow:ellipsis">https://[SUB-DOMAIN]/sub/[TOKEN]</div>'
               f'<div style="display:flex;gap:8px">{btn(t, "کپی لینک", "copy", "navy", 44, True)}{btn(t, "ذخیره‌ی QR", "dl", "soft", 44, True)}</div>'
               f'<div style="display:flex;gap:8px;border-radius:14px;background:{t["warnSoft"]};color:{t["warn"]};padding:10px 12px;font-size:12px;line-height:1.8">{ic("alert", 18)}<span>این لینک از حجم همین سرویس مصرف می‌کند؛ فقط روی دستگاه‌های خودت استفاده کن.</span></div></section>')
    actions = (f'<a href="Desktop-Shop.dc.html" style="height:42px;padding:0 6px 0 14px;border-radius:13px;background:#062845;color:#FFFFFF;display:flex;align-items:center;gap:10px;font-size:13px;font-weight:700;text-decoration:none">خرید سرویس<span style="width:30px;height:30px;border-radius:10px;background:#FFFFFF;color:#062845;display:flex;align-items:center;justify-content:center">{ic("plus", 16, 2.6)}</span></a>')
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">{header("سرویس‌های من", "سرویس‌های حساب خودکار با ربات همگام می‌شوند", actions)}'
            f'{clearchips(t, ["فعال · ۲", "همه · ۳"])}{cards}{manual}</div>{qrpanel}')
    return page("سرویس‌ها — دسکتاپ", shell(t, "سرویس‌ها", main), t=t)

# ============================================================================
def shop_page():
    t = LIGHT
    tier = segmented(t, ["مستقیم", "تونل", "ویژه"], 0, 46)
    dur = "".join(
        f'<button type="button" aria-pressed="{"true" if i == 0 else "false"}" style="flex-grow:1;flex-basis:0;height:66px;border-radius:16px;{"border:none;background:#062845;color:#FFFFFF" if i == 0 else "border:1px solid #D5E3EF;background:#FFFFFF;color:#062845"};font-family:inherit;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:2px;cursor:pointer"><span style="font-size:20px;font-weight:800">{fa(m)}</span><span style="font-size:11px;font-weight:600;opacity:0.75">ماهه</span></button>'
        for i, m in enumerate((1, 2, 3, 6)))
    vol = "".join(
        f'<button type="button" aria-pressed="{"true" if v == 40 else "false"}" style="height:48px;border-radius:13px;{"border:2px solid #0078C8;background:#EAF5FD" if v == 40 else "border:1px solid #D5E3EF;background:#FFFFFF"};color:#062845;{FONT};font-size:16px;font-weight:700;cursor:pointer">{v}</button>'
        for v in (10, 20, 40, 60, 100, 200))
    left = card(t,
                ctitle(t, "نوع سرویس", "مستقیم برای اسکنر کلادفلر، تونل برای شبکه‌های سخت‌گیر") + tier +
                f'<span style="font-size:13px;font-weight:800;margin-top:4px">مدت اعتبار</span><div style="display:flex;gap:8px">{dur}</div>'
                f'<div style="display:flex;justify-content:space-between;align-items:center;margin-top:4px"><span style="font-size:13px;font-weight:800">حجم (گیگابایت)</span>{badge(t, "٪۲۰ تخفیف", "ok")}</div>'
                f'<div style="display:grid;grid-template-columns:repeat(6,minmax(0,1fr));gap:8px">{vol}</div>'
                f'<div style="display:flex;gap:10px;align-items:flex-end"><div style="flex-grow:1">{field(t, "cp", "کد تخفیف", "", "اگر کد داری وارد کن")}</div>{btn(t, "بررسی", None, "soft", 44)}</div>',
                extra="flex-grow:1;flex-basis:0", pad=20)
    methods = "".join(
        f'<label style="display:flex;align-items:center;gap:12px;padding:12px 14px;border-radius:16px;{"background:#EAF5FD;border:2px solid #0078C8" if i == 0 else "background:#FFFFFF;border:1px solid #D5E3EF"};cursor:pointer">'
        f'<input type="radio" name="pm" {"checked" if i == 0 else ""} style="width:18px;height:18px;accent-color:#062845;margin:0">'
        f'<span style="width:36px;height:36px;border-radius:11px;background:#EAF5FD;color:#0078C8;display:flex;align-items:center;justify-content:center;flex-shrink:0">{ic(icn, 18)}</span>'
        f'<span style="flex-grow:1;display:flex;flex-direction:column"><span style="font-size:14px;font-weight:800;color:#062845">{n}</span><span style="font-size:12px;color:#3F5F7E">{s}</span></span></label>'
        for i, (icn, n, s) in enumerate((("wallet", "کیف پول", "موجودی: ۰ تومان"), ("copy", "کارت به کارت", "بررسی رسید توسط پشتیبانی"), ("globe", "درگاه آنلاین", "بازگشت خودکار به برنامه"))))
    right = (f'<div style="width:380px;flex-shrink:0;display:flex;flex-direction:column;gap:16px">'
             f'<div style="border-radius:20px;{t["clear"]};display:flex;align-items:center;gap:12px;padding:12px 12px 12px 16px">{ic("gift", 22, 2, "#FFFFFF")}<span style="flex-grow:1;display:flex;flex-direction:column"><span style="font-size:14px;font-weight:800">تست رایگان</span><span style="font-size:12px;opacity:0.9">۵۰ مگابایت · ۲ روز · یک بار برای هر حساب</span></span><button type="button" style="height:38px;padding:0 14px;border-radius:12px;border:none;background:#FFFFFF;color:#062845;font-family:inherit;font-size:13px;font-weight:800;cursor:pointer">دریافت</button></div>'
             + card(t, ctitle(t, "روش پرداخت") + methods, pad=16) +
             f'<section style="border-radius:24px;background:#062845;padding:16px;display:flex;align-items:center;gap:12px;box-shadow:0 14px 30px rgba(2,24,56,0.35)"><span style="flex-grow:1;display:flex;flex-direction:column;gap:2px"><span style="font-size:11px;opacity:0.7;text-decoration:line-through">[قیمت] تومان</span><span style="font-size:19px;font-weight:800">[مبلغ نهایی] تومان</span><span style="font-size:11px;color:#00ACFE">مستقیم · ۴۰ گیگ · ۳۰ روز</span></span>'
             f'<button type="button" style="height:52px;padding:0 20px;border-radius:15px;border:none;background:#00ACFE;color:#062845;font-family:inherit;font-size:15px;font-weight:800;display:flex;align-items:center;gap:8px;cursor:pointer">{ic("lock", 18, 2.2)}پرداخت</button></section></div>')
    wallet = (f'<a href="Desktop-Account.dc.html" style="height:40px;padding:0 12px 0 6px;border-radius:13px;{t["clear"]};display:flex;align-items:center;gap:8px;text-decoration:none;font-size:13px;font-weight:700"><span style="width:28px;height:28px;border-radius:9px;background:#FFFFFF;color:#062845;display:flex;align-items:center;justify-content:center">{ic("wallet", 16, 2.2)}</span>۰ تومان</a>')
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">{header("فروشگاه", "قیمت نهایی را فقط سرور حساب می‌کند", wallet)}'
            f'<div style="flex-grow:1;display:flex;gap:16px;min-height:0">{left}{right}</div></div>')
    return page("فروشگاه — دسکتاپ", shell(t, "فروشگاه", main), t=t)

# ============================================================================
def account_page():
    t = LIGHT
    prof = (f'<div style="display:flex;align-items:center;gap:14px"><span style="width:64px;height:64px;border-radius:21px;{t["clear"]};display:flex;align-items:center;justify-content:center">{ic("user", 30, 1.8, "#FFFFFF")}</span>'
            f'<span style="flex-grow:1;display:flex;flex-direction:column;gap:6px"><span style="font-size:22px;font-weight:800">کاربر ۱۰۱۱۷۸۸۱۲۳</span><span style="align-self:flex-start;height:26px;padding:0 10px;border-radius:9px;background:rgba(255,255,255,0.18);display:flex;align-items:center;gap:6px;font-size:12px;font-weight:700">{ic("plane", 13, 2, "#FFFFFF")}متصل به تلگرام · این دستگاه: DESKTOP-7Q2 (Windows)</span></span>'
            f'<button type="button" style="height:42px;padding:0 14px;border-radius:13px;{t["clear"]};display:flex;align-items:center;gap:8px;font-family:inherit;font-size:13px;font-weight:700;cursor:pointer">{ic("logout", 17, 2, "#FFFFFF")}خروج از حساب</button></div>')
    wallet = (f'<div style="border-radius:24px;background:#062845;padding:18px;display:flex;flex-direction:column;gap:14px;position:relative;overflow:hidden;box-shadow:0 14px 30px rgba(2,24,56,0.35)">'
              f'<img src="{GLASSES}" alt="" style="position:absolute;left:-26px;bottom:-34px;width:150px;height:150px;opacity:0.18">'
              f'<span style="position:relative;display:flex;align-items:center;gap:8px;font-size:12px;opacity:0.8">{ic("wallet", 16, 2, "#FFFFFF")}موجودی کیف پول</span>'
              f'<span style="position:relative;font-size:32px;font-weight:800;line-height:1">۰ <span style="font-size:14px;opacity:0.8">تومان</span></span>'
              f'<div style="position:relative;display:flex;gap:8px"><button type="button" style="flex-grow:1;height:46px;border-radius:14px;border:none;background:#00ACFE;color:#062845;display:flex;align-items:center;justify-content:center;gap:6px;font-family:inherit;font-size:14px;font-weight:800;cursor:pointer">{ic("plus", 17, 2.6)}افزایش موجودی</button><button type="button" style="height:46px;padding:0 16px;border-radius:14px;border:none;background:rgba(255,255,255,0.12);color:#FFFFFF;font-family:inherit;font-size:13px;font-weight:700;cursor:pointer">تراکنش‌ها</button></div></div>')
    ref = (f'<a href="Desktop-Referral.dc.html" style="border-radius:22px;{t["clear"]};display:flex;align-items:center;gap:12px;padding:14px 16px;text-decoration:none;color:#FFFFFF">{ic("gift", 24, 2, "#FFFFFF")}'
           f'<span style="flex-grow:1;display:flex;flex-direction:column"><span style="font-size:15px;font-weight:800">دعوت از دوستان</span><span style="font-size:12px;opacity:0.9">۴ دعوت · ۱ خرید · درآمد ۰ تومان</span></span>{ic("chev", 18, 2, "#FFFFFF")}</a>')
    col1 = f'<div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;gap:14px">{wallet}{ref}' + group(t, [
        row(t, "chart", "مصرف روزانه", "ترافیک VPN همین کامپیوتر، ۷ یا ۳۰ روز", href="Desktop-Usage.dc.html"),
        row(t, "gauge", "تست سرعت", "پینگ، دانلود و آپلود", href="Desktop-Tools.dc.html", last=True)], "ابزارها") + '</div>'
    col2 = f'<div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;gap:14px">' + group(t, [
        row(t, "chat", "تیکت‌های من", "جواب‌های پشتیبانی، داخل برنامه", f'<span style="display:flex;align-items:center;gap:8px">{badge(t, "۲ جدید", "bad")}<span style="color:{t["mute"]};display:flex">{ic("chev", 18)}</span></span>', href="Desktop-Support.dc.html"),
        row(t, "flag", "گزارش مشکل", "یک گزارش فنی برای پشتیبانی می‌فرستد", href="Desktop-Report.dc.html"),
        row(t, "plane", "ربات پشتیبانی", "@GeekVPN_bot", last=True)], "پشتیبانی") + group(t, [
        row(t, "dl", "به‌روزرسانی برنامه", "نسخه‌ی ۱٫۲٫۰ آماده است", f'<span style="display:flex;align-items:center;gap:8px">{badge(t, "جدید", "link")}<span style="color:{t["mute"]};display:flex">{ic("chev", 18)}</span></span>', href="Desktop-Update.dc.html"),
        row(t, "info", "درباره‌ی برنامه", "نسخه‌ی ۱٫۱٫۴ · Xray 25.9 · sing-box 1.12 · cfscan 1.0.0", last=True)], "برنامه") + '</div>'
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:16px;min-width:0">{prof}<div style="display:flex;gap:16px;flex-grow:1">{col1}{col2}</div></div>')
    return page("حساب — دسکتاپ", shell(t, "حساب", main), t=t)

# ============================================================================
def settings_page():
    t = LIGHT
    theme = segmented(t, ["روشن", "تیره", "هماهنگ با سیستم"], 2, 42)
    mode = (f'<div style="display:flex;gap:10px">'
            + "".join(f'<label style="flex-grow:1;flex-basis:0;display:flex;gap:10px;padding:12px;border-radius:16px;{"background:#EAF5FD;border:2px solid #0078C8" if i == 1 else "background:#FFFFFF;border:1px solid #D5E3EF"};cursor:pointer">'
                      f'<input type="radio" name="md" {"checked" if i == 1 else ""} style="width:18px;height:18px;accent-color:#062845;margin:2px 0 0">'
                      f'<span style="display:flex;flex-direction:column;gap:2px"><span style="font-size:14px;font-weight:800">{n}</span><span style="font-size:12px;line-height:1.7;color:#3F5F7E">{s}</span></span></label>'
                      for i, (n, s) in enumerate((("پروکسی سیستم", "مرورگرها و برنامه‌هایی که پروکسی سیستم را می‌خوانند؛ بدون دسترسی مدیر"),
                                                  ("TUN (کل سیستم)", "همه‌ی برنامه‌ها، بازی‌ها و ترمینال؛ لازمه‌ی Kill Switch و تونل برنامه‌ای")))) + '</div>')
    route = segmented(t, ["هوشمند", "سراسری", "مستقیم"], 0, 42)
    colA = (f'<div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;gap:14px;min-width:0">'
            + card(t, ctitle(t, "حالت اتصال") + mode + f'<span style="font-size:13px;font-weight:800">مسیر ترافیک</span>{route}'
                   f'<span style="font-size:12px;line-height:1.8;color:{t["text2"]}">هوشمند: سایت‌ها و IPهای ایران مستقیم، بقیه از VPN.</span>', pad=18)
            + group(t, [row(t, "split", "تونل تفکیکی", "سایت‌ها و برنامه‌های ایرانی مستقیم", href="Desktop-Split.dc.html"),
                        row(t, "gear", "تنظیمات پیشرفته", "هسته، DNS، پورت‌ها، Mux و Fragment", last=True)]) + '</div>')
    colB = (f'<div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;gap:14px;min-width:0">' + group(t, [
        row(t, "power", "اجرا با روشن شدن سیستم", "کوچک‌شده در کنار ساعت شروع می‌شود", switch(t, True, "اجرا با ویندوز")),
        row(t, "bolt", "اتصال خودکار بعد از اجرا", "با اتصال هوشمند", switch(t, True, "اتصال خودکار")),
        row(t, "wifi", "روی شبکه‌ی ناشناس", "کافه، هتل و Wi-Fi عمومی؛ نه شبکه‌های مورد اعتماد", switch(t, True, "شبکه ناشناس")),
        row(t, "shield", "این شبکه مورد اعتماد است", "MikroTik-Home · اینجا خودکار وصل نشو", switch(t, False, "شبکه مورد اعتماد"), last=True)], "اتصال خودکار")
        + group(t, [
            row(t, "wall", "Kill Switch", "اگر تونل افتاد، اینترنت بدون VPN بسته شود", switch(t, True, "Kill Switch"), tone="bad"),
            row(t, "lock", "حالت سخت‌گیر", "بعد از کرش یا ری‌استارت هم باز نشود تا دوباره وصل شوی", switch(t, False, "حالت سخت‌گیر"), tone="bad"),
            row(t, "hub", "اجازه به شبکه‌ی محلی", "پرینتر، NAS و دستگاه‌های خانه", switch(t, True, "شبکه محلی"), last=True)], "Kill Switch")
        + '</div>')
    colC = (f'<div style="width:300px;flex-shrink:0;display:flex;flex-direction:column;gap:14px">'
            + card(t, ctitle(t, "ظاهر") + theme, pad=16)
            + group(t, [
                row(t, "lock", "قفل برنامه", "Windows Hello / Touch ID / رمز سیستم", switch(t, False, "قفل برنامه")),
                row(t, "refresh", "بروزرسانی خودکار سرویس‌ها", "هر بار اجرای برنامه", switch(t, True, "بروزرسانی خودکار")),
                row(t, "tray", "بستن = رفتن به کنار ساعت", "برنامه در tray می‌ماند", switch(t, True, "کوچک به tray")),
                row(t, "alert", "هشدار تمام شدن سرویس", "۸۰٪ حجم یا ۳ روز مانده", switch(t, True, "هشدار سرویس"), last=True)], "عمومی")
            + group(t, [row(t, "keyboard", "میانبرهای کیبورد", f'<span dir="ltr" style="{FONT}">Ctrl+Shift+K</span> اتصال/قطع', last=True)])
            + '</div>')
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">{header("تنظیمات", "تغییرهای اتصال از اتصال بعدی اعمال می‌شوند")}'
            f'<div style="flex-grow:1;display:flex;gap:16px;min-height:0">{colA}{colB}{colC}</div></div>')
    return page("تنظیمات — دسکتاپ", shell(t, "تنظیمات", main), t=t)

# ============================================================================
def split_page():
    t = LIGHT
    iran = (f'<section style="border-radius:24px;background:#062845;padding:18px;display:flex;gap:16px;align-items:center;box-shadow:0 14px 30px rgba(2,24,56,0.35)">'
            f'<span style="width:54px;height:54px;border-radius:17px;background:#00ACFE;color:#062845;display:flex;align-items:center;justify-content:center;flex-shrink:0">{ic("split", 26, 2)}</span>'
            f'<span style="flex-grow:1;display:flex;flex-direction:column;gap:4px"><span style="font-size:17px;font-weight:800">سایت‌ها و اپ‌های ایرانی مستقیم</span><span style="font-size:12px;line-height:1.8;opacity:0.85">دامنه‌ها و IPهای ایران (geosite:ir و geoip:ir) و برنامه‌های ایرانی شناخته‌شده از VPN رد نمی‌شوند؛ بانک‌ها و درگاه‌ها بدون خطا کار می‌کنند.</span></span>'
            f'{switch(dict(t, navy="#00ACFE", knobOn="#062845"), True, "ایرانی مستقیم")}</section>')
    rules = card(t, ctitle(t, "قوانین دامنه و IP", "به‌روزرسانی خودکار هر ۲۴ ساعت", iconbtn(t, "refresh", "بروزرسانی قوانین")) +
                 "".join(f'<div style="display:flex;align-items:center;gap:10px;padding:10px 12px;border-radius:14px;background:{t["softBtn"]}"><span dir="ltr" style="{FONT};font-size:13px;font-weight:700;flex-grow:1;text-align:right">{r}</span>{badge(t, a, tone)}</div>'
                         for r, a, tone in (("geosite:ir", "مستقیم", "ok"), ("geoip:ir", "مستقیم", "ok"), ("geosite:category-ads-all", "مسدود", "bad"), ("digikala.com", "مستقیم", "ok"), ("*.corp.example", "مستقیم", "ok"))) +
                 f'<div style="display:flex;gap:8px"><div style="flex-grow:1">{field(t, "nr", "افزودن قانون", "", "دامنه، IP یا CIDR", mono=True)}</div></div>'
                 f'{segmented(t, ["مستقیم", "VPN", "مسدود"], 0, 40)}', extra="width:360px;flex-shrink:0")
    apps = [("Telegram Desktop", "C:\\Users\\me\\AppData\\Roaming\\Telegram Desktop\\Telegram.exe", "مستقیم", True),
            ("بله", "C:\\Program Files\\Bale\\Bale.exe", "مستقیم", True),
            ("AnyDesk", "C:\\Program Files (x86)\\AnyDesk\\AnyDesk.exe", "مستقیم", False),
            ("Steam", "C:\\Program Files (x86)\\Steam\\steam.exe", "VPN", False),
            ("Chrome", "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", "VPN", False)]
    arows = "".join(
        f'<div style="display:grid;grid-template-columns:40px minmax(0,1fr) 150px 36px;align-items:center;gap:12px;padding:10px 16px;border-bottom:1px solid {t["hair"]}">'
        f'<span style="width:40px;height:40px;border-radius:12px;background:{t["soft"]};color:{t["link"]};display:flex;align-items:center;justify-content:center;{FONT};font-size:14px;font-weight:700">{n[0]}</span>'
        f'<span style="display:flex;flex-direction:column;min-width:0"><span style="font-size:14px;font-weight:800">{n}{" · پیشنهاد ایرانی" if iran_ else ""}</span><span dir="ltr" style="{FONT};font-size:11px;color:{t["mute"]};white-space:nowrap;overflow:hidden;text-overflow:ellipsis;text-align:right">{p}</span></span>'
        f'{segmented(t, ["مستقیم", "VPN"], 0 if m == "مستقیم" else 1, 36)}'
        f'<button type="button" aria-label="حذف {n}" style="width:36px;height:36px;border:none;background:none;color:{t["mute"]};cursor:pointer;display:flex;align-items:center;justify-content:center">{ic("trash", 18)}</button></div>'
        for n, p, m, iran_ in apps)
    applist = (f'<section style="flex-grow:1;border-radius:26px;{t["milk"]};display:flex;flex-direction:column;overflow:hidden">'
               f'<div style="padding:16px;display:flex;gap:8px;align-items:center;border-bottom:1px solid {t["hair"]}">'
               f'<span style="flex-grow:1;display:flex;flex-direction:column"><span style="font-size:16px;font-weight:800">برنامه‌ها</span><span style="font-size:12px;color:{t["text2"]}">فقط در حالت TUN · با مسیر فایل اجرایی شناخته می‌شوند</span></span>'
               f'{btn(t, "برنامه‌های در حال اجرا", "apps", "soft", 40)}{btn(t, "انتخاب فایل اجرایی", "folder", "navy", 40)}</div>{arows}</section>')
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">{header("تونل تفکیکی", "تنظیمات ← تونل تفکیکی · تغییرها از اتصال بعدی اعمال می‌شوند")}{iran}'
            f'<div style="flex-grow:1;display:flex;gap:16px;min-height:0">{applist}{rules}</div></div>')
    return page("تونل تفکیکی — دسکتاپ", shell(t, "تنظیمات", main), t=t)

# ============================================================================
def tools_page():
    t = LIGHT
    res = [("104.18.34.121", 88, 142, 3.1, "FRA", 5.8), ("172.67.180.12", 94, 150, 4.0, "FRA", 5.1), ("104.21.77.5", 101, 163, 2.6, "AMS", 4.7),
           ("162.159.140.9", 117, 181, 6.2, "IST", 3.9), ("104.16.132.229", 126, 199, 5.5, "FRA", 3.2)]
    rrows = "".join(
        f'<div style="display:grid;grid-template-columns:minmax(0,1.5fr) 70px 80px 70px 60px 80px 88px;align-items:center;gap:8px;padding:9px 16px;border-bottom:1px solid {t["hair"]};{FONT};font-size:13px">'
        f'<span dir="ltr" style="font-weight:700;text-align:right">{ip}</span><span>{p}ms</span><span>{l}ms</span><span>{j}ms</span><span>{c}</span><span>{d} MB/s</span>'
        f'<button type="button" style="height:30px;border-radius:10px;border:none;background:{t["soft"] if i else t["navy"]};color:{t["link"] if i else t["onNavy"]};font-family:Vazirmatn,sans-serif;font-size:12px;font-weight:700;cursor:pointer">{"استفاده" if i else "در حال استفاده"}</button></div>'
        for i, (ip, p, l, j, c, d) in enumerate(res))
    scanner = (f'<section style="flex-grow:1;flex-basis:0;border-radius:26px;{t["milk"]};display:flex;flex-direction:column;overflow:hidden;min-width:0">'
               f'<div style="padding:16px;display:flex;flex-direction:column;gap:12px;border-bottom:1px solid {t["hair"]}">'
               + ctitle(t, "بهینه‌ساز کلادفلر", "اسکن IP تمیز برای سرویس‌های مستقیم پشت CDN", btn(t, "توقف", "x", "danger", 40)) +
               f'<div style="display:flex;gap:10px;align-items:center"><span style="flex-grow:1;display:flex;flex-direction:column;gap:6px"><span style="display:flex;justify-content:space-between;font-size:12px;color:{t["text2"]}"><span>۱۸۴ از ۳۰۰ آدرس · ۵ سالم</span><span dir="ltr" style="{FONT}">SNI: cdn.[YOUR-DOMAIN]</span></span>'
               f'<span style="height:8px;border-radius:4px;background:{t["track"]};overflow:hidden"><span style="display:block;width:61%;height:100%;background:{t["link"]}"></span></span></span></div>'
               f'<div style="display:flex;gap:8px;flex-wrap:wrap">{badge(t, "شبکه: Wi-Fi · MikroTik-Home", "link")}{badge(t, "دامنه تأییدشده در رنج کلادفلر", "ok")}{badge(t, "پینگ ICMP خاموش؛ TCP + TLS", "warn")}</div></div>'
               f'<div style="display:grid;grid-template-columns:minmax(0,1.5fr) 70px 80px 70px 60px 80px 88px;gap:8px;padding:10px 16px;font-size:12px;font-weight:700;color:{t["text2"]};border-bottom:1px solid {t["hair"]}"><span>IP</span><span>پینگ</span><span>تأخیر</span><span>لرزش</span><span>colo</span><span>دانلود</span><span></span></div>{rrows}'
               f'<div style="padding:12px 16px;display:flex;gap:8px;margin-top:auto">{btn(t, "بازگشت به IP اصلی", "refresh", "soft", 40)}<span style="flex-grow:1"></span><span style="font-size:12px;color:{t["text2"]};align-self:center">بهترین IP خودکار اعمال شد · اتصال دوباره ساخته شد</span></div></section>')
    # speed test gauge
    import math
    def arc(p, r=92, c=110):
        a0, a1 = math.radians(150), math.radians(150 + 240 * p)
        x0, y0 = c + r * math.cos(a0), c + r * math.sin(a0)
        x1, y1 = c + r * math.cos(a1), c + r * math.sin(a1)
        large = 1 if 240 * p > 180 else 0
        return f"M{x0:.1f} {y0:.1f} A{r} {r} 0 {large} 1 {x1:.1f} {y1:.1f}"
    gauge = (f'<div style="position:relative;width:220px;height:190px;align-self:center"><svg aria-hidden="true" width="220" height="220" viewBox="0 0 220 220" style="position:absolute;top:0;right:0">'
             f'<path d="{arc(1)}" fill="none" stroke="{t["track"]}" stroke-width="14" stroke-linecap="round"/><path d="{arc(0.58)}" fill="none" stroke="{t["link"]}" stroke-width="14" stroke-linecap="round"/></svg>'
             f'<div style="position:absolute;top:70px;right:0;left:0;display:flex;flex-direction:column;align-items:center"><span dir="ltr" style="{FONT};font-size:40px;font-weight:700;line-height:1">48.6</span><span style="font-size:12px;color:{t["text2"]}">مگابیت بر ثانیه · دانلود</span></div></div>')
    speed = card(t, ctitle(t, "تست سرعت", "از داخل تونل فعلی · آلمان · کلاد") + gauge +
                 f'<div style="display:flex;gap:10px;text-align:center">{stat(t, "پینگ", "121", "ms", 22)}{stat(t, "لرزش", "8", "ms", 22)}{stat(t, "آپلود", "—", "", 22)}</div>'
                 f'<div style="display:flex;gap:6px">{badge(t, "پینگ", "ok")}{badge(t, "دانلود…", "warn")}{badge(t, "آپلود", "link")}</div>'
                 f'{btn(t, "شروع دوباره", "gauge", "navy", 46)}<span style="font-size:11px;line-height:1.8;color:{t["text2"]}">هر مرحله حداکثر ۱۰ ثانیه (دانلود تا ۵۰، آپلود تا ۲۰ مگابایت).</span>',
                 extra="width:330px;flex-shrink:0", pad=18)
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">{header("ابزارها", "اسکنر IP تمیز و تست سرعت")}'
            f'{clearchips(t, ["اسکنر و تست سرعت", "تست تأخیر سرورها", "لاگ هسته"])}<div style="flex-grow:1;display:flex;gap:16px;min-height:0">{scanner}{speed}</div></div>')
    return page("ابزارها — دسکتاپ", shell(t, "ابزارها", main), t=t)

# ============================================================================
def usage_page():
    t = LIGHT
    vals = [0.8, 1.4, 0.6, 2.2, 1.9, 0.4, 1.1, 2.8, 3.1, 1.2, 0.9, 1.6, 2.4, 0.7, 1.3, 1.8, 2.0, 0.5, 0.9, 1.7, 2.6, 3.4, 1.0, 0.6, 1.5, 2.1, 1.2, 0.8, 2.3, 1.6]
    W, H, pad = 760, 260, 28
    mx = 4
    bw = (W - pad) / len(vals)
    grid = "".join(f'<line x1="{pad}" x2="{W}" y1="{H - 24 - (H-40)*g/mx:.1f}" y2="{H - 24 - (H-40)*g/mx:.1f}" stroke="{t["hair"]}" stroke-width="1"/><text x="0" y="{H - 20 - (H-40)*g/mx:.1f}" font-size="11" fill="{t["text2"]}" font-family="Space Grotesk">{g} GB</text>' for g in range(0, 5))
    barsvg = ""
    for i, v in enumerate(vals):
        h = (H - 40) * v / mx
        x = W - (i + 1) * bw + 3
        peak = v == max(vals)
        barsvg += f'<rect x="{x:.1f}" y="{H - 24 - h:.1f}" width="{bw - 6:.1f}" height="{h:.1f}" rx="4" fill="{t["navy"] if peak else t["link"]}"/>'
    labels = "".join(f'<text x="{W - (i + 0.5) * bw:.1f}" y="{H - 6}" font-size="11" text-anchor="middle" fill="{t["text2"]}" font-family="Vazirmatn">{fa(i+1)} مهر</text>' for i in range(0, 30, 5))
    chart = (f'<svg role="img" aria-label="مصرف روزانه‌ی ۳۰ روز اخیر" width="{W}" height="{H}" viewBox="0 0 {W} {H}" style="display:block;max-width:100%">{grid}{barsvg}{labels}</svg>')
    main_card = card(t, ctitle(t, "مصرف روزانه", "ترافیک VPN همین کامپیوتر · سهم دستگاه‌های دیگر در آن نیست", segmented(t, ["۷ روز", "۳۰ روز"], 1, 38).replace("display:flex;gap:4px", "display:flex;gap:4px;width:180px")) + chart,
                     extra="flex-grow:1", pad=18)
    stats = (f'<div style="display:flex;gap:16px">' + "".join(card(t, stat(t, l, v, u, 28), extra="flex-grow:1;flex-basis:0") for l, v, u in (
        ("مجموع ۳۰ روز", "۴۶٫۶", "گیگ"), ("میانگین روزانه", "۱٫۵۵", "گیگ"), ("پرمصرف‌ترین روز", "۲۲ مهر", "۳٫۴ گیگ"), ("باقی‌مانده‌ی سرویس", "۲۷٫۶", "از ۴۰ گیگ"))) + '</div>')
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">{header("مصرف داده", "تاریخچه روی همین کامپیوتر ساخته می‌شود و ۴۵ روز می‌ماند")}{stats}{main_card}</div>')
    return page("مصرف روزانه — دسکتاپ", shell(t, "حساب", main), t=t)

# ============================================================================
def support_page():
    t = LIGHT
    tickets = [("مشکل اتصال", "روی ایرانسل وصل نمی‌شود", ("پاسخ داده شد", "ok"), "۲ جدید", True),
               ("پرداخت و مالی", "رسید کارت به کارت", ("منتظر پاسخ تو", "warn"), "", False),
               ("سرعت و کیفیت", "کندی عصرها", ("بسته شده", "link"), "", False)]
    tl = "".join(
        f'<button type="button" aria-pressed="{"true" if sel else "false"}" style="display:flex;flex-direction:column;gap:6px;padding:12px 14px;border-radius:16px;{"background:#EAF5FD;border:2px solid #0078C8" if sel else "background:none;border:2px solid transparent"};font-family:inherit;color:{t["text"]};cursor:pointer;text-align:right">'
        f'<span style="display:flex;align-items:center;gap:8px"><span style="flex-grow:1;font-size:14px;font-weight:800">{s}</span>{badge(t, u, "bad") if u else ""}</span>'
        f'<span style="display:flex;align-items:center;gap:8px"><span style="flex-grow:1;font-size:12px;color:{t["text2"]}">{tp}</span>{badge(t, *st)}</span></button>'
        for tp, s, st, u, sel in tickets)
    left = (f'<section style="width:330px;flex-shrink:0;border-radius:26px;{t["milk"]};padding:14px;display:flex;flex-direction:column;gap:8px">'
            + ctitle(t, "تیکت‌های من", "جواب‌ها در ربات تلگرام هم می‌آیند") + btn(t, "تیکت جدید", "plus", "navy", 44) + tl +
            f'<a href="Desktop-Report.dc.html" style="margin-top:auto;display:flex;align-items:center;gap:10px;padding:12px;border-radius:16px;background:{t["softBtn"]};text-decoration:none;color:{t["text"]}">'
            f'<span style="width:36px;height:36px;border-radius:11px;background:{t["warnSoft"]};color:{t["warn"]};display:flex;align-items:center;justify-content:center">{ic("flag", 18)}</span><span style="display:flex;flex-direction:column"><span style="font-size:13px;font-weight:800">گزارش مشکل اتصال</span><span style="font-size:11px;color:{t["text2"]}">با گزارش فنی حذف‌شده از اطلاعات حساس</span></span></a></section>')
    def bubble(me, text, when):
        if me:
            return (f'<div style="align-self:flex-start;max-width:70%;display:flex;flex-direction:column;gap:4px"><div style="border-radius:18px 18px 18px 6px;background:{t["navy"]};color:{t["onNavy"]};padding:12px 14px;font-size:13px;line-height:1.9">{text}</div><span style="font-size:11px;color:{t["text2"]}">تو · {when}</span></div>')
        return (f'<div style="align-self:flex-end;max-width:70%;display:flex;flex-direction:column;gap:4px;align-items:flex-end"><div style="border-radius:18px 18px 6px 18px;background:{t["softBtn"]};padding:12px 14px;font-size:13px;line-height:1.9">{text}</div><span style="font-size:11px;color:{t["text2"]}">پشتیبانی · {when}</span></div>')
    thread = (f'<section style="flex-grow:1;border-radius:26px;{t["milk"]};display:flex;flex-direction:column;overflow:hidden;min-width:0">'
              f'<div style="padding:14px 18px;border-bottom:1px solid {t["hair"]};display:flex;align-items:center;gap:10px"><span style="flex-grow:1;display:flex;flex-direction:column"><span style="font-size:16px;font-weight:800">روی ایرانسل وصل نمی‌شود</span><span dir="ltr" style="{FONT};font-size:11px;color:{t["mute"]};text-align:right">T-2647 · مشکل اتصال</span></span>{badge(t, "پاسخ داده شد", "ok")}</div>'
              f'<div style="flex-grow:1;padding:18px;display:flex;flex-direction:column;gap:14px">'
              + bubble(True, "از دیشب روی ایرانسل هیچ سروری وصل نمی‌شود. روی وای‌فای خانه مشکلی نیست.", "دیروز ۲۲:۱۰")
              + bubble(False, "سلام؛ لطفاً در ابزارها ← بهینه‌ساز کلادفلر یک اسکن روی ایرانسل بزنید و نتیجه را بگویید.", "امروز ۰۹:۴۲")
              + bubble(False, "اگر IP تمیزی پیدا نشد، سرویس تونل را از فروشگاه امتحان کنید.", "امروز ۰۹:۴۳") +
              f'</div><div style="padding:12px;border-top:1px solid {t["hair"]};display:flex;gap:8px;align-items:flex-end"><label for="rp" style="position:absolute;width:1px;height:1px;overflow:hidden">جوابت</label>'
              f'<textarea id="rp" placeholder="جوابت را بنویس… (حداقل ۱۰ حرف)" style="flex-grow:1;height:48px;resize:none;border-radius:14px;border:1px solid {t["chipBorder"]};background:{t["chip"]};padding:12px;font-family:inherit;font-size:13px;box-sizing:border-box"></textarea>'
              f'{iconbtn(t, "file", "پیوست گزارش فنی", "soft", 48)}{btn(t, "ارسال", "send", "navy", 48)}</div></section>')
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">{header("پشتیبانی", "هر ۲۰ ثانیه تازه می‌شود")}<div style="flex-grow:1;display:flex;gap:16px;min-height:0">{left}{thread}</div></div>')
    return page("پشتیبانی — دسکتاپ", shell(t, "پشتیبانی", main), t=t)

# ============================================================================
def referral_page():
    t = LIGHT
    hero = (f'<section style="border-radius:26px;background:#062845;padding:24px;display:flex;gap:24px;align-items:center;position:relative;overflow:hidden;box-shadow:0 14px 30px rgba(2,24,56,0.35)">'
            f'<img src="{GLASSES}" alt="" style="position:absolute;left:-30px;bottom:-40px;width:200px;height:200px;opacity:0.14">'
            f'<div style="position:relative;flex-grow:1;display:flex;flex-direction:column;gap:10px"><span style="font-size:13px;opacity:0.8">لینک دعوت تو</span>'
            f'<div style="display:flex;gap:10px;align-items:center"><span dir="ltr" style="flex-grow:1;height:50px;border-radius:14px;background:rgba(255,255,255,0.1);display:flex;align-items:center;padding:0 14px;{FONT};font-size:15px;font-weight:600">https://t.me/[BOT]?start=ref_K7Q2MX</span>'
            f'<button type="button" style="height:50px;padding:0 18px;border-radius:14px;border:none;background:#00ACFE;color:#062845;font-family:inherit;font-size:14px;font-weight:800;display:flex;align-items:center;gap:8px;cursor:pointer">{ic("copy", 18, 2.2)}کپی لینک</button></div>'
            f'<span style="font-size:13px;opacity:0.85">کد معرف: <span dir="ltr" style="{FONT};font-weight:700;color:#00ACFE">K7Q2MX</span></span></div>'
            f'<div style="position:relative;padding:12px;border-radius:20px;background:#FFFFFF">{qr(132, seed=5)}</div></section>')
    stats = f'<div style="display:flex;gap:16px">' + "".join(card(t, stat(t, l, v, u, 30), extra="flex-grow:1;flex-basis:0") for l, v, u in (
        ("دعوت‌شده", "۴", "نفر"), ("خرید کرده", "۱", "نفر"), ("درآمد", "۰", "تومان"), ("در انتظار", "[مبلغ]", "تومان"))) + '</div>'
    terms = card(t, ctitle(t, "شرایط", "ادمین می‌تواند درصدها را تغییر دهد؛ عددها از سرور می‌آیند") +
                 "".join(f'<div style="display:flex;align-items:center;gap:12px;padding:10px 0;border-bottom:1px solid {t["hair"]}"><span style="width:36px;height:36px;border-radius:11px;background:{t["soft"]};color:{t["link"]};display:flex;align-items:center;justify-content:center">{ic(i, 18)}</span><span style="flex-grow:1;font-size:14px;font-weight:700">{a}</span><span style="font-size:14px;font-weight:800">{b}</span></div>'
                         for i, a, b in (("gift", "هدیه‌ی عضویت دوستت", "[مبلغ] تومان"), ("bag", "از اولین خرید", "[٪]"), ("refresh", "از خریدهای بعدی", "[٪]"))), pad=18)
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">{header("دعوت از دوستان", "حساب ← دعوت از دوستان")}{hero}{stats}{terms}</div>')
    return page("دعوت از دوستان — دسکتاپ", shell(t, "حساب", main), t=t)

# ============================================================================
def report_page():
    t = LIGHT
    tech = ("GeekVPN 1.1.4 (windows x86_64) · Windows 11 23H2\n"
            "network: wifi · mode: tun · route: smart · core: xray 25.9 + sing-box 1.12\n"
            "config: vless / ws / tls / 443 · service: direct\n"
            "last failure: core exited (1): failed to dial &lt;HOST&gt;:443: i/o timeout\n"
            "sing-box: [warn] outbound/socks[proxy]: dial &lt;IP&gt;:10808 …")
    form = card(t, ctitle(t, "چه اتفاقی افتاد؟", "جواب را در «تیکت‌های من» و ربات تلگرام می‌گیری") +
                f'<label for="rd" style="position:absolute;width:1px;height:1px;overflow:hidden">شرح مشکل</label><textarea id="rd" style="height:130px;resize:none;border-radius:14px;border:1px solid {t["chipBorder"]};background:{t["chip"]};padding:12px;font-family:inherit;font-size:14px;line-height:1.9;box-sizing:border-box;color:{t["text"]}">از صبح با حالت TUN وصل می‌شود ولی هیچ سایتی باز نمی‌شود.</textarea>'
                f'<div style="display:flex;gap:8px;border-radius:14px;background:{t["soft"]};color:{t["text2"]};padding:10px 12px;font-size:12px;line-height:1.8">{ic("lock", 18, 2, t["link"])}<span>گزارش شامل نسخه‌ی برنامه و سیستم‌عامل، نوع شبکه، حالت، نوع کانفیگ و آخرین خطای اتصال است. آدرس سرورها، UUID، IPها و لینک‌ها حذف می‌شوند.</span></div>'
                f'<div style="display:flex;gap:10px">{btn(t, "ارسال به پشتیبانی", "send", "navy", 48, True)}{btn(t, "کپی گزارش", "copy", "soft", 48)}</div>', extra="flex-grow:1;flex-basis:0", pad=20)
    techc = card(t, ctitle(t, "گزارش فنی", "همان چیزی که فرستاده می‌شود", btn(t, "پنهان کردن", "eye", "soft", 36)) +
                 f'<pre dir="ltr" style="margin:0;flex-grow:1;border-radius:14px;background:#062845;color:#CFE8FF;padding:14px;{FONT};font-size:12px;line-height:1.9;white-space:pre-wrap;text-align:left">{tech}</pre>',
                 extra="flex-grow:1;flex-basis:0", pad=20)
    main = (f'<div style="flex-grow:1;display:flex;flex-direction:column;gap:14px;min-width:0">{header("گزارش مشکل", "پشتیبانی ← گزارش مشکل اتصال")}<div style="flex-grow:1;display:flex;gap:16px;min-height:0">{form}{techc}</div></div>')
    return page("گزارش مشکل — دسکتاپ", shell(t, "پشتیبانی", main), t=t)

# ============================================================================
def update_page():
    t = LIGHT
    base = shell(t, "حساب", home_left(t, "on") + servers_panel(t))
    notes = "".join(f'<li style="margin:0 0 6px">{n}</li>' for n in ("تونل تفکیکی برنامه‌ها روی مک", "اسکنر سریع‌تر روی شبکه‌های موبایل", "رفع قطعی Kill Switch بعد از خواب سیستم"))
    dlg = (f'<div role="dialog" aria-modal="true" aria-label="به‌روزرسانی برنامه" style="position:absolute;top:170px;right:50%;margin-right:-240px;width:480px;border-radius:28px;{t["milk"]};padding:24px;display:flex;flex-direction:column;gap:14px">'
           f'<div style="display:flex;align-items:center;gap:12px"><span style="width:52px;height:52px;border-radius:16px;background:#00ACFE;display:flex;align-items:center;justify-content:center"><img src="{LOGO}" alt="" style="width:34px;height:34px"></span>'
           f'<span style="flex-grow:1;display:flex;flex-direction:column"><span style="font-size:19px;font-weight:800">نسخه‌ی ۱٫۲٫۰ آماده است</span><span style="font-size:12px;color:{t["text2"]}">نسخه‌ی فعلی ۱٫۱٫۴ · ۱۴٫۲ مگابایت · امضاشده</span></span>{iconbtn(t, "x", "بستن")}</div>'
           f'<span style="font-size:13px;font-weight:800">تغییرات</span><ul style="margin:0;padding-right:18px;font-size:13px;line-height:1.9;color:{t["text2"]}">{notes}</ul>'
           f'<div style="display:flex;flex-direction:column;gap:6px"><span style="display:flex;justify-content:space-between;font-size:12px;color:{t["text2"]}"><span>در حال دانلود از mirror داخلی…</span><span>۶۴٪</span></span><span style="height:8px;border-radius:4px;background:{t["track"]};overflow:hidden"><span style="display:block;width:64%;height:100%;background:{t["link"]}"></span></span></div>'
           f'<div style="display:flex;gap:10px">{btn(t, "نصب و راه‌اندازی دوباره", "dl", "navy", 48, True)}{btn(t, "بعداً", None, "soft", 48)}</div>'
           f'<span style="font-size:11px;line-height:1.8;color:{t["text2"]}">امضای فایل قبل از نصب بررسی می‌شود. اتصال VPN حین نصب قطع و بعد دوباره وصل می‌شود.</span></div>')
    return page("به‌روزرسانی — دسکتاپ", base + dialog_scrim(t) + dlg, t=t)

# ============================================================================
def tray_flyout():
    t = LIGHT
    w, h = 360, 560
    favs = "".join(
        f'<button type="button" aria-pressed="{"true" if s else "false"}" style="display:flex;align-items:center;gap:10px;padding:6px 8px;border-radius:13px;{"background:#EAF5FD" if s else "background:none"};border:none;font-family:inherit;color:#062845;cursor:pointer;text-align:right">'
        f'{cc(t, c, s, 32)}<span style="flex-grow:1;font-size:13px;font-weight:700">{n}</span>{ping(t, ms)}</button>'
        for c, n, ms, s in (("DE", "آلمان · کلاد", 121, True), ("TR", "ترکیه · کلاد", 96, False), ("US", "آمریکا · کلاد", 142, False)))
    body = (backdrop(t, w, h, cx=300, cy=90) +
            f'<div style="position:absolute;inset:14px;display:flex;flex-direction:column;gap:12px">'
            f'<div style="display:flex;align-items:center;gap:10px"><img src="{LOGO}" alt="" style="width:30px;height:30px"><span dir="ltr" style="flex-grow:1;{FONT};font-size:17px;font-weight:700;text-align:right">GeekVPN</span>'
            f'<button type="button" aria-label="باز کردن برنامه" style="width:36px;height:36px;border-radius:12px;{t["clear"]};display:flex;align-items:center;justify-content:center;cursor:pointer">{ic("monitor", 17, 2, "#FFFFFF")}</button>'
            f'<button type="button" aria-label="تنظیمات" style="width:36px;height:36px;border-radius:12px;{t["clear"]};display:flex;align-items:center;justify-content:center;cursor:pointer">{ic("gear", 17, 2, "#FFFFFF")}</button></div>'
            f'<div style="display:flex;align-items:center;gap:16px;padding:4px 2px"><div style="position:relative;width:118px;height:118px;flex-shrink:0;display:flex;align-items:center;justify-content:center">'
            f'<svg aria-hidden="true" width="118" height="118" viewBox="0 0 118 118" style="position:absolute;inset:0"><circle cx="59" cy="59" r="55" fill="none" stroke="#00ACFE" stroke-width="4"/></svg>'
            f'<button type="button" aria-label="قطع اتصال" style="width:100px;height:100px;border-radius:50%;{t["milk"]};display:flex;align-items:center;justify-content:center;cursor:pointer"><img src="{GLASSES}" alt="" style="width:56px;height:56px;object-fit:contain"></button></div>'
            f'<div style="display:flex;flex-direction:column;gap:6px"><span style="display:flex;align-items:center;gap:8px;font-size:14px;font-weight:700"><span style="width:8px;height:8px;border-radius:2px;background:#7DFFCB"></span>متصل و امن</span>'
            f'<span dir="ltr" style="{FONT};font-size:30px;font-weight:700;line-height:1;text-align:right">00:12:48</span>'
            f'<span dir="ltr" style="{FONT};font-size:12px;opacity:0.9;text-align:right">↓ 2.48 MB/s · ↑ 312 KB/s</span></div></div>'
            f'<section style="border-radius:20px;{t["milk"]};padding:12px;display:flex;flex-direction:column;gap:10px">'
            f'<span style="font-size:12px;font-weight:800;color:#3F5F7E">مسیر ترافیک</span>{segmented(t, ["هوشمند", "سراسری", "مستقیم"], 0, 38)}'
            f'<div style="display:flex;align-items:center;gap:8px"><span style="flex-grow:1;font-size:12px;font-weight:800;color:#3F5F7E">سرورهای ستاره‌دار</span><span style="font-size:11px;color:#3F5F7E">خودکار</span>{switch(t, False, "انتخاب خودکار")}</div>{favs}</section>'
            f'<div style="display:flex;gap:8px">{btn(t, "TUN", "shield", "white", 40, True)}{btn(t, "Kill Switch", "wall", "white", 40, True)}</div>'
            f'<div style="display:flex;gap:8px;margin-top:auto"><button type="button" style="flex-grow:1;height:40px;border-radius:13px;{t["clear"]};font-family:inherit;font-size:13px;font-weight:700;cursor:pointer">باز کردن GeekVPN</button><button type="button" style="height:40px;padding:0 14px;border-radius:13px;border:none;background:rgba(217,63,72,0.9);color:#FFFFFF;font-family:inherit;font-size:13px;font-weight:700;cursor:pointer;display:flex;align-items:center;gap:6px">{ic("power", 16, 2.2, "#FFFFFF")}خروج</button></div></div>')
    return page("پنل کوچک کنار ساعت", body.replace('<div dir="rtl"', '<div dir="rtl"'), w=w, h=h, t=t).replace(
        f'width:{w}px;height:{h}px;position:relative;overflow:hidden', f'width:{w}px;height:{h}px;position:relative;overflow:hidden;border-radius:22px')

def tray_icons():
    t = LIGHT
    w, h = 360, 250
    def icon(bg, fg, label, sub):
        return (f'<div style="flex-grow:1;flex-basis:0;display:flex;flex-direction:column;align-items:center;gap:8px">'
                f'<span style="width:64px;height:64px;border-radius:18px;background:#1F2937;display:flex;align-items:center;justify-content:center"><span style="width:32px;height:32px;border-radius:9px;background:{bg};display:flex;align-items:center;justify-content:center"><svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="{fg}" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="7.5" cy="13" r="3.5"/><circle cx="16.5" cy="13" r="3.5"/><path d="M11 13h2"/></svg></span></span>'
                f'<span style="font-size:13px;font-weight:800">{label}</span><span style="font-size:11px;opacity:0.85;text-align:center">{sub}</span></div>')
    body = (backdrop(t, w, h, cx=300, cy=40) + f'<div style="position:absolute;inset:18px;display:flex;flex-direction:column;gap:16px"><span style="font-size:15px;font-weight:800">آیکن کنار ساعت</span>'
            f'<div style="display:flex;gap:10px">{icon("#00ACFE", "#062845", "متصل", "آبی")}{icon("#8A9BB0", "#FFFFFF", "قطع", "خاکستری")}{icon("#F59E0B", "#062845", "در حال تلاش", "نارنجی · اسکن و اتصال")}</div>'
            f'<span style="font-size:11px;line-height:1.8;opacity:0.9">کلیک چپ: پنل کوچک · کلیک راست: منوی بومی سیستم‌عامل</span></div>')
    return page("آیکن‌های tray", body, w=w, h=h, t=t)

# ============================================================================
BOARDS = {
    "Desktop-Login.dc.html": (login, "دسکتاپ — ورود"),
    "Desktop-Login-Wait.dc.html": (login_wait, "دسکتاپ — منتظر تأیید تلگرام"),
    "Desktop-Home-Connecting.dc.html": (home_connecting, "دسکتاپ — اتصال هوشمند"),
    "Desktop-Home-Dark.dc.html": (home_dark, "دسکتاپ — خانه (تیره)"),
    "Desktop-Servers.dc.html": (servers_page, "دسکتاپ — سرورها"),
    "Desktop-Services.dc.html": (services_page, "دسکتاپ — سرویس‌ها"),
    "Desktop-Shop.dc.html": (shop_page, "دسکتاپ — فروشگاه"),
    "Desktop-Account.dc.html": (account_page, "دسکتاپ — حساب"),
    "Desktop-Settings.dc.html": (settings_page, "دسکتاپ — تنظیمات"),
    "Desktop-Split.dc.html": (split_page, "دسکتاپ — تونل تفکیکی"),
    "Desktop-Tools.dc.html": (tools_page, "دسکتاپ — ابزارها"),
    "Desktop-Usage.dc.html": (usage_page, "دسکتاپ — مصرف روزانه"),
    "Desktop-Support.dc.html": (support_page, "دسکتاپ — پشتیبانی"),
    "Desktop-Referral.dc.html": (referral_page, "دسکتاپ — دعوت از دوستان"),
    "Desktop-Report.dc.html": (report_page, "دسکتاپ — گزارش مشکل"),
    "Desktop-Update.dc.html": (update_page, "دسکتاپ — به‌روزرسانی"),
    "Desktop-Tray.dc.html": (tray_flyout, "دسکتاپ — پنل کوچک tray"),
    "Desktop-TrayIcons.dc.html": (tray_icons, "دسکتاپ — آیکن‌های tray"),
}

if __name__ == "__main__":
    for name, (fn, _) in BOARDS.items():
        write(name, fn())
    json.dump({k: v[1] for k, v in BOARDS.items()}, open(OUT / "titles.json", "w"), ensure_ascii=False)
    print("ok", len(BOARDS))
