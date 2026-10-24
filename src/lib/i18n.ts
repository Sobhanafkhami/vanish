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
