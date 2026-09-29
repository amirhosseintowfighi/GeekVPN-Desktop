# GeekVPN Desktop

کلاینت دسکتاپ GeekVPN برای Windows، macOS و Linux (Tauri v2 + React + TypeScript).

- معماری و تصمیم‌ها: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- بردهای طراحی دسکتاپ: [`design/desktop/`](design/desktop/) (تولیدشده با `gen.py`)

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

## آدرس بک‌اند

در زمان build از متغیرهای محیطی خوانده می‌شود و هیچ آدرس واقعی در ریپو نیست (مثل اپ اندروید):

| متغیر | کاربرد |
|---|---|
| `GEEK_ENV` | `prod` یا `staging`؛ پیش‌فرض: release ← prod، debug ← staging |
| `GEEK_API_BASE_PROD` / `GEEK_API_BASE_STAGING` | آدرس `https://` بک‌اند. `http://127.0.0.1` فقط در build دیباگ پذیرفته می‌شود |

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
