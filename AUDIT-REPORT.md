# گزارش بررسی کامل پروژه GeekVPN Desktop

> **تاریخ:** ۱۴۰۴/۰۷/۱۲ (2026-10-04)  
> **ریپو:** `amirhosseintowfighi/GeekVPN-Desktop` — شاخه `claude/laughing-johnson-x4m41g`  
> **استک:** Tauri v2 + React 19 + TypeScript + Rust + Go (Xray-core + cfscan + sing-box)  
> **حجم کد:** ~۱۰۷ فایل — ~۷۳۵KB سورس — ۷ کرت Rust + یک ماژول Go + یک اپ Tauri

---

## فهرست

1. [خلاصه اجرایی](#1-خلاصه-اجرایی)
2. [طراحی و فرانت‌اند](#2-طراحی-و-فرانت-اند)
3. [بک‌اند و هسته اتصال](#3-بک-اند-و-هسته-اتصال)
4. [معماری و زیرساخت](#4-معماری-و-زیرساخت)
5. [کارایی و بنچمارک](#5-کارایی-و-بنچمارک)
6. [امنیت](#6-امنیت)
7. [نقاط قوت](#7-نقاط-قوت-خلاصه)
8. [ایرادات و ریسک‌ها](#8-ایرادات-و-ریسکها-خلاصه)
9. [پیشنهادات اولویت‌دار](#9-پیشنهادات-اولویتدار-roadmap)
10. [امتیازدهی نهایی](#10-امتیازدهی-نهایی)

---

## ۱. خلاصه اجرایی

پروژه GeekVPN Desktop یک کلاینت VPN دسکتاپ برای ویندوز، مک و لینوکس است که با **Tauri v2 + React** ساخته شده و از نظر معماری در سطح حرفه‌ای و قابل مقایسه با کلاینت‌های مطرحی مثل Mullvad است.

بررسی کامل پروژه از سه زاویه طراحی، بک‌اند و زیرساخت نشان می‌دهد که:

- **معماری کلی عالی است** — جداسازی `helper` با دسترسی بالا از UI عادی، پروتکل IPC نسخه‌دار، و استفاده از دو موتور (Xray + sing-box) تصمیم‌های درستی هستند.
- **کد Rust بسیار تمیز و سخت‌گیرانه نوشته شده** — تایپ‌سیف، تست‌شده، با مدیریت توکن پیشرفته.
- **طراحی UI منسجم و وفادار به بردهای طراحی است** — سیستم توکن‌محور، فارسی‌سازی عمیق، و a11y فراتر از حد معمول.
- **ضعف‌های اصلی در حواشی هستند** — ریسپانسیو صفر، پوشش تست UI کم، نبود هش برای فایل‌های geo، و updater غیرفعال.

> **جمع‌بندی یک‌خطی:** پروژه از نظر هسته و معماری **در حد تولید حرفه‌ای** است؛ ۴ تا ۵ PR کوچک آن را کاملاً production-ready می‌کند.

---

## ۲. طراحی و فرانت‌اند

### ۲.۱ ساختار کد

| بخش | وضعیت |
|-----|--------|
| تفکیک `lib` / `design-system` / `pages` / `shell` | ✅ عالی — منطق خالص از کامپوننت جدا |
| سخت‌گیری TypeScript (`strict` + `noUncheckedIndexedAccess` + ...) | ✅ بسیار سخت‌گیر |
| روتینگ (`createHashRouter` برای Tauri) | ✅ درست |
| استیت (`AuthContext` + `TunnelContext` با `alive` flag) | ✅ تمیز، race-safe |
| Code splitting (`React.lazy`) | ❌ ندارد — همه صفحات ایستا import شده‌اند |
| ErrorBoundary سراسری | ❌ ندارد |

### ۲.۲ دیزاین سیستم

| مورد | وضعیت | توضیح |
|------|--------|-------|
| توکن‌ها (`tokens.css` — Sky Lens) | ✅ | ۵۰+ متغیر، light/dark از روی `DarkGeekColors` اندروید |
| کامپوننت‌ها (`Button`, `Switch`, `Card`, `Row`...) | ✅ | ۶ نوع دکمه، a11y کامل (`role`, `aria-*`) |
| آیکون‌ها (۶۰ آیکون stroke) | ✅ | single source of truth با `gen.py` |
| `glass-milk / glass-clear` (blur 24px) | ✅ | زیبا ولی روی GPU ضعیف ویندوز ممکن است لگ دهد |
| فونت (Vazirmatn + Space Grotesk) | ✅ | فارسی‌سازی کامل اعداد |
| پوشش تست کامپوننت | ⚠️ | فقط ۳ تست (Switch, Segmented, PingBars) |

### ۲.۳ صفحات و UX

| صفحه | وضعیت | نکته |
|------|--------|------|
| `Home` (قهرمان پروژه — دکمه عینکی، گراف، آمار) | ✅ عالی | ۴ حالت `off/connecting/on/failed` با پیام فارسی دقیق |
| `Login` (QR + تلگرام + رمز) | ✅ | ۳ حالت + تایمر + QR ۱۹۶px |
| `ServerList` (فیلتر، سرچ، پنل راست) | ✅ | AutoSelect + OptimizerCard |
| `Settings` / `Split` / `Tools` / ... | ✅ | همگی `PageHeader + Card/Group/Row` منسجم |
| `EmptyState` در صفحات خالی | ✅ | همه‌جا یک CTA مشخص |

### ۲.۴ Shell (قاب پنجره)

| بخش | وضعیت | توضیح |
|-----|--------|-------|
| `TitleBar` (frameless، drag region، traffic lights مک) | ✅ | `pin/min/max/close` با `opacity-85` |
| `Sidebar` (راست، ۹۲px، glass-clear) | ✅ | بج `unread` با `faDigits` |
| `Backdrop` (glow + رینگ‌ها) | ✅ | مطابق `Foundations` |
| Responsive | ❌ | `absolute` + `px` ثابت، زیر ۱۱۰۰px می‌شکند |

### ۲.۵ مشکلات طراحی

| # | مشکل | شدت | راه حل |
|---|------|-----|--------|
| 1 | هیچ responsive وجود ندارد | 🔴 | `@media (max-width:1200px)` برای collapsed پنل |
| 2 | بدون `ErrorBoundary` — کرش یک صفحه کل اپ را سفید می‌کند | 🟠 | wrapper دور `Outlet` |
| 3 | بدون `React.lazy` — باندل اولیه سنگین (۱۵ صفحه) | 🟠 | `lazy(() => import(...))` + `Suspense` |
| 4 | `backdrop-filter: blur(24px)` روی GPU ضعیف لگ | 🟡 | `@supports` fallback مات |
| 5 | FOUC تم دارک (فلش روشن قبل از دارک) | 🟡 | اسکریپت inline در `index.html` |
| 6 | `Escape` در `UpdateDialog` هندل نشده | 🟡 | `FocusTrap` یکسان |
| 7 | پوشش تست React فقط ۳ کامپوننت | 🟡 | تست `Home` (۴ استیت) + `Shop` + `Support` |

---

## ۳. بک‌اند و هسته اتصال

### ۳.۱ ساختار کرت‌ها

| کرت | نقش | کیفیت |
|-----|-----|--------|
| `geek-api` | کلاینت تایپ‌شده بک‌اند GeekVPNBot | ✅ عالی — `Session` با single-flight refresh |
| `geek-secrets` | توکن در Keychain + `device_id` | ✅ امن — هیچ فایل متنی |
| `geek-config` | پارس لینک + تولید کانفیگ Xray/sing-box | ✅ pure، تست‌شده (۲۰۰+ خط تست) |
| `geek-core` | سوپروایزر `geekcore` (Go/Xray) | ✅ timeout دار، tail لاگ |
| `geek-netplat` | پروکسی سیستمی + Snapshot | ✅ trait انتزاعی، بدون `cfg` در منطق مشترک |
| `geek-ipc` | پروتکل IPC به helper | ✅ نسخه‌دار (`PROTOCOL=1`) |
| `geek-helper` | دیمن root برای TUN + Kill Switch | ✅ WFP/pf/nftables |

### ۳.۲ هسته Go (`core/geekcore`)

| مورد | وضعیت | توضیح |
|------|--------|-------|
| پروتکل JSON خط‌به‌خط روی stdin/stdout | ✅ | stdout به stderr منتقل تا با لاگ Xray قاطی نشود |
| همه پروتکل‌های Xray (`distro/all`) | ✅ | تطابق با اندروید |
| `measure` با double-GET روی `generate_204` | ✅ | انتخاب دقیق سریع‌ترین سرور |
| `testDelays` موازی (sem=8) | ✅ | ولی `results[i]` بدون Mutex — با `go vet -race` flag می‌شود |
| `Double-Timeout` در `measure` | ⚠️ | `context 12s` + `http.Client 12s` + `TLS 6s` — خطای مبهم |

### ۳.۳ موتور اتصال (`tunnel.rs` — ۶۶۳ خط)

| مورد | وضعیت |
|------|--------|
| ماشین حالت ۴ حالته (`Off/Connecting/On/Failed`) | ✅ |
| ۳ کاندید اول + `free_port()` + SOCKS token تصادفی | ✅ |
| `IranRule` از geo با `OnceCell + spawn_blocking` | ✅ |
| Snapshot پروکسی قبل از apply (روی دیسک) | ✅ — `recover()` بعد کرش |
| `watch()` با تیک ۱ ثانیه‌ای + `monitor()` failover ۳۰ ثانیه‌ای | ✅ |
| `blocking: bool` در `Failed` + پیام فارسی Kill Switch | ✅ کاربرمحور |
| خاموش کردن خطای `release_system_proxy` بدون لاگ | ⚠️ دیباگ سخت |
| `token()` بدون `secrecy` crate | 🟡 |

### ۳.۴ مدیریت توکن و احراز هویت

| مورد | وضعیت | توضیح |
|------|--------|-------|
| `vault.rs` با `keyring` native | ✅ عالی | Credential Manager / Keychain / Secret Service |
| `device.rs` با `HMAC-SHA256(machine-id)` | ✅ ایده درست | ولی `DEVICE_ID_KEY` هاردکد در باینری قابل استخراج است |
| `Session::token()` با `Mutex` روی `await refresh()` | ✅ بهترین پیاده‌سازی دیده‌شده | تست `concurrent_callers_share_one_refresh` |
| `wait_for_approval` با long-poll ۲۵s + backoff + `CancellationToken` | ✅ | لپ‌تاپ هنگام تغییر WiFi approval را از دست نمی‌دهد |
| `fetch_subscription` بدون محدودیت حجم/اسکیم | ⚠️ | SSRF تئوریک — باید `https://` + `MAX_BODY` |

### ۳.۵ Kill Switch

| OS | پیاده‌سازی | وضعیت |
|----|-----------|--------|
| Linux | nftables با `mark=0x2024` | ✅ |
| Windows | WFP با ALE app-id | ✅ |
| macOS | pf با anchor `com.apple/250.GeekVPN` | ⚠️ نمی‌تواند per-process فیلتر کند (محدودیت pf) |

### ۳.۶ کیفیت کد Rust

| معیار | وضعیت |
|------|--------|
| `unsafe` | ✅ فقط ۲ محل (Windows pipe + WinINet)، مستند با SAFETY |
| `unwrap/expect` در پروداکشن | ✅ فقط `lock.expect("tail lock")` و `Hmac::new` (غیرقابل فیل) |
| Error handling (`thiserror` + `Result`) | ✅ `ApiError::user_message()` فارسی دقیق |
| `serde(rename_all="camelCase")` یک‌دست | ✅ تست `tagged_enums...camel_case` |
| تست‌ها | ✅ `geek-config` ۲۰۰+ خط، `geek-api` wiremock، `geek-core` e2e واقعی |
| تست فایروال / failover | ❌ صفر |

---

## ۴. معماری و زیرساخت

### ۴.۱ مستندات

| مورد | وضعیت |
|------|--------|
| `docs/ARCHITECTURE.md` (۴۷۶ خط، فاز ۰ تا ۶) | ✅ بی‌نظیر — از روی کد واقعی نوشته شده |
| دیاگرام پروسه‌ها + جدول هر OS + قرارداد ۱۳ اندپوینت | ✅ |
| ۱۵ تصمیم بسته‌شده با دلیل + یادداشت هر فاز | ✅ شفافیت کمیاب (باگ‌های پیدا شده مستند) |
| هم‌ترازی با اندروید (۱۸ قابلیت) | ✅ |
| تک‌فایلی بودن (بدون `DEV.md` / `SECURITY.md`) | ⚠️ |

### ۴.۲ Workspace و وابستگی‌ها

| مورد | وضعیت |
|------|--------|
| `resolver="2"`, `rust-version=1.88`, `edition=2021` | ✅ |
| `profile.release: lto + codegen=1 + opt-level=s + strip` | ✅ مناسب توزیع (ولی CI کند) |
| `Cargo.lock` + `pnpm-lock.yaml` + `go.sum` کامیت | ✅ |
| `onlyBuiltDependencies: [esbuild]` (محدود postinstall) | ✅ |
| `[workspace.dependencies]` مرکزی ندارد | ⚠️ ریسک ناهماهنگی patch |
| `go.mod` با `cfscan` پرایوت pseudo-version | ⚠️ build بدون دسترسی به ریپوی اندروید غیرقابل تکرار |
| `tauri.conf.json: bundle.linux.deb.depends: []` خالی | ⚠️ کاربر بدون `libwebkit2gtk-4.1` runtime می‌شکند |

### ۴.۳ CI/CD

| بخش | وضعیت |
|-----|--------|
| `ci.yml`: `web` → `app` ماتریس ۳گانه (linux/win/mac) + `fail-fast:false` | ✅ |
| `release.yml`: tag `v*` → build ماتریس + `releaseDraft` + امضا | ✅ |
| `cargo clippy -D warnings` + `cargo test` در CI | ✅ |
| `Swatinem/rust-cache@v2` + `setup-go` cache | ✅ |
| بدون `cargo audit` / `cargo deny` / `pnpm audit` | ❌ |
| بدون SLSA provenance / `cosign` | ⚠️ |
| `actions/*` با تگ شناور (نه SHA پین) | ⚠️ |

### ۴.۴ اسکریپت‌ها

| اسکریپت | وضعیت | نکته |
|---------|--------|------|
| `build-geekcore.sh` | ✅ نمونه عالی | `mktemp + trap`, patch forward, `go test` قبل از build, `-trimpath -buildid=` |
| `build-singbox.sh` | ✅ | پین `v1.14.2`، ولی `grep -v` خطای `go get` را قورت می‌دهد |
| `build-helper.sh` | ✅ | بدون `--locked` (باید locked بسازد) |
| `fetch-geo.sh` | 🔴 ضعیف‌ترین حلقه | فقط چک سایز `>10KB`، بدون `sha256`، `latest` شناور |
| `release-config.mjs` | ✅ | regex سخت‌گیر، نسخه از tag |

### ۴.۵ مدیریت env

| متغیر | اعتبارسنجی در `build.rs` | وضعیت |
|------|--------------------------|--------|
| `GEEK_ENV` (`prod`/`staging`) | panic اگر جز این دو | ✅ |
| `GEEK_API_BASE_PROD/STAGING` | `https://` اجباری، `http://127.0.0.1` فقط debug | ✅ |
| `GEEK_BOT_USERNAME` | `5..32`, `^[A-Za-z0-9_]+$` | ✅ |
| `GEEK_UPDATER_PUBKEY` | `len>40 && base64`، خالی = updater خاموش | ✅ |
| `GEEK_PROBE_URL` | فقط `#[cfg(debug_assertions)]` | ✅ |

---

## ۵. کارایی و بنچمارک

### ۵.۱ پروفایل Release

```toml
[profile.release]
codegen-units = 1
lto = true
opt-level = "s"    # بهینه برای حجم (نه سرعت خام)
panic = "abort"
strip = true
```

| معیار | مقدار / وضعیت |
|-------|--------------|
| حجم باینری Tauri (تخمینی) | ۸–۱۵ MB (با `opt-level=s` + `strip`) |
| رم در حالت idle (تخمینی) | ۶۰–۱۰۰ MB (WebView بومی هر OS) |
| `lto + codegen=1` | حجم کم، ولی زمان CI ۲–۳ برابر |
| گزینه جایگزین | `opt-level="z"` کندتر ولی کوچک‌تر؛ `s` تعادل بهتری دارد |

### ۵.۲ کارایی Runtime

| بخش | تکنیک | وضعیت |
|-----|--------|-------|
| تست تأخیر موازی | `sem=8` + `WaitGroup` + استریم `test.result` | ✅ تا ۸ سرور هم‌زمان |
| ترافیک | تیک ۱ ثانیه‌ای `core.traffic` + `broadcast(1024)` | ✅ |
| Failover | هر ۳۰s (بعد ۲۰s settle) + cooldown ۲→۳۰ دقیقه | ✅ هوشمند |
| `proxy-snapshot.json` | بدون `chmod 0600` | ⚠️ |
| `STDERR_TAIL=60`, `TAIL=40` | حافظه ثابت برای لاگ | ✅ |
| `geek-config` pure بدون I/O | تست واحد سریع، بدون نیاز به FS/NET | ✅ |
| `Tailwind v4 + @tailwindcss/vite` | بیلد CSS سریع | ✅ |
| `backdrop-filter: blur(24px)` | زیبا ولی روی Intel UHD ویندوز فریم‌دراپ | ⚠️ `will-change` یا fallback پیشنهاد می‌شود |

### ۵.۳ بنچمارک پیشنهادی (برای اجرای بعدی)

> در این محیط `cargo` و `pnpm` نصب نبود، پس این بنچمارک‌ها باید روی سیستم توسعه اجرا شوند:

```bash
# ۱. حجم باندل فرانت
pnpm -C apps/desktop exec vite build --mode production
# بررسی dist/assets/*.js — هدف: < 300KB gzipped

# ۲. تست سرعت
cargo test -p geek-config -- --nocapture  # پارس ۱۰۰۰ لینک
cargo test -p geek-api -- --nocapture     # ریت refresh هم‌زمان

# ۳. حجم باینری
cargo build --release && ls -lh target/release/geekvpn-helper

# ۴. Lighthouse (در حالت Tauri dev)
pnpm -C apps/desktop dev  # سپس Chrome DevTools → Lighthouse

# ۵. Clippy + Audit
cargo clippy --workspace --all-targets -- -D warnings
cargo audit
pnpm audit --audit-level high
```

---

## ۶. امنیت

### ۶.۱ نقاط قوت امنیتی

| مورد | توضیح |
|------|--------|
| جداسازی privileged (`helper` root) از UI عادی | هیچ‌وقت WebView با دسترسی بالا اجرا نمی‌شود |
| IPC نسخه‌دار `PROTOCOL=1` + هشدار `HELPER_OUTDATED` | جلوگیری از spoof |
| `TunSpec` به جای کانفیگ خام — helper خودش sing-box config می‌سازد | اصل least privilege |
| SOCKS داخلی با `Uuid::new_v4` تصادفی هر اتصال | برنامه‌های دیگر نمی‌توانند از آن استفاده کنند |
| توکن فقط در Rust، هرگز در WebView | `LoginStatus` فقط `approved/denied/expired` |
| `Session` single-flight refresh (یک `Mutex` async) | جلوگیری از revoke ناشی از race |
| Keychain native (Credential Manager / Keychain / Secret Service) | هیچ فایل متنی برای توکن |
| CSP سخت‌گیرانه `default-src 'self'; connect-src ipc:` + `freezePrototype:true` | ✅ |
| امضای updater با minisign + `requireSignedVersion:true` | بدون کلید، آپدیت خاموش (fail-secure) |

### ۶.۲ ریسک‌های امنیتی

| # | ریسک | شدت | راه حل |
|---|------|-----|--------|
| 1 | `fetch-geo.sh` بدون هش — `geoip.dat` مسموم قابل تزریق | 🔴 بالا | پین tag + `sha256sum -c` |
| 2 | `DEVICE_ID_KEY` هاردکد — HMAC عملاً بی‌کلید | 🟠 متوسط | انتقال به `build.rs` env یا Keychain |
| 3 | `fetch_subscription` بدون محدودیت `https` + `MAX_BODY` | 🟡 کم | whitelist + `Content-Length < 2MB` |
| 4 | `cfscan` پرایوت — build غیرقابل تکرار | 🟡 کم | `go mod vendor` |
| 5 | بدون `cargo audit` در CI | 🟡 کم | افزودن به `ci.yml` |
| 6 | Kill Switch در macOS نمی‌تواند per-process فیلتر کند | 🟡 محدودیت ذاتی pf | مستند شده — اطلاع به کاربر |

---

## ۷. نقاط قوت (خلاصه)

| # | نقطه قوت | حوزه |
|---|----------|------|
| 1 | معماری least-privilege با IPC تایپ‌شده — در حد Mullvad | معماری |
| 2 | مدیریت توکن با single-flight refresh + تست هم‌زمانی | بک‌اند |
| 3 | `geek-config` pure و heavily tested (VLESS/Reality/VMess/SS) | بک‌اند |
| 4 | دیزاین سیستم token-first — ۱:۱ با `gen.py` و اندروید | طراحی |
| 5 | فارسی‌سازی عمیق (`faDigits`, `toman`, `durationLabel` با تست) | طراحی |
| 6 | a11y فراتر از حد معمول (`role`, `aria-*`, `prefers-reduced-motion`) | طراحی |
| 7 | `ARCHITECTURE.md` بی‌نظیر — ۴۷۶ خط از روی کد واقعی | مستندات |
| 8 | اسکریپت‌های build شفاف با `set -euo pipefail` + reproducible | زیرساخت |
| 9 | `build.rs` اعتبارسنجی سخت‌گیرانه env (https اجباری) | امنیت |
| 10 | `keyring` native + CSP سخت‌گیرانه | امنیت |
| 11 | پروکسی snapshot + `recover()` — کرش = اینترنت قطع نمی‌ماند | پایداری |
| 12 | Kill Switch cross-platform با engage دو مرحله‌ای | پایداری |

---

## ۸. ایرادات و ریسک‌ها (خلاصه)

| # | ایراد | شدت | حوزه |
|---|-------|-----|------|
| 1 | `fetch-geo.sh` بدون هش — مهم‌ترین حفره supply chain | 🔴 | امنیت |
| 2 | Responsive صفر — `absolute + px ثابت` زیر ۱۱۰۰px می‌شکند | 🔴 | طراحی |
| 3 | Updater غیرفعال (`pubkey="" endpoints=[]`) در پروداکشن | 🔴 | زیرساخت |
| 4 | `DEVICE_ID_KEY` هاردکد | 🟠 | امنیت |
| 5 | بدون `ErrorBoundary` + بدون `React.lazy` | 🟠 | طراحی |
| 6 | پوشش تست React فقط ۳ کامپوننت | 🟠 | کیفیت |
| 7 | `cfscan` پرایوت — build غیرقابل تکرار | 🟠 | زیرساخت |
| 8 | بدون `cargo audit` / `pnpm audit` در CI | 🟡 | امنیت |
| 9 | `blur(24px)` روی GPU ضعیف ویندوز لگ | 🟡 | طراحی |
| 10 | FOUC تم دارک بدون اسکریپت inline | 🟡 | طراحی |
| 11 | `store.rs` بدون `fsync` — ریسک از دست رفتن بعد قطع برق | 🟡 | بک‌اند |
| 12 | تست فایروال و failover صفر | 🟡 | بک‌اند |
| 13 | `bundle.linux.deb.depends: []` خالی | 🟡 | زیرساخت |

---

## ۹. پیشنهادات اولویت‌دار (Roadmap)

### 🔴 اولویت P0 — قبل از اولین انتشار

| # | کار | فایل | زمان |
|---|-----|------|------|
| 1 | پین هش برای `fetch-geo.sh` — `geo.lock.json` با `url, tag, sha256` + `sha256sum -c` | `scripts/fetch-geo.sh` | ۱ ساعت |
| 2 | پر کردن `updater.pubkey` و تست یک release آزمایشی (و رد بسته دست‌کاری‌شده) | `tauri.conf.json` + Secrets | ۲ ساعت |
| 3 | `go mod vendor` یا `replace` برای `cfscan` + CI بدون دسترسی پرایوت | `core/geekcore/go.mod` | ۱ ساعت |

### 🟠 اولویت P1 — هفته اول

| # | کار | فایل | زمان |
|---|-----|------|------|
| 4 | `React.lazy` برای صفحات + `ErrorBoundary` دور `Outlet` + `Suspense` | `apps/desktop/src/App.tsx` | ۲ ساعت |
| 5 | انتقال `DEVICE_ID_KEY` به `build.rs` env | `crates/geek-secrets/device.rs` | ۱ ساعت |
| 6 | `store.rs: save()` با `fsync` + `MoveFileExW` اتمیک در ویندوز | `crates/.../store.rs` | ۱ ساعت |
| 7 | محدود کردن `fetch_subscription` به `https://` + `MAX_BODY 2MB` | `crates/geek-api/client.rs` | ۳۰ دقیقه |
| 8 | افزودن `cargo audit` + `pnpm audit` به CI | `.github/workflows/ci.yml` | ۳۰ دقیقه |

### 🟡 اولویت P2 — ماه اول

| # | کار | فایل | زمان |
|---|-----|------|------|
| 9 | اسکریپت inline تم در `index.html` برای جلوگیری از FOUC | `apps/desktop/index.html` | ۲۰ دقیقه |
| 10 | `FocusTrap` برای `UpdateDialog` + `Flyout` + هندل `Escape` | `apps/desktop/src/shell/` | ۱ ساعت |
| 11 | `@media (max-width:1200px)` برای `Home` و `Settings` | `apps/desktop/src/pages/*.tsx` | ۲ ساعت |
| 12 | تست‌های `Home` (۴ استیت) + `Shop` + `Support` با `vitest + msw` | `apps/desktop/src/pages/*.test.tsx` | ۴ ساعت |
| 13 | تست فایروال (mock) + تست `FailoverPolicy` با زمان مجازی | `crates/geek-netplat`, `crates/geek-core` | ۳ ساعت |
| 14 | پین `actions/*` روی SHA + `dependabot.yml` | `.github/workflows/*.yml` | ۱ ساعت |

### 🔵 اولویت P3 — بهبود مستمر

| # | کار | زمان |
|---|-----|------|
| 15 | `@supports not (backdrop-filter: blur())` fallback مات | ۳۰ دقیقه |
| 16 | اسکرین‌شات رگرسیون `gen.py` ↔ `playwright` | ۳ ساعت |
| 17 | `docs/DEV.md` + `docs/SECURITY.md` جدا | ۲ ساعت |
| 18 | `justfile` برای `build-geekcore + build-singbox + build-helper + pnpm install` یک‌خطی | ۳۰ دقیقه |
| 19 | SLSA provenance برای release artifacts | ۲ ساعت |
| 20 | `cargo tarpaulin` / `vitest --coverage` در CI | ۱ ساعت |

---

## ۱۰. امتیازدهی نهایی

| محور | امتیاز | توضیح |
|------|:------:|-------|
| 🏗️ معماری و طراحی سیستم | ⭐⭐⭐⭐⭐ | جداسازی helper/TUN/sing-box/geekcore در حد Mullvad |
| 🎨 طراحی و UI/UX | ⭐⭐⭐⭐☆ | منسجم و زیبا، ولی ریسپانسیو صفر |
| ⚙️ بک‌اند و هسته اتصال | ⭐⭐⭐⭐⭐ | تمیزترین کد Rust دیده‌شده، مدیریت توکن حرفه‌ای |
| 🚀 کارایی | ⭐⭐⭐⭐☆ | بهینه برای حجم، تست موازی، ولی blur روی GPU ضعیف |
| 🔒 امنیت | ⭐⭐⭐☆☆ | هسته امن، ولی geo بدون هش + کلید هاردکد |
| 📚 مستندات | ⭐⭐⭐⭐☆ | ARCH بی‌نظیر ولی تک‌فایلی |
| 🔧 DX و اسکریپت‌ها | ⭐⭐⭐⭐☆ | شفاف و reproducible، بدون `just/make` |
| 🔄 CI/CD | ⭐⭐⭐⭐☆ | ماتریس کامل، بدون audit و provenance |
| 🧪 تست | ⭐⭐⭐☆☆ | Rust عالی، React کم |
| **میانگین کل** | **⭐⭐⭐⭐☆ (۴.۰ / ۵)** | **حرفه‌ای — با ۳ اقدام فوری production-ready** |

---

### سه اقدام فوری برای Production-Ready شدن

> ۱. **پین هش geo** — `fetch-geo.sh` را امن کن  
> ۲. **فعال‌سازی updater** — `pubkey` را پر کن و یک release آزمایشی تست کن  
> ۳. ** audit در CI** — `cargo audit` + `pnpm audit` اضافه کن

بعد از این سه، پروژه آماده انتشار نسخه `v1.0.0` است.

---

*این گزارش با بررسی دقیق ~۱۰۷ فایل سورس، ۷ کرت Rust، ماژول Go، تنظیمات Tauri، workflowهای CI/CD، و بردهای طراحی تولید شده است. برای اجرای بنچمارک‌های runtime (حجم باندل، Lighthouse، تست e2e) روی سیستم توسعه با `cargo` و `pnpm` نصب‌شده اقدام کنید.*
