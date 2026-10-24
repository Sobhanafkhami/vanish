//! Traffic / expiry info of a subscription. Panels expose it in different ways, so two sources are read:
//!
//! 1. the de-facto standard `Subscription-Userinfo: upload=..; download=..; total=..; expire=..` header
//!    (Marzban, V2Board, Xboard, 3x-ui, Hiddify, Remnawave, most Clash/sing-box panels);
//! 2. "info nodes": many panels that send no header put the numbers into the remark of a node
//!    (e.g. `[ID:1] 📊[66.78GB] 📅[59Days]`, `剩余流量：12.3 GB`, `Remaining: 5GB`, `used 3GB / 100GB`).

use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use regex::Regex;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Usage {
    pub upload: Option<u64>,
    pub download: Option<u64>,
    /// `Some(0)` means unlimited.
    pub total: Option<u64>,
    pub remaining: Option<u64>,
    /// Unix seconds.
    pub expire: Option<u64>,
}

impl Usage {
    pub fn has_data(&self) -> bool {
        self.upload.is_some() || self.download.is_some() || self.total.is_some() || self.remaining.is_some() || self.expire.is_some()
    }

    fn has_traffic(&self) -> bool {
        self.upload.is_some() || self.download.is_some() || self.total.is_some() || self.remaining.is_some()
    }

    /// Fills fields this one lacks from `other`.
    fn fill_from(&mut self, other: &Usage) {
        if !self.has_traffic() {
            self.upload = other.upload;
            self.download = other.download;
            self.total = other.total;
            self.remaining = other.remaining;
        }
        self.expire = self.expire.or(other.expire);
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// `upload=1; download=2; total=3; expire=4` (values may also be written as floats or in exponent form).
pub fn from_header(h: &str) -> Usage {
    let mut u = Usage::default();
    for part in h.split(';') {
        let Some((k, v)) = part.split_once('=') else { continue };
        let Ok(v) = v.trim().parse::<f64>() else { continue };
        if v < 0.0 || !v.is_finite() {
            continue;
        }
        let v = v as u64;
        match k.trim().to_ascii_lowercase().as_str() {
            "upload" => u.upload = Some(v),
            "download" => u.download = Some(v),
            "total" => u.total = Some(v),
            "expire" => u.expire = Some(v).filter(|e| *e > 0),
            _ => {}
        }
    }
    u
}

/// Reads usage out of node remarks. The first value found for each field wins.
pub fn from_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Usage {
    let mut out = Usage::default();
    for name in names {
        let found = from_name(name);
        out.upload = out.upload.or(found.upload);
        out.download = out.download.or(found.download);
        out.total = out.total.or(found.total);
        out.remaining = out.remaining.or(found.remaining);
        out.expire = out.expire.or(found.expire);
    }
    out
}

/// Header first; anything it lacks (typically the expiry) is filled in from the remarks.
pub fn combine(header: Option<Usage>, remarks: Usage) -> Option<(Usage, &'static str)> {
    match header.filter(Usage::has_data) {
        Some(mut h) => {
            h.fill_from(&remarks);
            Some((h, "header"))
        }
        None if remarks.has_data() => Some((remarks, "remarks")),
        None => None,
    }
}

// ------------------------------------------------------------------ remark parsing

const AMOUNT: &str = r"(\d+(?:[.,]\d+)?)\s*([A-Za-z]{1,3}|گیگابایت|گیگ|مگابایت|ترابایت)";
const REMAINING_KW: &str = r"📊|剩余流量|剩余|剩|remaining|remain|left|باقی\s?مانده|باقیمانده|مانده";
const USED_KW: &str = r"已用流量|已用|已使用|used|usage|مصرف\s?شده|مصرفی|مصرف";
const TOTAL_KW: &str = r"总流量|总计|总量|total|حجم\s?کل|کل";

fn re(cell: &'static OnceLock<Regex>, pattern: String) -> &'static Regex {
    cell.get_or_init(|| Regex::new(&pattern).expect("static regex"))
}

fn unit_factor(unit: &str) -> Option<f64> {
    let u = unit.to_lowercase();
    Some(match u.as_str() {
        "b" => 1.0,
        "kb" | "kib" => 1024.0,
        "mb" | "mib" | "مگابایت" => 1024f64.powi(2),
        "gb" | "gib" | "گیگ" | "گیگابایت" => 1024f64.powi(3),
        "tb" | "tib" | "ترابایت" => 1024f64.powi(4),
        "pb" | "pib" => 1024f64.powi(5),
        _ => return None,
    })
}

fn to_bytes(num: &str, unit: &str) -> Option<u64> {
    let n: f64 = num.replace(',', ".").parse().ok()?;
    Some((n * unit_factor(unit)?) as u64)
}

/// First `<keyword> … <amount>` in `name`; amounts with an unknown unit (like "59Days") are skipped.
fn amount_after(name: &str, cell: &'static OnceLock<Regex>, keywords: &str) -> Option<u64> {
    let r = re(cell, format!(r"(?i)(?:{keywords})[^\d\n]{{0,16}}{AMOUNT}"));
    r.captures_iter(name).find_map(|c| to_bytes(&c[1], &c[2]))
}

fn from_name(name: &str) -> Usage {
    static REMAINING: OnceLock<Regex> = OnceLock::new();
    static USED: OnceLock<Regex> = OnceLock::new();
    static TOTAL: OnceLock<Regex> = OnceLock::new();
    static PAIR: OnceLock<Regex> = OnceLock::new();
    static UNLIMITED: OnceLock<Regex> = OnceLock::new();
    static DAYS_AFTER: OnceLock<Regex> = OnceLock::new();
    static DAYS_BEFORE: OnceLock<Regex> = OnceLock::new();
    static DATE: OnceLock<Regex> = OnceLock::new();

    let mut u = Usage::default();
    let remaining = amount_after(name, &REMAINING, REMAINING_KW);
    let used = amount_after(name, &USED, USED_KW);
    let total = amount_after(name, &TOTAL, TOTAL_KW);

    // "12.3GB / 100GB": used / total unless the remark says it is the remaining part.
    let pair = re(&PAIR, r"(?i)(\d+(?:[.,]\d+)?)\s*([A-Za-z]{1,3})?\s*/\s*(\d+(?:[.,]\d+)?)\s*([A-Za-z]{1,3})".into())
        .captures(name)
        .and_then(|c| {
            let unit_b = c.get(4)?.as_str();
            let a = to_bytes(&c[1], c.get(2).map_or(unit_b, |m| m.as_str()))?;
            Some((a, to_bytes(&c[3], unit_b)?))
        });
    let says_remaining = remaining.is_some();

    match (remaining, used, total, pair) {
        (r, us, t, None) => {
            u.remaining = r;
            u.download = us;
            u.total = t;
        }
        (r, us, t, Some((a, b))) => {
            u.total = t.or(Some(b));
            if says_remaining {
                u.remaining = r.or(Some(a));
            } else {
                u.download = us.or(Some(a));
            }
        }
    }
    if !u.has_traffic()
        && re(&UNLIMITED, r"(?i)(?:📊|流量|traffic|حجم)[^\n]{0,16}(?:不限|无限|unlimited|نامحدود)".into()).is_match(name)
    {
        u.total = Some(0);
    }

    // Expiry: "📅[59Days]", "到期 30 天", "Expires in 12 days", "59 days left", or an explicit date.
    let days = re(
        &DAYS_AFTER,
        r"(?i)(?:📅|⏳|⌛|到期|过期|expire[sd]?|expiry|exp\b|days?\s*left|انقضا|روز)[^\d\n]{0,16}(\d{1,5})\s*(?:days?|d\b|天|روز)".into(),
    )
    .captures(name)
    .or_else(|| re(&DAYS_BEFORE, r"(?i)(\d{1,5})\s*(?:days?|天|روز)\s*(?:left|remaining|remain|باقی)".into()).captures(name))
    .and_then(|c| c[1].parse::<u64>().ok());
    if let Some(d) = days {
        u.expire = Some(now() + d * 86_400);
    } else if let Some(c) = re(&DATE, r"(?i)(?:expire[sd]?|expiry|exp|到期|过期|انقضا|📅)[^\d\n]{0,16}(\d{4})[-/.](\d{1,2})[-/.](\d{1,2})".into()).captures(name) {
        let (y, m, d) = (c[1].parse().ok(), c[2].parse().ok(), c[3].parse().ok());
        if let (Some(y), Some(m), Some(d)) = (y, m, d) {
            u.expire = unix_from_civil(y, m, d);
        }
    }
    u
}

/// Midnight UTC of a calendar date (Howard Hinnant's days-from-civil).
fn unix_from_civil(y: i64, m: i64, d: i64) -> Option<u64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || !(1970..=2200).contains(&y) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn header_standard_and_float_forms() {
        let u = from_header("upload=10; download=20; total=100; expire=1800000000");
        assert_eq!((u.upload, u.download, u.total, u.expire), (Some(10), Some(20), Some(100), Some(1_800_000_000)));
        assert_eq!(from_header("expire=0").expire, None); // 0 = never expires
        assert_eq!(from_header("UPLOAD=0;DOWNLOAD=5;TOTAL=1.5e3").total, Some(1500));
    }

    #[test]
    fn info_node_with_emoji_remarks() {
        // Real shape from a V2Board panel: remaining traffic + days left, no header.
        let u = from_name("[ID:167221] 📊[66.78GB]  📅[59Days]");
        assert_eq!(u.remaining, Some((66.78 * GB as f64) as u64));
        assert!(u.expire.unwrap() > now() + 58 * 86_400 && u.expire.unwrap() < now() + 60 * 86_400);
        assert_eq!((u.download, u.total), (None, None));
    }

    #[test]
    fn remark_variants() {
        let u = from_name("剩余流量：12.5 GB");
        assert_eq!(u.remaining, Some((12.5 * GB as f64) as u64));
        let u = from_name("Used: 3GB / Total: 100GB");
        assert_eq!((u.download, u.total), (Some(3 * GB), Some(100 * GB)));
        let u = from_name("12.3GB/100GB");
        assert_eq!((u.download, u.total), (Some((12.3 * GB as f64) as u64), Some(100 * GB)));
        let u = from_name("Remaining 5GB / 100GB");
        assert_eq!((u.remaining, u.total), (Some(5 * GB), Some(100 * GB)));
        assert_eq!(from_name("Expire: 2030-01-01").expire, Some(1_893_456_000));
        assert_eq!(from_name("📊 Unlimited").total, Some(0));
    }

    #[test]
    fn ordinary_node_names_are_not_usage() {
        for n in ["🇩🇪 Germany LX52@Shahram_VPN", "US-West 1/2", "Left Server 3", "CDN-S1 vless 443"] {
            assert!(!from_name(n).has_data(), "{n}");
        }
    }

    #[test]
    fn header_wins_and_remarks_fill_gaps() {
        let remarks = from_names(["x", "[1] 📊[10GB] 📅[5Days]"]);
        let (u, src) = combine(Some(from_header("download=1;total=2")), remarks.clone()).unwrap();
        assert_eq!((src, u.total, u.remaining), ("header", Some(2), None));
        assert!(u.expire.is_some()); // gap filled
        assert_eq!(combine(None, remarks).unwrap().1, "remarks");
        assert!(combine(None, Usage::default()).is_none());
    }
}
