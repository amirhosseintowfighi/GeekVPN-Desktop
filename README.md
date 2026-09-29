# GeekVPN Desktop

کلاینت دسکتاپ GeekVPN برای Windows، macOS و Linux (Tauri v2 + React + TypeScript).

- معماری و تصمیم‌ها: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- بردهای طراحی دسکتاپ: [`design/desktop/`](design/desktop/) (تولیدشده با `gen.py`)

## اجرا

پیش‌نیازها: Node 22 و pnpm 10، Rust stable، و روی لینوکس
`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`.

```bash
pnpm install
pnpm dev          # اپ در حالت توسعه
pnpm check        # typecheck
pnpm test         # تست‌های UI
pnpm build        # بسته‌ی نصبی سیستم‌عامل فعلی
```

`pnpm -C apps/desktop dev` فقط UI را در مرورگر روی `localhost:1420` بالا می‌آورد.

## لایسنس

GPL-3.0، مثل اپ اندروید GeekVPN.
