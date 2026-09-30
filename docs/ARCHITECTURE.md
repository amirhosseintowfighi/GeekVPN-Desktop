# GeekVPN Desktop — سند معماری (فاز ۰)

وضعیت: **تأییدشده (فاز ۰)** — تصمیم‌های باز در بخش ۸ بسته شدند. فاز ۱ در حال اجراست.

این سند از روی کد واقعی نوشته شده، نه فرض:

- بک‌اند `amirhosseintowfighi/GeekVPNBot` (روترهای `app_auth.py`، `auth.py`، `miniapp.py`، `app_release.py`)
- اپ اندروید `amirhosseintowfighi/GeekVPN-Android` (`docs/geekvpn.md`، `smartconnect/SmartConnect.kt`، ماژول Go `cfscan/`)
- کانواس طراحی (بردهای `Desktop-Home`، `Desktop-Connections`، `Foundations` و بردهای موبایل)

---

## ۱. تصمیم پشته

| لایه | انتخاب | چرا |
|---|---|---|
| پوسته‌ی اپ | **Tauri v2** (Rust) | باینری ۸–۱۵ مگابایت، رم ~۶۰–۱۰۰ مگابایت، WebView بومی هر OS. پلاگین‌های رسمی برای tray، autostart، deep-link، updater، notification، single-instance. |
| UI | React 19 + TypeScript + Vite + TailwindCSS v4 | طراحی به صورت HTML/CSS است و تقریباً یک‌به‌یک به کامپوننت React تبدیل می‌شود؛ هیچ فریم‌ورک دیگری این وفاداری به پیکسل را با این هزینه نمی‌دهد. |
| هسته‌ی پروتکل | **`geekcore`** — یک باینری Go که Xray-core را به‌صورت کتابخانه لینک می‌کند + `cfscan` + API تست تأخیر | دقیقاً همان کاری که اندروید می‌کند (`AndroidLibXrayLite` + `cfscan` در یک build). XHTTP، REALITY و همه‌ی پروتکل‌هایی که لینک‌های پنل می‌دهند را پوشش می‌دهد. |
| لایه‌ی TUN و مسیریابی | **sing-box** (باینری رسمی، نسخه‌ی پین‌شده) | TUN روی هر سه OS (Wintun / utun / tun)، `auto_route` + `strict_route` ضد نشت DNS، قانون `process_path` برای مسیریابی برنامه‌ای، و Clash API برای صفحه‌ی «اتصالات». Xray هیچ‌کدام از این سه را روی دسکتاپ ندارد. |
| دسترسی root/admin | **`geekvpn-helper`** — دیمن Rust جدا | TUN، فایروال Kill Switch و تغییر پروکسی سیستم در مک به دسترسی بالا نیاز دارند. UI هرگز با دسترسی بالا اجرا نمی‌شود. |

### چرا Flutter Desktop یا Avalonia نه
- **Flutter Desktop:** RTL و فونت فارسی خوب است، ولی tray، پنجره‌ی frameless و flyout در دسکتاپ به پکیج‌های جامعه وابسته‌اند و backdrop blur روی کل پنجره گران است. طراحی HTML/CSS است؛ بازنویسی آن در ویجت‌ها وفاداری ۱۰۰٪ را سخت می‌کند.
- **Avalonia (.NET):** v2rayN روی همین ساخته شده و مرجع خوبی است، ولی runtime دات‌نت ۶۰–۸۰ مگابایت به بسته اضافه می‌کند و سیستم استایل آن از طراحی HTML فاصله دارد.
- **Electron:** رد شد؛ ۱۵۰+ مگابایت و ۳۰۰+ مگابایت رم برای اپی که همیشه در tray است قابل قبول نیست.

### یک نکته‌ی مهم درباره‌ی «Glassmorphism»
در طراحی، کارت‌های شیشه‌ای روی **پس‌زمینه‌ی آبی خود اپ** (`#0870D4` + دایره‌های `#00ACFE`) blur می‌شوند، نه روی دسکتاپ کاربر. پس `backdrop-filter` داخل WebView دقیقاً همان تصویر طراحی را می‌سازد و روی هر سه OS یکسان است. Mica/Acrylic/Vibrancy واقعی OS پنجره را شفاف می‌کند و رنگ آبی طراحی را از بین می‌برد؛ پیشنهاد من استفاده از آن **فقط برای flyout کوچک tray** است (سؤال ۳ پایین). روی لینوکس (WebKitGTK) اگر GPU ضعیف باشد، کارت‌ها به حالت مات `rgba(255,255,255,0.94)` بدون blur برمی‌گردند.

---

## ۲. مدل پروسه‌ها

```
┌─────────────────────────── کاربر عادی ───────────────────────────┐
│  GeekVPN.exe / .app  (Tauri)                                      │
│  ┌─────────────┐  invoke/events  ┌──────────────────────────────┐ │
│  │ WebView     │ ◄─────────────► │ Rust app-core                │ │
│  │ React UI    │  (tauri-specta, │  • API client + refresh      │ │
│  │ (بدون توکن) │   تایپ‌شده)      │  • keyring / device_id       │ │
│  └─────────────┘                 │  • SmartConnect, Failover    │ │
│  پنجره‌ی اصلی + flyout tray       │  • state machine اتصال       │ │
│                                  │  • سوپروایزر geekcore-tools  │ │
│                                  └──────┬───────────────┬───────┘ │
│                   stdio JSON-RPC        │               │ IPC     │
│            ┌────────────────────────────▼───┐           │         │
│            │ geekcore (نقش tools)           │           │         │
│            │  تست real-delay، اسکن cfscan   │           │         │
│            └────────────────────────────────┘           │         │
└─────────────────────────────────────────────────────────┼─────────┘
                    named pipe (ACL) / unix socket (peer cred)
┌─────────────────────────── root / SYSTEM ─────────────────▼───────┐
│  geekvpn-helper  (Windows Service / launchd daemon / systemd)      │
│   • Kill Switch: WFP / pf anchor / nftables                        │
│   • System Proxy (مک)، بازگرداندن پروکسی بعد از کرش                 │
│   • سوپروایزر: sing-box (TUN یا mixed inbound)  ──socks──► geekcore │
│     (نقش tunnel؛ outbound واقعی)                                    │
└────────────────────────────────────────────────────────────────────┘
```

چرا این شکل:

1. **جداسازی کرش (قانون ۵.۳):** تونل در helper زندگی می‌کند. اگر UI کرش کند، تونل و Kill Switch سر جایشان‌اند و UI بعد از باز شدن دوباره وضعیت را از helper می‌خواند. اگر هسته کرش کند، helper کد خروج و ۵۰ خط آخر stderr را به UI می‌فرستد و UI آن را به پیام فارسی ترجمه می‌کند (جدول خطاها در فاز ۳). هر فراخوانی IPC timeout دارد؛ UI هرگز روی هسته block نمی‌شود.
2. **توکن‌ها هرگز وارد WebView نمی‌شوند.** همه‌ی درخواست‌های API از Rust می‌روند. refresh token بک‌اند با هر استفاده عوض می‌شود و استفاده‌ی دوباره را سرقت حساب حساب می‌کند (`sessions.rotate`)؛ پس refresh فقط در یک نقطه و به‌صورت single-flight (یک `Mutex` async) انجام می‌شود — همان درسی که اپ اندروید با «فقط پروسه‌ی اصلی» گرفت.
3. **اسکنر و تست تأخیر بدون دسترسی بالا** اجرا می‌شوند. وقتی TUN روشن است، ترافیک آن‌ها با قانون `process_path` در sing-box مستقیم می‌رود (معادل «خود اپ از VPN مستثناست» در اندروید).

### مسیر ترافیک

| حالت | زنجیره |
|---|---|
| System Proxy | برنامه‌ها → `127.0.0.1:<port>` mixed inbound sing-box → قوانین مسیر → (direct \| block \| socks → geekcore) → سرور |
| TUN | همه‌ی ترافیک → TUN sing-box → قوانین مسیر (+ `process_path`) → (direct \| block \| socks → geekcore) → سرور |

sing-box در هر دو حالت جلوی geekcore است تا **فقط یک پیاده‌سازی مسیریابی** داشته باشیم (قوانین ایران، per-app، block دستی، DNS) و صفحه‌ی «اتصالات» در هر دو حالت کار کند. socks داخلی geekcore روی loopback با نام کاربری/رمز تصادفی هر اجرا قفل می‌شود تا برنامه‌های دیگر نتوانند از آن استفاده کنند. هزینه‌ی hop اضافه روی loopback ناچیز است (v2rayN هم در حالت TUN همین کار را می‌کند).

---

## ۳. TUN، پروکسی سیستم و Kill Switch روی هر OS

### ویندوز
| کار | پیاده‌سازی |
|---|---|
| System Proxy | WinINet: `InternetSetOptionW(INTERNET_OPTION_PER_CONNECTION_OPTION)` + `INTERNET_OPTION_SETTINGS_CHANGED` / `REFRESH`. per-user است، پس از خود اپ انجام می‌شود (بدون admin). مقدار قبلی ذخیره و بعد از قطع برمی‌گردد. bypass: `localhost;127.*;10.*;172.16.*…;192.168.*;<local>`. |
| TUN | sing-box با `wintun.dll` (امضاشده توسط WireGuard LLC، کنار باینری). نیاز به SYSTEM → helper به‌صورت Windows Service (نصب با MSI/NSIS). |
| DNS | `strict_route` در sing-box با WFP جلوی DNS روی کارت‌های دیگر را می‌گیرد. |
| Kill Switch | WFP از helper (`FwpmEngineOpen0`، sublayer اختصاصی، فیلترهای **غیر-dynamic** تا با کرش هسته نمانند ولی با خاموشی helper پاک نشوند): اجازه فقط به loopback، کارت TUN، IP سرورهای فعلی (بعد از override اسکنر)، DHCP/NDP و در صورت انتخاب کاربر LAN. بقیه block. |
| Per-App | `process_path` در sing-box (فقط حالت TUN). |

### macOS (Intel + Apple Silicon، باینری universal)
| کار | پیاده‌سازی |
|---|---|
| System Proxy | `SCPreferences` روی همه‌ی network serviceهای فعال (معادل `networksetup -setwebproxy/-setsecurewebproxy/-setsocksfirewallproxy`)، از helper تا هر بار پسورد نخواهد. |
| TUN | sing-box روی `utun` (کرنل؛ بدون kext و بدون Network Extension). |
| Helper | `SMAppService.daemon` (macOS 13+): کاربر یک بار در System Settings › Login Items تأیید می‌کند. به build امضا و notarize شده نیاز دارد. |
| Kill Switch | `pf` با anchor اختصاصی `com.geekvpn` (`pfctl -a com.geekvpn -f -`)، با توکن `pfctl -E` تا pf کاربر را خاموش نکنیم. |
| Per-App | `process_path` / `process_name` در sing-box (حالت TUN). |

### لینوکس
| کار | پیاده‌سازی |
|---|---|
| System Proxy | GNOME/Cinnamon/Budgie: `gsettings org.gnome.system.proxy`؛ KDE: `kwriteconfig5/6 --file kioslaverc` + سیگنال DBus. بقیه‌ی محیط‌ها API پروکسی سراسری ندارند → پیشنهاد پیش‌فرض لینوکس **TUN** است. |
| TUN | sing-box روی `/dev/net/tun` با `auto_route` (policy routing و `ip rule`)، `auto_redirect` روی nftables. |
| DNS | sing-box با systemd-resolved کنار می‌آید (DNS هایجک روی TUN)؛ اگر resolved نباشد `/etc/resolv.conf` دست نمی‌خورد و هایجک در TUN انجام می‌شود. |
| Helper | سرویس systemd که `.deb`/`.rpm` نصب می‌کنند. برای AppImage: بار اول با `pkexec` helper نصب می‌شود (سؤال ۷). |
| Kill Switch | nftables: جدول `inet geekvpn` با chain خروجی policy drop و همان استثناها. اگر nft نبود، iptables-nft. |
| Per-App | `process_path` در sing-box. |

### انتزاع مشترک (قانون ۵.۴)
همه‌ی این‌ها پشت trait های crate `geek-netplat` هستند؛ منطق مشترک هیچ `cfg(target_os)` ندارد:

```rust
trait SystemProxy { fn apply(&self, p: &ProxySpec) -> Result<Snapshot>; fn restore(&self, s: Snapshot) -> Result<()>; }
trait KillSwitch  { fn engage(&self, allow: &AllowList) -> Result<()>; fn update(&self, allow: &AllowList) -> Result<()>; fn release(&self) -> Result<()>; }
trait NetworkMonitor { fn current(&self) -> NetworkId; fn subscribe(&self) -> Receiver<NetworkEvent>; }
```

### بازیابی بعد از کرش
helper یک فایل state (`/var/lib/geekvpn/state.json`، `%ProgramData%\GeekVPN\state.json`) نگه می‌دارد: پروکسی قبلی، وضعیت Kill Switch، PID‌ها. در شروع، اگر state کثیف باشد: پروکسی قبلی برمی‌گردد، و Kill Switch **فقط اگر کاربر حالت «سخت‌گیر» را زده باشد** فعال می‌ماند؛ وگرنه آزاد می‌شود تا اینترنت کاربر بعد از کرش قفل نماند.

---

## ۴. قرارداد با بک‌اند

همه با Bearer بعد از ورود. آدرس پایه از متغیر build (`GEEK_API_BASE_STAGING` / `GEEK_API_BASE_PROD`، مثل اندروید؛ غیر-`https` بیلد را می‌شکند؛ پیش‌فرض یک host با پسوند `.invalid`).

| کار | اندپوینت | نکته برای دسکتاپ |
|---|---|---|
| شروع ورود | `POST /api/app/auth/link/start` `{device_id, device_name, platform, app_version}` | `platform`: `windows`/`macos`/`linux` (حداکثر ۱۶ کاراکتر، سرور محدودیت enum ندارد). `device_id` ≤ ۶۴. محدودیت: ۵ بار برای هر device، ۲۰ بار برای هر IP. |
| لینک | پاسخ: `deep_link = https://t.me/<bot>?start=applogin_<code>` | اپ آن را به `tg://resolve?domain=<bot>&start=applogin_<code>` تبدیل می‌کند؛ اگر هیچ handler ی برای `tg://` ثبت نشده بود، همان لینک `https` در مرورگر باز می‌شود. |
| انتظار | `POST /api/app/auth/link/poll` `{poll_token, wait:true}` | **سرور هر poll را حداکثر ۲۵ ثانیه نگه می‌دارد**، نه ۵ دقیقه. اپ تا `expires_in` حلقه می‌زند (هدف ۵ دقیقه‌ی سند با `expires_in` سرور محدود می‌شود) و دکمه‌ی انصراف درخواست جاری را abort می‌کند. وضعیت‌ها: `pending/approved/denied/expired`. |
| ورود با رمز | `POST /api/app/auth/password` | در صفحه‌ی ورود اندروید هست؛ پیشنهاد می‌کنم دسکتاپ هم داشته باشد. |
| refresh | `POST /api/v1/auth/refresh` `{refresh_token}` | single-flight. 401 یعنی نشست باطل شده (مثلاً دستگاه از ربات قطع شده) → پاک کردن سرویس‌های حساب و خروج، مثل اندروید. |
| خروج | `POST /api/v1/auth/logout` | |
| سرویس‌ها | `GET /api/miniapp/subscriptions` | `subscription_url`، `tier` (`direct/tunnel/elite`؛ اسکنر فقط برای `direct`)، `used_gib`، `quota_gib`، `expires_at`. اپ لینک اشتراک را خودش دانلود و parse می‌کند. |
| فروشگاه، کیف پول، پرداخت | `/api/miniapp/storefront`, `quote`, `coupon/preview`, `payment-methods`, `checkout/*`, `wallet*`, `payments/*` | بازگشت درگاه: deep link `geekvpn://payment/result` با `tauri-plugin-deep-link`. |
| تست رایگان | `GET/POST /api/miniapp/trial` | |
| ریفرال | `GET /api/miniapp/referral` | |
| تیکت | `GET/POST /api/miniapp/tickets`، `GET/POST /tickets/{id}/messages` | فقط متن؛ ضمیمه API ندارد (پایین). |
| پروفایل | `GET/POST /api/miniapp/profile`، `preferences` | |
| وضعیت سرورها | `GET /api/miniapp/servers` | |
| آپدیت | `GET /api/app/version` | فقط APK برمی‌گرداند (پایین). |

### `device_id` پایدار
`HMAC-SHA256(key = ثابت اپ, msg = machine_id)` به hex = دقیقاً ۶۴ کاراکتر. `machine_id` از `MachineGuid` رجیستری (ویندوز)، `IOPlatformUUID` (مک)، `/etc/machine-id` (لینوکس). شناسه‌ی خام هرگز فرستاده نمی‌شود. `device_name` = hostname کاربر (≤۶۴).

### ذخیره‌ی امن
crate `keyring`: Credential Manager / Keychain / Secret Service (libsecret). فقط refresh token و access token آنجا می‌روند؛ باقی تنظیمات در فایل JSON در پوشه‌ی config کاربر.

### کمبودهای بک‌اند (نیاز به تغییر در GeekVPNBot)
1. **متن ربات «اندروید» را hardcode کرده** (`APP_LOGIN_PROMPT`: «درخواست اتصال از اپلیکیشن اندروید GeekVPN») و `device_name` پیش‌فرض `"Android"` است. باید بر اساس `platform` بنویسد.
2. **تاریخچه‌ی مصرف روزانه/هفتگی/ماهانه اندپوینت ندارد.** فقط `used_gib` لحظه‌ای هست. نیاز: جدول snapshot مصرف (usage_sync هم‌اکنون داده را می‌گیرد) + `GET /api/miniapp/subscriptions/{id}/usage?range=day|week|month`.
3. **ضمیمه‌ی تیکت** API ندارد (`attachment_count` در مدل هست). نیاز: `POST /api/miniapp/tickets/{id}/attachments` (مثل `receipt-photo` که فایل را در ربات برای ادمین می‌فرستد).
4. **`/api/app/version`** فقط APK دارد. پیشنهاد: updater دسکتاپ مستقیم از `latest.json` امضاشده‌ی GitHub Releases بخواند (خروجی `tauri-action`) و بک‌اند فقط `min_version` دسکتاپ را اضافه کند.

---

## ۵. اتصال هوشمند و اسکنر

- **اسکنر:** همان ماژول Go اندروید (`github.com/amirhosseintowfighi/geekvpn-android/cfscan`) بدون بازنویسی، داخل `geekcore` لینک می‌شود. API آن (`Start(configJSON, ranges, listener)`، `Stop()`) به JSON-RPC روی stdio تبدیل می‌شود: `scan.start` → رویدادهای `scan.result`، `scan.progress`، `scan.finish`. `ipv4.txt` در بسته است و از سرور به‌روز می‌شود (سؤال ۹).
- **شناسه‌ی شبکه** (برای ذخیره‌ی IP تمیز به ازای شبکه و تشخیص شبکه‌ی جدید): MAC درگاه پیش‌فرض + DNS suffix از DHCP. SSID عمداً استفاده نمی‌شود: ویندوز 11 24H2 و macOS 14+ برایش مجوز Location می‌خواهند. «شبکه‌ی عمومی» در ویندوز از `INetworkListManager` (دسته‌ی Public) خوانده می‌شود؛ در مک/لینوکس «شبکه‌ی دیده‌نشده» ملاک است.
- **SmartConnect:** پورت مستقیم `SmartConnectUseCase` کاتلین به Rust (`geek-smart`) با همان پورت‌ها (`servers`, `scannable`, `freshIps`, `quickScan`, `useIp`, `measure`, `connect`) و همان تست‌های واحد. سند می‌گوید «۲ تکرار failover»؛ اندروید `MAX_ATTEMPTS = 3` است (یک تلاش + ۲ failover) — همان را نگه می‌دارم مگر بگویی. مراحل در هاب وضعیت: پیدا کردن IP → تست تأخیر → تنظیم فایروال → اتصال (n از m) → تست نهایی.
- **Failover زنده:** `FailoverPolicy` اندروید (دو اندازه‌گیری بد پشت سر هم، cooldown از ۲ تا ۳۰ دقیقه). جابه‌جایی سرور با reload خود geekcore انجام می‌شود؛ TUN پایین نمی‌آید و Kill Switch با allow-list جدید به‌روز می‌شود.
- **تست real-delay موازی:** `geekcore` هر کانفیگ را در یک instance درون‌پروسه‌ای Xray تست می‌کند (مثل `measureOutboundDelay` در libv2ray)، با همروندی قابل تنظیم.
- **Speedtest:** دانلود/آپلود/jitter از طریق proxy محلی، روی endpointی که سؤال ۱۰ مشخص می‌کند.

---

## ۶. نقشه‌ی طراحی → کامپوننت

از بردهای دسکتاپ (۱۲۸۰×۸۰۰):

| عنصر | مشخصات از طراحی |
|---|---|
| پس‌زمینه | `#0870D4`، دو دایره‌ی `rgba(0,172,254,.55/.45)`، حلقه‌های سفید کم‌رنگ، لوگوی ۶٪ opacity |
| TitleBar | ارتفاع ۳۸px، دکمه‌ها `44×34` به ترتیب LTR: سنجاق (always-on-top)، minimize، maximize، close. در مک پیشنهاد: traffic light بومی در چپ (سؤال ۲). |
| Sidebar | راست، `92px`، شیشه‌ی تیره `rgba(255,255,255,.13)` + border `.34` + blur 18، radius 26. آیتم‌ها: خانه، سرویس‌ها، فروشگاه، حساب، سرورها، پروفایل‌ها، درخواست‌ها، اتصالات، گزارش‌ها، ابزارها |
| کارت روشن | `rgba(255,255,255,.88)`، blur 24 saturate 180%، radius 24–26، سایه `0 14px 34px rgba(2,36,84,.28)` |
| Home | دکمه‌ی عینکی اتصال، تایمر Space Grotesk، کارت سرور فعلی، نوار آمار (دانلود، آپلود، IP خروجی، اتصالات فعال، زمان باقی‌مانده)، نوار مصرف ۴۰ قطعه‌ای، پنل ۳۶۰px سرورها (جستجو، chipها، انتخاب خودکار، «بهینه‌ساز کلادفلر»، لیست با میله‌های پینگ) |
| Connections | جدول: مقصد و برنامه، آپلود، دانلود، مسیر (VPN · DE / مستقیم)، زمان، دکمه‌ی مسدود؛ فیلترها همه/VPN/مستقیم/مسدود؛ «بستن همه» — از Clash API sing-box |
| توکن‌ها | `#0870D4` bg، `#00ACFE` آبی لوگو، `#062845` سرمه‌ای، `#EAF5FD` سطح نرم، `#3F5F7E` متن ثانویه، `#0E9F6E` موفق، `#C77700` هشدار، `#D93F48` خطر؛ Vazirmatn 300–800، Space Grotesk 500–700 (لوکال، woff2) |

**بدون طراحی دسکتاپ:** ورود، flyout tray، سرویس‌ها، فروشگاه، حساب، پروفایل‌ها، درخواست‌ها، گزارش‌ها، ابزارها (اسکنر، speedtest)، تنظیمات، ریفرال، تیکت، نمودار مصرف، تم تیره. سؤال ۱ را ببین.

---

## ۷. ساختار دایرکتوری

```
GeekVPN-Desktop/
├─ apps/desktop/
│  ├─ src/                       # React + TS
│  │  ├─ design-system/          # tokens.css، preset تیلویند، GlassCard، Sidebar، TitleBar،
│  │  │                          # Switch، PingBars، CountryBadge، Chip، StatStrip، UsageBar
│  │  ├─ features/               # auth، home، servers، connections، services، shop، account،
│  │  │                          # referral، tickets، analytics، tools، settings
│  │  ├─ windows/                # main، tray-flyout
│  │  ├─ ipc/                    # bindingهای تولیدشده با tauri-specta
│  │  └─ i18n/fa.ts              # همه‌ی متن‌ها و پیام‌های خطای فارسی
│  └─ src-tauri/                 # Tauri: commands، tray، windows، capabilities
├─ crates/
│  ├─ geek-api/                  # کلاینت بک‌اند، refresh single-flight
│  ├─ geek-secrets/              # keyring، device_id
│  ├─ geek-config/               # parse لینک‌ها (vless/vmess/trojan/ss/hy2/…)،
│  │                             # تولید کانفیگ Xray و sing-box، presetهای مسیر
│  ├─ geek-smart/                # SmartConnect + FailoverPolicy (پورت کاتلین)
│  ├─ geek-ipc/                  # پروتکل نسخه‌دار helper ↔ app
│  ├─ geek-netplat/              # traitها + پیاده‌سازی هر OS
│  └─ geek-helper/               # دیمن privileged
├─ core/geekcore/                # Go: xray-core + cfscan + delay، JSON-RPC stdio
├─ resources/                    # rule-set ایران (.srs)، ipv4.txt، فونت‌ها، wintun.dll
├─ scripts/fetch-deps.*          # دانلود sing-box/wintun با نسخه و sha256 پین‌شده
├─ packaging/                    # MSI/NSIS، pkg + launchd plist، deb/rpm + systemd unit
├─ design/                       # خروجی بردها و توکن‌ها (مرجع)
├─ docs/
└─ .github/workflows/
```

---

## ۸. تصمیم‌ها (بسته‌شده پس از فاز ۰)

| موضوع | تصمیم |
|---|---|
| طراحی صفحه‌های بدون برد | در کانواس طراحی شدند (۱۸ برد دسکتاپ جدید، بخش ۱۰). |
| TitleBar مک | traffic light بومی در مک، دکمه‌های سفارشی طراحی در ویندوز/لینوکس. |
| Mica / Vibrancy | فقط برای flyout کنار ساعت؛ پنجره‌ی اصلی همان پس‌زمینه‌ی آبی طراحی را دارد. |
| تم تیره | از پالت تیره‌ی اپ اندروید (`GeekColors.DarkGeekColors`) — همان توکن‌ها، برد `Desktop-Home-Dark`. |
| بک‌اند | فقط یک تغییر لازم بود: متن تأیید ورود در ربات «اندروید» را hardcode کرده بود. در شاخه‌ی `claude/desktop-app-signin` ریپوی GeekVPNBot درست شد. مصرف روزانه و گزارش مشکل مثل اندروید روی خود دستگاه ساخته می‌شوند و API جدید نمی‌خواهند. |
| cfscan | به‌صورت Go module از ریپوی اندروید (`github.com/amirhosseintowfighi/geekvpn-android/cfscan`) با پین روی commit. |
| patchهای Xray | همان `scripts/xray-patches` اندروید روی `geekcore` اعمال می‌شود (VLESS بدون TLS برای سرویس‌های تونل). شمارش اتصال از Clash API sing-box می‌آید و patch دوم لازم نیست. |
| لینوکس | `.deb`، `.rpm` و AppImage. AppImage بار اول helper را با `pkexec` نصب می‌کند. |
| امضا | گواهی ویندوز و Apple Developer ID را مالک پروژه تهیه می‌کند؛ CI از Secrets می‌خواند و بدون آن‌ها build امضانشده می‌سازد. |
| حداقل macOS | 13 (به خاطر `SMAppService`). |
| لایسنس | GPL-3.0، مثل اپ اندروید. |
| Kill Switch پس از کرش | پیش‌فرض باز؛ «حالت سخت‌گیر» در تنظیمات. |
| SmartConnect | مثل اندروید: ۳ تلاش (۱ + ۲ failover)، ۳ IP تمیز، آستانه‌ی failover پیش‌فرض ۲ ثانیه. |
| Speedtest | `speed.cloudflare.com` از داخل proxy محلی، مثل اندروید (میانه‌ی ۵ پینگ، هر مرحله حداکثر ۱۰ ثانیه، دانلود ۵۰ و آپلود ۲۰ مگابایت). |
| به‌روزرسانی | Tauri updater؛ منبع: GitHub Releases همین ریپو با mirror داخلی. endpoint دسکتاپ بک‌اند در فاز ۶ اضافه می‌شود. |

## ۹. هم‌ترازی با اپ اندروید (PR شماره‌ی ۳ ریپوی GeekVPN-Android)

| قابلیت اندروید | معادل دسکتاپ | برد |
|---|---|---|
| ورود با تلگرام، نام کاربری، شروع سریع | همان؛ به‌علاوه‌ی QR لینک ورود برای اسکن با گوشی | Desktop-Login، Desktop-Login-Wait |
| اتصال هوشمند با مراحل روی Home | هاب مراحل کنار دکمه‌ی اتصال (IP تمیز، تست، فایروال، تلاش n از m، تست نهایی) | Desktop-Home-Connecting |
| Failover و آستانه (هرگز/قطعی/۱/۲/۳ ث) | همان، در helper تا با بسته شدن UI ادامه دهد | Desktop-Servers |
| سرورهای ستاره‌دار، مرتب‌سازی «کمترین پینگ» | همان + ستاره‌دارها در منوی tray | Desktop-Servers، Desktop-Tray |
| اسکنر کلادفلر (فقط سرویس `direct` پشت CDN، `CloudflareCheck`) | همان ماژول Go، همان قواعد | Desktop-Tools |
| تست سرعت | همان | Desktop-Tools |
| «اتصالات» (تعداد اتصال‌های باز) | فهرست کامل اتصال‌ها از Clash API + بستن و مسدودسازی | Desktop-Connections |
| مصرف روزانه (۷/۳۰ روز، روی دستگاه، ۴۵ روز) | همان؛ شمارنده از آمار sing-box در helper | Desktop-Usage |
| تونل تفکیکی + «برنامه‌های ایرانی مستقیم» | geosite/geoip ایران + انتخاب فایل اجرایی برنامه‌ها (`process_path`) | Desktop-Split |
| اتصال خودکار با روشن شدن / Wi-Fi ناشناس / شبکه‌ی مورد اعتماد | autostart سیستم‌عامل + NetworkMonitor | Desktop-Settings |
| راهنمای Kill Switch | Kill Switch واقعی با فایروال (WFP / pf / nftables) | Desktop-Settings |
| قفل برنامه (اثر انگشت) | Windows Hello / Touch ID / polkit | Desktop-Settings |
| هشدار تمام شدن سرویس (۸۰٪ یا ۳ روز) | نوتیفیکیشن بومی با دکمه‌ی «تمدید» | — |
| تیکت‌های من + تعداد نخوانده | همان، با badge روی سایدبار | Desktop-Support |
| گزارش مشکل با redact | همان منطق `ProblemReport.redact` | Desktop-Report |
| دعوت از دوستان | همان + QR لینک دعوت | Desktop-Referral |
| لینک و QR سرویس | پنل کناری QR | Desktop-Services |
| افزودن/ویرایش لینک دستی | همان + چسباندن با Ctrl+V | Desktop-Services |
| به‌روزرسانی داخل اپ | Tauri updater با تأیید امضا | Desktop-Update |
| تنظیمات پیشرفته | همان گروه‌ها (هسته، DNS، Mux، Fragment) | Desktop-Settings |
| پیام خلاصه پس از قطع (مدت و حجم) | نوتیفیکیشن بومی | — |
| کاشی، ویجت، میانبر | منوی tray، پنل کوچک، میانبر سراسری `Ctrl+Shift+K` | Desktop-Tray |
| Push از Firebase | کنار گذاشته شد: FCM روی دسکتاپ پشتیبانی رسمی ندارد. اطلاعیه‌ها از API خوانده می‌شوند (در صورت نیاز). | — |

## ۱۰. ناوبری دسکتاپ

سایدبار (از راست): خانه، سرورها، سرویس‌ها، فروشگاه، اتصالات، ابزارها، پشتیبانی، حساب، تنظیمات. «پروفایل‌ها» مثل اندروید حذف شد (لینک‌های دستی در سرویس‌ها هستند) و «درخواست‌ها» و «گزارش‌ها» در اتصالات و ابزارها ادغام شدند. لوگوی کم‌رنگ پس‌زمینه، مثل اندروید، حذف شد.

منبع بردها: `design/desktop/` (تولیدشده با `design/desktop/gen.py`) و کانواس GeekVPN Client.

## ۱۱. وضعیت فازها

| فاز | وضعیت |
|---|---|
| ۰ — معماری | تأیید شد |
| ۱ — پوسته، پنجره‌ی بی‌قاب، سیستم طراحی | انجام شد |
| ۲ — ورود با تلگرام و نشست امن | انجام شد و end-to-end روی GeekVPNBot واقعی (Postgres و Redis) تست شد |
| ۳a — هسته، سرورها، پروکسی سیستم، اتصال | انجام شد و end-to-end روی لینوکس (GNOME) با یک سرور VLESS واقعی تست شد |
| ۳b — helper، TUN، Kill Switch، تونل برنامه‌ای، اتصالات | انجام شد؛ end-to-end روی لینوکس (داخل network namespace، با سرور VLESS واقعی) تست شد |

### یادداشت‌های فاز ۲
- `crates/geek-api`: کلاینت تایپ‌شده، `Session` با refresh یکی‌یکی (single-flight)، و `wait_for_approval` با retry و لغو. تست‌های قرارداد با fixtureهایی اجرا می‌شوند که از مدل‌های Pydantic خود بک‌اند ساخته شده‌اند (camelCase، `message_fa`).
- `crates/geek-secrets`: `device_id` = HMAC-SHA256 شناسه‌ی ماشین (۶۴ کاراکتر hex)، و نشست در Keychain / Credential Manager / Secret Service. اگر کلیدساز در دسترس نباشد، هیچ فایل متنی‌ای نوشته نمی‌شود؛ نشست تا بستن برنامه می‌ماند و UI دلیلش را می‌گوید.
- توکن‌ها هرگز وارد WebView نمی‌شوند.
- لینک تلگرام: اگر handler `tg:` نصب باشد با `tg://` باز می‌شود، وگرنه با `https://t.me`. کد QR همان لینک برای تأیید از گوشی است.
- دو باگ بک‌اند که در این تست پیدا شد و در [amirhosseintowfighi/GeekVPNBot#7](https://github.com/amirhosseintowfighi/GeekVPNBot/pull/7) درست شد. اولی اپ اندروید را هم در production تحت تأثیر قرار می‌دهد:
  1. CSRF همه‌ی refreshهای بدون Bearer را با ۴۰۳ رد می‌کرد؛ در نتیجه نشست اپ‌ها بعد از ۱۵ دقیقه بی‌صدا از کار می‌افتاد.
  2. revoke شدن نشست بعد از تشخیص استفاده‌ی دوباره از refresh token، rollback می‌شد و توکن rotateشده همچنان کار می‌کرد.

### یادداشت‌های فاز ۳a
- **`core/geekcore`** (Go): همان commit ایکس‌ری اندروید (Xray 26.9.9) با همان patch (VLESS بدون TLS برای سرویس‌های تونل)، به‌علاوه‌ی `cfscan` از ریپوی اندروید. پروتکلش JSON خط‌به‌خط روی stdin/stdout است، با این متدها:
  - `core.start` / `core.stop`: اجرا و توقف هسته
  - `core.delay`: تست تأخیر روی اتصال زنده
  - `core.traffic`: شمارنده‌های بایت
  - `test.delay`: تست تأخیر واقعی موازی، مثل v2rayNG
  - `scan.*`: اسکنر

  ساخت با `scripts/build-geekcore.sh` انجام می‌شود و داده‌ی geo با `scripts/fetch-geo.sh`، از همان منبع اندروید.
- **`crates/geek-config`**: لینک‌های vless، vmess، trojan و ss (با همان نام فیلدهای v2rayN و v2rayNG) و اشتراک base64 را به کانفیگ Xray تبدیل می‌کند. مسیر «هوشمند» قانون‌به‌قانون همان preset `WHITE_IRAN` در v2rayNG است. `domainStrategy: AsIs` است تا انتخاب مسیر هیچ درخواست DNS محلی نسازد.
- **`crates/geek-core`**: سوپروایزر geekcore. هر فراخوانی timeout دارد. اگر هسته کرش کند، درخواست‌های در جریان فوراً با آخرین خط‌های لاگ fail می‌شوند و رویداد `core.exited` فرستاده می‌شود.
- **`crates/geek-netplat`**: پروکسی سیستم.
  - ویندوز: رجیستری Internet Settings به‌علاوه‌ی refresh از طریق WinINet
  - مک: `networksetup` روی همه‌ی سرویس‌های شبکه‌ی فعال
  - لینوکس: GNOME (`gsettings`) و KDE (`kioslaverc`)

  تنظیمات قبلی قبل از هر تغییر روی دیسک نوشته می‌شود و این حالت‌ها برمی‌گردد: قطع اتصال، خروج، SIGTERM/SIGINT/SIGHUP، و شروع بعد از کرش.
- **سرویس‌های حساب**: از `/api/miniapp/subscriptions` همگام می‌شوند؛ سرویس غیرفعال سرور ندارد. لینک‌های دستی دست نمی‌خورند، مثل `AccountSync` اندروید.
- **تغییر نسبت به طرح فاز ۰:** در حالت پروکسی سیستم، خود Xray با inboundهای socks و http و قانون‌های مسیرش کار می‌کند و sing-box جلوی آن نیست. دلیلش این است که قطعه‌ی متحرک کمتری دارد و قانون‌های مسیر همان قانون‌های Xray اندروید است. sing-box با TUN در فاز ۳b وارد می‌شود.

### یادداشت‌های فاز ۳b
- **`crates/geek-helper` (`geekvpn-helper`)**: تنها بخشی که با دسترسی root یا SYSTEM اجرا می‌شود و فقط دو کار دارد: TUN (با sing-box) و فایروال Kill Switch. به‌صورت سرویس اجرا می‌شود:
  - لینوکس: systemd
  - مک: launchd daemon
  - ویندوز: Windows Service (LocalSystem، شروع خودکار)

  `install` خودش و sing-box را به پوشه‌ای کپی می‌کند که فقط root/SYSTEM می‌تواند در آن بنویسد (`/usr/local/lib/geekvpn`، `/Library/PrivilegedHelperTools/com.geekvpn.helper`، `%ProgramFiles%\GeekVPN Helper`) و سرویس را از همان‌جا ثبت می‌کند، نه از جایی که اپ نصب است (AppImage در `/tmp` mount می‌شود و نصب ویندوز per-user است).
- **نصب helper**:
  - بسته‌ی `.deb` و `.rpm` در postinst خودش نصبش می‌کند و در prerm برش می‌دارد.
  - در بقیه‌ی حالت‌ها دکمه‌ی «نصب سرویس» در تنظیمات با پنجره‌ی اجازه‌ی مدیر خود سیستم‌عامل اجرا می‌شود: `pkexec` در لینوکس، پنجره‌ی رمز در مک، UAC در ویندوز.
  - در مک، `SMAppService` بعد از امضا و notarize جای این روش را می‌گیرد.
- **`crates/geek-ipc`**: پروتکل نسخه‌دار (`PROTOCOL = 1`) روی Unix socket یا named pipe. همه‌ی درخواست‌ها تایپ‌شده‌اند: `hello`، `status`، `tunStart`، `tunStop`، `killSwitchRelease`.
  - helper کانفیگ sing-box را خودش از یک `TunSpec` می‌سازد. هیچ فراخواننده‌ای نمی‌تواند کانفیگ یا مسیر فایلی به پروسه‌ی root بدهد؛ فقط تصمیم مسیریابی را می‌گیرد.
  - مثل daemon مالواد، هر کاربر محلی می‌تواند وصل شود. named pipe ویندوز به کاربرهای وارد‌شده فقط خواندن و نوشتن می‌دهد و با first-instance ساخته می‌شود تا پروسه‌ی دیگری نتواند خودش را جای helper جا بزند.
- **sing-box** با `scripts/build-singbox.sh` از ماژول رسمی نسخه‌ی v1.14.2 ساخته می‌شود و go.sum و checksum database Go آن را می‌سنجند. اسمش `geekvpn-sing-box` است تا با sing-box خود کاربر قاطی نشود.
- **مسیر در حالت TUN**:
  - geekcore پشت یک SOCKS inbound روی loopback اجرا می‌شود که با نام کاربری و رمز تصادفی هر اتصال قفل است.
  - sing-box تصمیم مسیر را می‌گیرد: ایران مستقیم، شبکه‌ی محلی مستقیم، و تونل برنامه‌ای.
  - اتصال‌های خود geekcore با قانون `process_path` مستقیم می‌روند تا تونل خودش را تغذیه نکند.
  - DNS از داخل تونل حل می‌شود (DoH روی 1.1.1.1). دامنه‌های ایران و DNS خود geekcore از resolver محلی می‌روند.
- **فهرست ایران یکی است**: sing-box 1.12+ فایل‌های geo ایکس‌ری را نمی‌خواند. اپ `geoip:ir` و `geosite:category-ir` را از همان `geoip.dat` و `geosite.dat` که همراه دارد بیرون می‌کشد (پارسر protobuf کوچک در `geek-config`) و inline به sing-box می‌دهد. پس «هوشمند» در هر دو حالت با یک داده مسیر می‌دهد.
  - تغییر نسبت به طرح فاز ۰: در حالت پروکسی سیستم هنوز Xray مسیر را تعیین می‌کند (فاز ۳a) و در حالت TUN، sing-box.
- **Kill Switch** فقط در حالت TUN است، همان‌طور که در طراحی آمده. چیزی که بیرون می‌رود: loopback، کارت TUN، اتصال‌های خود sing-box، DHCP و در صورت اجازه شبکه‌ی محلی. بقیه بسته است. اتصال‌های sing-box در هر سیستم‌عامل این‌طور شناخته می‌شوند:
  - لینوکس: nftables با mark فایروال (`route.default_mark`)
  - ویندوز: WFP با app id فایل sing-box
  - مک: pf ابزاری برای شناختن پروسه ندارد، پس socketهای root را رد می‌کند؛ یعنی sing-box و daemonهای سیستم، نه برنامه‌های کاربر. anchor آن `com.apple/250.GeekVPN` است تا بدون دست زدن به `/etc/pf.conf` اعمال شود.
- **مالکیت**:
  - اتصالی که تونل را شروع کرده مالک آن است. وقتی این اتصال بسته شود (اپ بسته شده یا کرش کرده)، تونل هم می‌ایستد.
  - Kill Switch معمولی در این حالت باز می‌شود. حالت سخت‌گیر بعد از کرش و ری‌استارت هم بسته می‌ماند: در ویندوز با فیلترهای persistent و در لینوکس و مک با بازگرداندن آن در شروع helper.
  - اگر sing-box خودش بمیرد، Kill Switch می‌ماند و خانه دکمه‌ی «قطع و باز کردن اینترنت» نشان می‌دهد.
  - اتصالی که اصلاً بالا نیامده چیزی برای محافظت ندارد، پس Kill Switch آن آزاد می‌شود.
- **اتصالات**: از Clash API خود sing-box روی loopback با secret تصادفی خوانده می‌شود، هر ثانیه یک بار. در حالت پروکسی سیستم فهرستی نیست و صفحه همین را می‌گوید.
- **تونل برنامه‌ای**: سه حالت دارد: همه از VPN، این برنامه‌ها مستقیم، فقط این برنامه‌ها از VPN. برنامه‌ها با مسیر فایل اجرایی شناخته می‌شوند و از فهرست برنامه‌های در حال اجرا یا با انتخاب فایل اضافه می‌شوند.
- **آنچه تست شده** (لینوکس، داخل network namespace، با اپ واقعی):
  - اتصال TUN به سرور VLESS
  - فهرست اتصالات
  - Kill Switch بعد از مرگ sing-box
  - «قطع» که اینترنت را باز می‌کند
  - تونل برنامه‌ای در حالت مستقیم (curl مستقیم، بقیه از تونل)
  - Kill Switch سخت‌گیر بعد از ری‌استارت helper
- **آنچه تست نشده**:
  - WFP ویندوز و pf مک روی دستگاه واقعی اجرا نشده‌اند؛ فقط کامپایل و در CI build می‌شوند.
  - نصب سرویس با systemd، launchd و SCM واقعی تست نشده، چون این محیط systemd ندارد.
- **باگ فاز ۳a که در این تست پیدا شد**: منبع‌ها (sources) با `kind` تودرتو serialize می‌شدند و نوار حجم سرویس در خانه هیچ‌وقت نشان داده نمی‌شد.
- **geekcore**: لاگ Xray دیگر وارد stdout پروتکل نمی‌شود و به stderr می‌رود. پیش از این، خطای کرش هسته خالی می‌آمد.
