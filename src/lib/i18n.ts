import { ref, watchEffect } from "vue";

// Strings in the UI are written in English and used as the lookup key: a missing translation simply shows the
// English text, and adding a language is just another dictionary.
export type Lang = "en" | "fa";

const STORAGE_KEY = "vanish-lang";

function initialLang(): Lang {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "en" || saved === "fa") return saved;
  } catch {
    /* storage unavailable */
  }
  return navigator.language?.toLowerCase().startsWith("fa") ? "fa" : "en";
}

export const lang = ref<Lang>(initialLang());

export function setLang(l: Lang) {
  lang.value = l;
  try {
    localStorage.setItem(STORAGE_KEY, l);
  } catch {
    /* storage unavailable */
  }
}

/** Keeps <html lang/dir> in sync so layout mirrors for right-to-left languages. */
export function applyLang() {
  watchEffect(() => {
    const el = document.documentElement;
    el.lang = lang.value;
    el.dir = lang.value === "fa" ? "rtl" : "ltr";
  });
}

const fa: Record<string, string> = {
  // general
  Vanish: "Vanish",
  Settings: "تنظیمات",
  "Core log": "لاگ هسته",
  Close: "بستن",
  Cancel: "انصراف",
  Add: "افزودن",
  Added: "اضافه شد",
  Clear: "پاک کردن",
  Copy: "کپی",
  Copied: "کپی شد",
  Paste: "چسباندن",
  Import: "ایمپورت",
  All: "همه",
  Manual: "دستی",
  Language: "زبان",
  "Loading…": "در حال بارگذاری…",
  "Testing…": "در حال تست…",
  "Checking…": "در حال بررسی…",
  "Updating…": "در حال به‌روزرسانی…",
  never: "هرگز",
  timeout: "تایم‌اوت",
  ms: "ms",
  all: "همه",
  warn: "هشدار",
  error: "خطا",

  // main window
  "System Proxy": "پراکسی سیستم",
  Working: "در حال انجام",
  Connected: "متصل",
  Offline: "قطع",
  Connect: "اتصال",
  Disconnect: "قطع اتصال",
  "Select or import a node": "یک کانفیگ انتخاب یا ایمپورت کنید",
  Latency: "تأخیر",
  Exit: "خروجی",
  Country: "کشور",
  City: "شهر",
  ZIP: "کد پستی",
  Host: "هاست",
  ISP: "ارائه‌دهنده",
  "Speed & leak test": "تست سرعت و نشت",
  "Live traffic": "ترافیک زنده",
  peak: "اوج",
  DOWN: "دانلود",
  UP: "آپلود",
  "Version {v} is available": "نسخه‌ی {v} در دسترس است",
  "Update now": "به‌روزرسانی",
  "The core stopped unexpectedly. The kill switch is blocking traffic. Press the power button to reconnect.":
    "هسته ناگهان متوقف شد و کیل سوییچ ترافیک را مسدود کرده است. برای اتصال دوباره دکمه‌ی پاور را بزنید.",

  // nodes
  "Nodes ·": "کانفیگ‌ها ·",
  Fastest: "سریع‌ترین",
  "Ping all, then select the quickest": "پینگ همه و انتخاب سریع‌ترین",
  "TCP ping": "پینگ TCP",
  "TCP connect time to each server": "زمان اتصال TCP به هر سرور",
  "Real delay": "تأخیر واقعی",
  "Real HTTP request through each node, like URL test": "درخواست HTTP واقعی از طریق هر کانفیگ، مثل URL test",
  "Real HTTP delay": "تأخیر واقعی HTTP",
  Ping: "پینگ",
  tcp: "tcp",
  Subscription: "اشتراک",
  "Links / JSON": "لینک / JSON",
  "Add subscription": "افزودن اشتراک",
  "Name (e.g. Work, Home)": "نام (مثلاً کار، خانه)",
  "vless:// vmess:// trojan:// ss:// wireguard:// links, or sing-box outbound JSON": "لینک‌های vless:// vmess:// trojan:// ss:// wireguard:// یا JSON خروجی sing-box",
  "Clipboard access was denied": "دسترسی به کلیپ‌بورد رد شد",
  "Search nodes": "جستجوی کانفیگ",
  "No nodes yet.": "هنوز کانفیگی نیست.",
  "Import a subscription or paste a config link.": "یک اشتراک ایمپورت کنید یا لینک کانفیگ را بچسبانید.",
  "No nodes match.": "کانفیگی پیدا نشد.",
  "Total traffic through this node": "کل ترافیک عبوری از این کانفیگ",
  "Show QR code": "نمایش QR",
  "Remove node": "حذف کانفیگ",
  "Save name": "ذخیره نام",
  "Rename subscription": "تغییر نام اشتراک",
  "Show subscription QR": "نمایش QR اشتراک",
  "Refresh subscription": "به‌روزرسانی اشتراک",
  "Remove subscription": "حذف اشتراک",
  Updated: "به‌روزرسانی",
  'Remove "{name}" and its {n} node(s)?': "«{name}» و {n} کانفیگ آن حذف شود؟",

  // subscription usage
  Used: "مصرف",
  Remaining: "باقی‌مانده",
  Total: "کل",
  Unlimited: "نامحدود",
  "not reported": "اعلام نشده",
  "Total not reported by this panel": "کل حجم توسط این پنل اعلام نشده",
  "% used": "٪ مصرف‌شده",
  Expired: "منقضی شده",
  "Expires today": "امروز منقضی می‌شود",
  "{n} days left": "{n} روز مانده",
  "Via Vanish": "از طریق Vanish",
  "Traffic sent through this subscription's nodes in Vanish": "ترافیکی که از کانفیگ‌های این اشتراک در Vanish عبور کرده",
  "This subscription does not report traffic or expiry. Refresh it to check again.": "این اشتراک مصرف یا تاریخ انقضا را اعلام نمی‌کند. برای بررسی دوباره آن را به‌روزرسانی کنید.",

  // settings
  General: "عمومی",
  "Start with Windows": "اجرا همراه ویندوز",
  "Launches minimized to the tray at login.": "هنگام ورود، کوچک‌شده در tray اجرا می‌شود.",
  "Connect on launch": "اتصال هنگام اجرا",
  "Connects to the selected node as soon as the app starts.": "به‌محض اجرای برنامه به کانفیگ انتخاب‌شده وصل می‌شود.",
  "Auto fallback": "جایگزینی خودکار",
  "Switches to the next-fastest node when the current one stops responding.": "وقتی کانفیگ فعلی جواب ندهد، به سریع‌ترین کانفیگ بعدی می‌رود.",
  "Kill switch": "کیل سوییچ",
  "System Proxy mode: if the core dies, traffic is blocked instead of leaking.": "در حالت پراکسی سیستم: اگر هسته از کار بیفتد، ترافیک به‌جای نشت مسدود می‌شود.",
  "Refresh subscriptions": "به‌روزرسانی اشتراک‌ها",
  "How often saved subscriptions update in the background.": "هر چند وقت اشتراک‌های ذخیره‌شده در پس‌زمینه به‌روز شوند.",
  Off: "خاموش",
  "1 hour": "۱ ساعت",
  "6 hours": "۶ ساعت",
  "12 hours": "۱۲ ساعت",
  "24 hours": "۲۴ ساعت",
  Routing: "مسیریابی",
  Global: "سراسری",
  Rule: "قانونی",
  Direct: "مستقیم",
  "Everything through the proxy.": "همه‌چیز از پراکسی می‌گذرد.",
  ".ir sites and your lists below go direct.": "سایت‌های .ir و فهرست‌های شما بدون پراکسی می‌روند.",
  "Nothing is proxied.": "هیچ‌چیز پراکسی نمی‌شود.",
  "Changes apply immediately, reconnecting if needed.": "تغییرات فوراً اعمال می‌شود و در صورت نیاز اتصال دوباره برقرار می‌شود.",
  "Direct domains": "دامنه‌های مستقیم",
  "One per line. Matches the domain and its subdomains (e.g. example.com).": "هر خط یک مورد. دامنه و زیردامنه‌هایش را شامل می‌شود (مثلاً example.com).",
  "Direct apps": "برنامه‌های مستقیم",
  "These programs bypass the proxy.": "این برنامه‌ها بدون پراکسی کار می‌کنند.",
  "No apps added.": "برنامه‌ای اضافه نشده.",
  "Choose app…": "انتخاب برنامه…",
  Network: "شبکه",
  "DNS over HTTPS server": "سرور DNS روی HTTPS",
  "Test URL": "آدرس تست",
  "Used by Real delay and the latency check. Any HTTP(S) URL that returns quickly.": "برای تأخیر واقعی و بررسی تأخیر استفاده می‌شود. هر آدرس HTTP(S) سریع.",
  "Local port": "پورت محلی",
  "API port": "پورت API",
  About: "درباره",
  "Check for updates": "بررسی به‌روزرسانی",
  "Install {v}": "نصب {v}",
  Version: "نسخه",
  "Reset to default": "بازنشانی به پیش‌فرض",
  "Restores every setting above. Your nodes and subscriptions are kept.": "همه‌ی تنظیمات بالا را برمی‌گرداند. کانفیگ‌ها و اشتراک‌ها حفظ می‌شوند.",
  Reset: "بازنشانی",
  "Click again to confirm": "برای تأیید دوباره کلیک کنید",
  "Settings restored to defaults": "تنظیمات به پیش‌فرض برگشت",

  // pickers / modals
  "Choose an application": "انتخاب برنامه",
  "Search running apps": "جستجوی برنامه‌های باز",
  "Browse…": "مرور…",
  "Apps open on this PC. Traffic from the apps you pick bypasses the proxy.": "برنامه‌های باز روی این رایانه. ترافیک برنامه‌هایی که انتخاب کنید بدون پراکسی می‌رود.",
  "No matching apps. Use Browse to pick an .exe.": "برنامه‌ای پیدا نشد. با «مرور» فایل .exe را انتخاب کنید.",
  "QR code": "کد QR",
  "Too long to fit in a QR code. Copy it instead.": "برای QR بیش از حد طولانی است. به‌جای آن کپی کنید.",
  "No log lines yet.": "هنوز لاگی نیست.",

  // tools
  "Network tools": "ابزارهای شبکه",
  "Speed test": "تست سرعت",
  "Leak test": "تست نشت",
  "Start speed test": "شروع تست سرعت",
  GO: "GO",
  Download: "دانلود",
  Upload: "آپلود",
  DOWNLOAD: "دانلود",
  UPLOAD: "آپلود",
  PING: "پینگ",
  "· jitter": "· جیتر",
  "Run again": "اجرای دوباره",
  "Run test": "اجرای تست",
  "via tunnel": "از طریق تونل",
  "direct connection": "اتصال مستقیم",
  "Measured through the active tunnel.": "از طریق تونل فعال اندازه‌گیری شد.",
  "Measured on your direct connection (not connected).": "روی اتصال مستقیم شما اندازه‌گیری شد (متصل نیستید).",
  "Runs through the active tunnel. Takes about 25 seconds.": "از طریق تونل فعال انجام می‌شود. حدود ۲۵ ثانیه طول می‌کشد.",
  "Not connected: this tests your direct connection. Takes about 25 seconds.": "متصل نیستید: اتصال مستقیم شما تست می‌شود. حدود ۲۵ ثانیه طول می‌کشد.",
  "Compares your direct connection with the tunnel: exit IP, DNS resolver and IPv6.": "اتصال مستقیم شما را با تونل مقایسه می‌کند: IP خروجی، DNS و IPv6.",
  OK: "سالم",
  Warning: "هشدار",
  Problem: "مشکل",
  Info: "اطلاعات",
  Tunnel: "تونل",
  "Your IP": "IP شما",
  "DNS resolver": "سرور DNS",
  "IP leak": "نشت IP",
  "DNS leak": "نشت DNS",
  IPv6: "IPv6",

  // store messages
  "Node unreachable — switched to {name}": "کانفیگ در دسترس نبود — به {name} تغییر کرد",
  "Fastest: {name}": "سریع‌ترین: {name}",
  "no node answered the TCP ping": "هیچ کانفیگی به پینگ TCP جواب نداد",
  "Imported {n} node(s)": "{n} کانفیگ ایمپورت شد",
  ", {n} skipped": "، {n} مورد رد شد",
  "Updated: {n} node(s)": "به‌روز شد: {n} کانفیگ",
  "You are on the latest version": "نسخه‌ی شما به‌روز است",
  "Could not check for updates: {e}": "بررسی به‌روزرسانی ممکن نشد: {e}",

  // game mode
  "Game mode": "گیم مود",
  "Low lag and a steady connection for online games.": "پینگ پایین و اتصال پایدار برای بازی آنلاین.",
  "Connect to start monitoring.": "برای شروع پایش، وصل شوید.",
  "No answer yet": "هنوز پاسخی نیامده",
  "Measuring…": "در حال اندازه‌گیری…",
  Excellent: "عالی",
  Good: "خوب",
  Playable: "قابل‌بازی",
  Laggy: "لگ دارد",
  Jitter: "جیتر",
  Loss: "افت بسته",
  Games: "بازی‌ها",
  "Add game": "افزودن بازی",
  Remove: "حذف",
  "Only these games use the tunnel. Everything else stays direct, so it never competes with them.": "فقط این بازی‌ها از تونل استفاده می‌کنند. بقیه مستقیم می‌روند و با بازی رقابت نمی‌کنند.",
  "No games added: all traffic uses the tunnel.": "بازی‌ای اضافه نشده: همه‌ی ترافیک از تونل می‌گذرد.",
  "Uses TUN, so run Vanish as administrator. If the server stops answering, Vanish switches to the next-best one within seconds.": "از TUN استفاده می‌کند، پس Vanish را با دسترسی ادمین اجرا کنید. اگر سرور پاسخ ندهد، Vanish ظرف چند ثانیه به بهترین سرور بعدی می‌رود.",
  "Game mode uses TUN.": "گیم مود از TUN استفاده می‌کند.",
  "Apps open on this PC. Only the games you pick use the tunnel; everything else stays direct.": "برنامه‌های باز روی این رایانه. فقط بازی‌هایی که انتخاب کنید از تونل می‌گذرند؛ بقیه مستقیم می‌روند.",

  // layout
  Servers: "سرورها",
  Mode: "حالت",
  Test: "تست",
  "Add servers": "افزودن سرور",
  Connection: "اتصال",
  "Connect to see your new location.": "برای دیدن موقعیت جدیدتان وصل شوید.",
  "Show details": "نمایش جزئیات",
  "Hide details": "پنهان کردن جزئیات",
  Dismiss: "بستن",
  "Technical details": "جزئیات فنی",
  "Measure latency": "اندازه‌گیری تأخیر",
  "Exit IP": "IP خروجی",
  "Browsers and apps that follow the Windows proxy settings.": "مرورگرها و برنامه‌هایی که از تنظیمات پراکسی ویندوز پیروی می‌کنند.",
  "Captures all traffic on this PC. Needs administrator rights.": "تمام ترافیک این رایانه را می‌گیرد. دسترسی ادمین لازم است.",
  "Only apps you point at the local port.": "فقط برنامه‌هایی که خودتان به پورت محلی وصل کنید.",
  "Disconnect to change the mode.": "برای تغییر حالت، اتصال را قطع کنید.",
  "Real HTTP delay through this server": "تأخیر واقعی HTTP از طریق این سرور",
  "TCP connect time": "زمان اتصال TCP",
  "Add a subscription link or paste a config.": "لینک اشتراک اضافه کنید یا کانفیگ را بچسبانید.",
  "Select a server first": "ابتدا یک سرور انتخاب کنید",

  // friendly errors
  "Something went wrong": "مشکلی پیش آمد",
  "Try again. If it keeps happening, open the Core log from the top bar.": "دوباره تلاش کنید. اگر تکرار شد، لاگ هسته را از نوار بالا باز کنید.",
  "This link isn't valid": "این لینک معتبر نیست",
  "A subscription link must start with http:// or https://.": "لینک اشتراک باید با http:// یا https:// شروع شود.",
  "Already added": "قبلاً اضافه شده",
  "This subscription is already in your list.": "این اشتراک از قبل در فهرست شما هست.",
  "The name is empty": "نام خالی است",
  "Type a name and try again.": "یک نام وارد کنید و دوباره تلاش کنید.",
  "You are not connected": "متصل نیستید",
  "Connect first, then run this again.": "اول وصل شوید، بعد دوباره امتحان کنید.",
  "A test is already running": "یک تست در حال اجراست",
  "Wait for it to finish.": "صبر کنید تا تمام شود.",
  "This engine isn't available yet": "این موتور هنوز در دسترس نیست",
  "Switch the engine to sing-box and try again.": "موتور را روی sing-box بگذارید و دوباره تلاش کنید.",
  "The port is already in use": "پورت از قبل در حال استفاده است",
  "Another app (such as Nekoray or v2rayN) is using it. Close that app or change the Local port in Settings.": "برنامه‌ی دیگری (مثل Nekoray یا v2rayN) از آن استفاده می‌کند. آن را ببندید یا پورت محلی را در تنظیمات عوض کنید.",
  "TUN mode needs administrator rights": "حالت TUN دسترسی ادمین می‌خواهد",
  "Close Vanish and run it as administrator, or switch to System Proxy mode.": "Vanish را ببندید و با Run as administrator اجرا کنید، یا حالت پراکسی سیستم را انتخاب کنید.",
  "The connection could not start": "اتصال برقرار نشد",
  "This server's config may be invalid. Try another server.": "ممکن است کانفیگ این سرور مشکل داشته باشد. سرور دیگری را امتحان کنید.",
  "The connection engine was not found": "موتور اتصال (sing-box) پیدا نشد",
  "Reinstall Vanish, or set the sing-box path in Settings.": "Vanish را دوباره نصب کنید یا مسیر sing-box را در تنظیمات بدهید.",
  "The engine is not responding": "موتور اتصال پاسخ نمی‌دهد",
  "Try again. If it keeps happening, check the Core log.": "دوباره تلاش کنید. اگر تکرار شد، لاگ هسته را ببینید.",
  "Could not change the Windows proxy": "تنظیم پراکسی ویندوز ممکن نشد",
  "Another program may be controlling proxy settings. Try TUN mode instead.": "ممکن است برنامه‌ی دیگری تنظیمات پراکسی را کنترل کند. حالت TUN را امتحان کنید.",
  "The subscription returned nothing": "اشتراک چیزی برنگرداند",
  "Check the link, and that the account on that panel is still active.": "لینک را بررسی کنید و ببینید اشتراک شما در آن پنل هنوز فعال است.",
  "No usable servers found": "سرور قابل استفاده‌ای پیدا نشد",
  "The list is empty or its format is not supported.": "فهرست خالی است یا قالب آن پشتیبانی نمی‌شود.",
  "The server refused access": "سرور اجازه‌ی دسترسی نداد",
  "The link may have expired or the token in it is wrong.": "ممکن است لینک منقضی شده یا توکن آن اشتباه باشد.",
  "Link not found": "لینک پیدا نشد",
  "Check the subscription link for typos.": "لینک اشتراک را از نظر اشتباه تایپی بررسی کنید.",
  "The server had a problem": "سرور مشکل دارد",
  "This is on the provider's side. Try again later.": "مشکل از طرف ارائه‌دهنده است. بعداً دوباره تلاش کنید.",
  "The speed test server is unreachable": "سرور تست سرعت در دسترس نیست",
  "Check your connection or try another server.": "اتصال را بررسی کنید یا سرور دیگری را امتحان کنید.",
  "Could not reach the server": "اتصال به سرور ممکن نشد",
  "Check your internet connection and the link. If the site is blocked, connect through a working server first.": "اینترنت و لینک را بررسی کنید. اگر سایت فیلتر است، اول با یک سرور سالم وصل شوید.",
  "Unsupported link": "لینک پشتیبانی نمی‌شود",
  "Supported: vless, vmess, trojan, ss, wireguard links, or sing-box JSON.": "پشتیبانی‌شده: لینک vless، vmess، trojan، ss، wireguard یا JSON خروجی sing-box.",
  "The config link looks damaged": "لینک کانفیگ خراب به نظر می‌رسد",
  "Copy it again in full and retry.": "آن را کامل دوباره کپی کنید و تلاش کنید.",
  "Could not detect the exit IP": "IP خروجی شناسایی نشد",
  "The lookup service did not answer. The connection itself may still work.": "سرویس جستجو جواب نداد. ممکن است خود اتصال سالم باشد.",
  "No server answered": "هیچ سروری پاسخ نداد",
  "All servers look unreachable. Check your internet connection or update the subscription.": "همه‌ی سرورها در دسترس نیستند. اینترنت را بررسی کنید یا اشتراک را به‌روزرسانی کنید.",
  "The update failed": "به‌روزرسانی انجام نشد",
  "Check your internet connection and try again.": "اینترنت را بررسی کنید و دوباره تلاش کنید.",
  "Could not save your changes": "تغییرات ذخیره نشد",
  "Vanish could not write to its data folder. Check disk space and permissions.": "Vanish نتوانست در پوشه‌ی داده‌هایش بنویسد. فضای دیسک و دسترسی‌ها را بررسی کنید.",
};

export function t(key: string, params?: Record<string, string | number>): string {
  let s = lang.value === "fa" ? (fa[key] ?? key) : key;
  if (params) s = s.replace(/\{(\w+)\}/g, (_, k: string) => String(params[k] ?? ""));
  return s;
}

// Sentences the Rust side produces for the leak test; they carry values (IPs, resolver names), so they are
// translated by pattern.
const faDetails: [RegExp, string][] = [
  [/^Not connected\. Showing your real network below\.$/, "متصل نیستید. شبکه‌ی واقعی شما در ادامه نشان داده می‌شود."],
  [/^Could not reach the lookup service\.$/, "اتصال به سرویس جستجو ممکن نشد."],
  [/^Exit (.+)$/, "خروجی $1"],
  [/^No answer through the proxy\. The node may be down\.$/, "از طریق پراکسی پاسخی نیامد. ممکن است کانفیگ از کار افتاده باشد."],
  [/^All traffic leaves through the tunnel\.$/, "تمام ترافیک از تونل خارج می‌شود."],
  [/^Direct connections still exit from (.+)\.$/, "اتصال‌های مستقیم هنوز از $1 خارج می‌شوند."],
  [/^Direct and tunnel IP are identical \(the node may be on this network\)\.$/, "IP مستقیم و IP تونل یکی است (شاید کانفیگ روی همین شبکه باشد)."],
  [/^Apps that ignore the proxy settings connect from (.+)\. Use TUN to cover them\.$/, "برنامه‌هایی که تنظیمات پراکسی را نادیده می‌گیرند از $1 وصل می‌شوند. برای پوشش آن‌ها از TUN استفاده کنید."],
  [/^Queries are answered by (.+)\.$/, "درخواست‌ها توسط $1 پاسخ داده می‌شوند."],
  [/^System DNS goes to (.+), not the tunnel's (.+)\.$/, "DNS سیستم به $1 می‌رود، نه $2 (سرور تونل)."],
  [/^Proxy-aware apps resolve via (.+); apps outside the proxy use (.+)\.$/, "برنامه‌های پراکسی‌دار از $1 استفاده می‌کنند؛ برنامه‌های بیرون از پراکسی از $2."],
  [/^Could not determine the DNS resolver\.$/, "سرور DNS مشخص نشد."],
  [/^IPv6 address (.+) is routed through the TUN interface\.$/, "آدرس IPv6 ‏$1 از طریق TUN عبور می‌کند."],
  [/^IPv6 is reachable outside the tunnel \((.+)\)\. Sites may see it instead of your exit IP\.$/, "IPv6 بیرون از تونل در دسترس است ($1). ممکن است سایت‌ها آن را به‌جای IP خروجی ببینند."],
  [/^No IPv6 connectivity, nothing to leak\.$/, "IPv6 فعال نیست، چیزی برای نشت وجود ندارد."],
  [/^(download|upload) failed: (.+)$/, "خطا در $1: $2"],
];

export interface FriendlyError {
  title: string;
  hint: string;
  /** The original message from the app, kept for the "technical details" view. */
  raw: string;
}

// Raw messages come from the Rust side (and the OS) in English. Each rule turns one family of them into a plain
// sentence plus what the user can do about it. The first match wins, so specific rules go before general ones.
const errorRules: [RegExp, string, string][] = [
  [/must start with http/i, "This link isn't valid", "A subscription link must start with http:// or https://."],
  [/already imported/i, "Already added", "This subscription is already in your list."],
  [/name cannot be empty/i, "The name is empty", "Type a name and try again."],
  [/^not connected|connect first/i, "You are not connected", "Connect first, then run this again."],
  [/already running/i, "A test is already running", "Wait for it to finish."],
  [/xray-core fallback/i, "This engine isn't available yet", "Switch the engine to sing-box and try again."],
  [/core exited[\s\S]*(address already in use|only one usage|bind:)/i, "The port is already in use", "Another app (such as Nekoray or v2rayN) is using it. Close that app or change the Local port in Settings."],
  [/core exited[\s\S]*(access is denied|permission denied|administrator|privilege|wintun|configure tun|create tun)/i, "TUN mode needs administrator rights", "Close Vanish and run it as administrator, or switch to System Proxy mode."],
  [/core exited/i, "The connection could not start", "This server's config may be invalid. Try another server."],
  [/cannot start .*(os error|cannot find|not found|no such file)/i, "The connection engine was not found", "Reinstall Vanish, or set the sing-box path in Settings."],
  [/did not become ready/i, "The engine is not responding", "Try again. If it keeps happening, check the Core log."],
  [/failed to set system proxy/i, "Could not change the Windows proxy", "Another program may be controlling proxy settings. Try TUN mode instead."],
  [/empty response from server/i, "The subscription returned nothing", "Check the link, and that the account on that panel is still active."],
  [/no usable nodes/i, "No usable servers found", "The list is empty or its format is not supported."],
  [/\(40[13]\b/, "The server refused access", "The link may have expired or the token in it is wrong."],
  [/\(404\b/, "Link not found", "Check the subscription link for typos."],
  [/\(5\d\d\b/, "The server had a problem", "This is on the provider's side. Try again later."],
  [/speed server unreachable/i, "The speed test server is unreachable", "Check your connection or try another server."],
  [/error sending request|dns error|connection (refused|reset|closed)|timed out|timeout|unreachable/i, "Could not reach the server", "Check your internet connection and the link. If the site is blocked, connect through a working server first."],
  [/unsupported link/i, "Unsupported link", "Supported: vless, vmess, trojan, ss, wireguard links, or sing-box JSON."],
  [/^(bad link|vmess:|ss:|wireguard:)/i, "The config link looks damaged", "Copy it again in full and retry."],
  [/exit ip lookup failed/i, "Could not detect the exit IP", "The lookup service did not answer. The connection itself may still work."],
  [/no node answered/i, "No server answered", "All servers look unreachable. Check your internet connection or update the subscription."],
  [/updat|signature|pubkey/i, "The update failed", "Check your internet connection and try again."],
  [/os error|permission|denied|no space|read-only/i, "Could not save your changes", "Vanish could not write to its data folder. Check disk space and permissions."],
];

/** Turns whatever a command rejected with into a title and a next step the user can act on. */
export function friendlyError(e: unknown): FriendlyError {
  const raw = String(e instanceof Error ? e.message : e).trim();
  for (const [re, title, hint] of errorRules) {
    if (re.test(raw)) return { title: t(title), hint: t(hint), raw };
  }
  return { title: t("Something went wrong"), hint: t("Try again. If it keeps happening, open the Core log from the top bar."), raw };
}

/** One-line form for places that only have room for a string. */
export function errorText(e: unknown): string {
  const f = friendlyError(e);
  return `${f.title}. ${f.hint}`;
}

export function tDetail(s: string): string {
  if (lang.value !== "fa") return s;
  for (const [re, out] of faDetails) {
    if (re.test(s)) {
      const r = s.replace(re, out);
      return /^(download|upload) failed/.test(s) ? r.replace("download", "دانلود").replace("upload", "آپلود") : r;
    }
  }
  return s;
}
