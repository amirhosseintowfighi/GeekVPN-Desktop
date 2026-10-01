# GeekVPN Desktop

کلاینت دسکتاپ GeekVPN برای Windows، macOS و Linux (Tauri v2 + React + TypeScript).

- معماری و تصمیم‌ها: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- بردهای طراحی دسکتاپ: [`design/desktop/`](design/desktop/) (تولیدشده با `gen.py`)

## هسته‌ی اتصال

قبل از اولین `pnpm dev` یا `pnpm build`:

```bash
scripts/build-geekcore.sh   # Go 1.27 به بعد (GOTOOLCHAIN=auto خودش می‌گیرد)
scripts/build-singbox.sh    # sing-box برای حالت TUN (نسخه‌ی پین‌شده)
scripts/build-helper.sh     # geekvpn-helper، سرویس TUN و Kill Switch
scripts/fetch-geo.sh        # geoip.dat و geosite.dat
```

## اجرا

پیش‌نیازها: Node 22 و pnpm 10، Rust stable، و روی لینوکس
`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev`.

```bash
pnpm install
pnpm dev          # اپ در حالت توسعه
pnpm check        # typecheck
pnpm test         # تست‌های UI
pnpm build        # بسته‌ی نصبی سیستم‌عامل فعلی
```

`pnpm -C apps/desktop dev` فقط UI را در مرورگر روی `localhost:1420` بالا می‌آورد.

## سرویس GeekVPN (حالت TUN)

حالت TUN و Kill Switch به `geekvpn-helper` نیاز دارند که با دسترسی root یا SYSTEM اجرا می‌شود. بسته‌ی `.deb` و `.rpm` آن را خودشان نصب می‌کنند. در بقیه‌ی حالت‌ها دکمه‌ی «نصب سرویس» در تنظیمات این کار را می‌کند. دستی:

```bash
sudo target/release/geekvpn-helper install     # یا uninstall
sudo geekvpn-helper run --console               # بدون سرویس، برای دیباگ
```

## آدرس بک‌اند

در زمان build از متغیرهای محیطی خوانده می‌شود و هیچ آدرس واقعی در ریپو نیست (مثل اپ اندروید):

| متغیر | کاربرد |
|---|---|
| `GEEK_ENV` | `prod` یا `staging`؛ پیش‌فرض: release ← prod، debug ← staging |
| `GEEK_API_BASE_PROD` / `GEEK_API_BASE_STAGING` | آدرس `https://` بک‌اند. `http://127.0.0.1` فقط در build دیباگ پذیرفته می‌شود |
| `GEEK_BOT_USERNAME` | نام کاربری ربات (بدون @) برای لینک دعوت و «ربات پشتیبانی»؛ خالی = این دو پنهان می‌شوند |
| `GEEK_UPDATER_PUBKEY` | کلید عمومی امضای به‌روزرسانی (خروجی `pnpm tauri signer generate`)؛ خالی = به‌روزرسانی خودکار خاموش |
| `GEEK_RELEASE_REPO` | `owner/repo` همین ریپو، برای fallback به `latest.json` خود GitHub وقتی بک‌اند در دسترس نیست |

## انتشار نسخه

با push یک tag به شکل `v1.2.0` (یا `v1.2.0-beta.1` برای prerelease)، workflow ‏`release.yml` برای ویندوز (NSIS)، مک (universal، `.app` و `.dmg`) و لینوکس (`.deb`، `.rpm`، AppImage) build می‌گیرد و یک release پیش‌نویس در GitHub می‌سازد. نسخه از خود tag خوانده می‌شود.

**یک بار، قبل از اولین انتشار:**

1. کلید امضای به‌روزرسانی را بساز و جای امن نگه دار. گم شدنش یعنی نسخه‌های نصب‌شده دیگر به‌روز نمی‌شوند:
   ```bash
   pnpm -C apps/desktop tauri signer generate -w geekvpn-updater.key
   ```
2. در Settings ← Secrets and variables ← Actions:

   | نوع | نام | مقدار |
   |---|---|---|
   | Secret | `GEEK_API_BASE_PROD` | آدرس `https://` بک‌اند (لازم) |
   | Secret | `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | محتوای `geekvpn-updater.key` و رمزش |
   | Variable | `GEEK_UPDATER_PUBKEY` | محتوای `geekvpn-updater.key.pub` |
   | Variable | `GEEK_BOT_USERNAME` | نام ربات |
   | Secret | `WINDOWS_CERTIFICATE` / `WINDOWS_CERTIFICATE_PASSWORD` | گواهی Authenticode به‌صورت `.pfx`، base64 |
   | Secret | `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD` / `APPLE_SIGNING_IDENTITY` | گواهی Developer ID Application به‌صورت `.p12`، base64 |
   | Secret | `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` | notarize (رمز app-specific) |

   هر کدام که نباشد، همان بخش انجام نمی‌شود و workflow هشدار می‌دهد: بدون کلید به‌روزرسانی `latest.json` ساخته نمی‌شود و بدون گواهی‌ها نصب‌کننده امضا ندارد.
3. در GeekVPNBot: `APP_RELEASE__DESKTOP_GITHUB_REPO` را `owner/repo` همین ریپو بگذار. `APP_RELEASE__MIRROR_BASE_URL` (همان mirror اپ اندروید) اگر فایل‌های release را آنجا هم کپی کنی، دانلود از mirror انجام می‌شود. `APP_RELEASE__DESKTOP_MIN_VERSION` نسخه‌های قدیمی‌تر را مجبور به به‌روزرسانی می‌کند.

**هر انتشار:** tag را push کن، release پیش‌نویس را با یادداشت تغییرات کامل کن (همین متن در پنجره‌ی به‌روزرسانی برنامه نشان داده می‌شود) و Publish بزن. اگر mirror داری، فایل‌های release را با همان اسم‌ها آنجا کپی کن.

## تست end-to-end با بک‌اند واقعی

`crates/geek-api/tests/e2e.rs` کل مسیر ورود را روی یک GeekVPNBot در حال اجرا تست می‌کند (ورود، تأیید ربات، poll، `me`، چرخش refresh token، تشخیص استفاده‌ی دوباره، خروج):

```bash
GEEK_E2E_API=http://127.0.0.1:8765/ \
GEEK_E2E_APPROVER='cd ../geekvpnbot && .venv/bin/python ../GeekVPN-Desktop/scripts/e2e/approve.py {link}' \
cargo test -p geek-api --test e2e -- --ignored
```

`GEEK_E2E_APPROVER` نقش ربات را بازی می‌کند: کد لینک را در دیتابیس بک‌اند claim و تأیید می‌کند، همان کاری که `handlers/app_login.py` انجام می‌دهد.

## لایسنس

GPL-3.0، مثل اپ اندروید GeekVPN.
