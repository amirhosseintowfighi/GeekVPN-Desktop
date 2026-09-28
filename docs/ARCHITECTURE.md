# GeekVPN Desktop — سند معماری (فاز ۰)

وضعیت: **پیش‌نویس برای تأیید**. هیچ کدی هنوز نوشته نشده است.

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

## ۸. سؤال‌های باز (قبل از فاز ۱)

1. **طراحی صفحه‌های بدون برد دسکتاپ.** (الف) اول بردهای دسکتاپ ورود، flyout tray، سرویس‌ها، فروشگاه، حساب، ابزارها، تنظیمات و حالت تیره را در همان کانواس طراحی کنم و بعد کد بزنم، یا (ب) در کد، بردهای موبایل را داخل قالب دسکتاپ (سایدبار + پنل‌ها) بچینم؟ پیشنهاد من (الف) برای ورود، flyout و تنظیمات است.
2. **TitleBar در مک:** دکمه‌های سفارشی طراحی (مثل ویندوز) یا traffic light بومی macOS؟ پیشنهاد: بومی در مک، سفارشی در ویندوز/لینوکس.
3. **Mica/Acrylic/Vibrancy** فقط برای flyout، یا کل پنجره‌ی اصلی هم شفاف شود (که رنگ آبی طراحی را تغییر می‌دهد)؟
4. **حالت تیره:** طراحی فقط یک تم دارد (Sky Lens). پالت تیره را من پیشنهاد بدهم یا در کانواس طراحی شود؟
5. **تغییرات بک‌اند** (بخش ۴): متن ربات بر اساس platform، endpoint تاریخچه‌ی مصرف، ضمیمه‌ی تیکت، `min_version` دسکتاپ. من این‌ها را در GeekVPNBot انجام بدهم (PR جدا) یا تیم بک‌اند؟ تا آن موقع نمودار مصرف و آپلود لاگ غیرفعال و با برچسب «به‌زودی» نشان داده شوند؟
6. **cfscan:** به‌صورت Go module از ریپوی اندروید (پین روی commit) import شود، یا به ریپوی مستقل منتقل شود تا دو اپ آن را share کنند؟
7. **توزیع لینوکس:** `.deb` و AppImage کافی است یا `.rpm` و AUR هم؟ برای AppImage نصب یک‌باره‌ی helper با `pkexec` قبول است؟
8. **امضای کد:** گواهی Windows (EV یا Azure Trusted Signing) و Apple Developer ID برای notarize دارید؟ بدون Developer ID، helper مک با `SMAppService` کار نمی‌کند. حداقل نسخه‌ی macOS: 13 قبول است؟
9. **رنج IP کلادفلر:** از کدام endpoint سرور به‌روز شود؟ (الان endpointی در بک‌اند نیست؛ فعلاً فایل باندل‌شده.)
10. **Speedtest:** `speed.cloudflare.com` از پشت VPN، یا سرور تست اختصاصی خودتان؟
11. **لایسنس ریپوی دسکتاپ:** اندروید GPL-3.0 است (فورک v2rayNG). sing-box هم GPL است و ما آن را به‌صورت باینری جدا کنار اپ می‌گذاریم. لایسنس این ریپو را GPL-3.0 بگذارم؟
12. **Kill Switch پیش‌فرض:** بعد از کرش helper/سیستم، ترافیک بسته بماند (سخت‌گیر) یا باز شود؟ پیشنهاد: پیش‌فرض باز، گزینه‌ی سخت‌گیر در تنظیمات.
13. **تعداد تلاش SmartConnect:** مثل اندروید ۳ تلاش (۱ + ۲ failover) یا دقیقاً ۲؟

---

## ۹. برنامه‌ی فاز ۱ (پس از تأیید)

- اسکلت monorepo (Cargo workspace + pnpm)، Tauri v2 با پنجره‌ی frameless و `data-tauri-drag-region`، single-instance.
- `design-system/`: توکن‌ها به CSS variables + preset تیلویند، فونت‌های لوکال، کامپوننت‌های پایه، `dir="rtl"` سراسری.
- TitleBar، Sidebar و صفحه‌ی Home با داده‌ی خالی/حالت قطع (بدون داده‌ی ساختگی: جاهایی که هنوز سیم‌کشی نشده‌اند وضعیت خالی واقعی نشان می‌دهند).
- مقایسه‌ی بصری خودکار با Playwright: رندر Home در ۱۲۸۰×۸۰۰ در برابر برد `Desktop-Home`.
- CI اولیه: lint + typecheck + `cargo test` + build روی سه OS.
