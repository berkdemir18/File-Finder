use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local, NaiveDate, Utc};

/// Ayrıştırılmış sorgu yapısı
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedQuery {
    /// Ham metin (operatörler çıkarılmış)
    pub text: String,
    /// Uzantı filtreleri: ext:pdf,docx
    pub extensions: Vec<String>,
    /// Minimum boyut (byte)
    pub size_min: Option<u64>,
    /// Maksimum boyut (byte)
    pub size_max: Option<u64>,
    /// Değiştirilme tarihi başlangıcı (Unix saniye)
    pub modified_after: Option<i64>,
    /// Değiştirilme tarihi bitişi (Unix saniye)
    pub modified_before: Option<i64>,
    /// Oluşturulma tarihi başlangıcı
    pub created_after: Option<i64>,
    /// Oluşturulma tarihi bitişi
    pub created_before: Option<i64>,
    /// Yol öneki
    pub path_prefix: Option<String>,
    /// Regex sorgusu mu? (/pattern/ formatı)
    pub is_regex: bool,
    /// Regex pattern (is_regex=true ise)
    pub regex_pattern: Option<String>,
}

/// Sorgu string'ini ayrıştır
/// Örnekler:
/// "invoice ext:pdf size:>10mb modified:<7d"
/// "/^IMG_\d{4}\.jpg$/"
/// "report path:C:\Projects modified:2024-01-01..2024-12-31"
pub fn parse(query: &str) -> ParsedQuery {
    let query = query.trim();

    // Regex sorgu kontrolü: /pattern/
    if query.starts_with('/') && query.len() > 1 {
        if let Some(end) = query.rfind('/') {
            if end > 0 {
                let pattern = &query[1..end];
                return ParsedQuery {
                    text: String::new(),
                    is_regex: true,
                    regex_pattern: Some(pattern.to_string()),
                    ..Default::default()
                };
            }
        }
    }

    let mut parsed = ParsedQuery::default();
    let mut remaining_parts: Vec<&str> = Vec::new();

    for token in query.split_whitespace() {
        if let Some(rest) = token.strip_prefix("ext:") {
            // ext:pdf veya ext:pdf,docx,xlsx
            parsed.extensions = rest
                .split(',')
                .map(|e| e.trim_start_matches('.').to_lowercase())
                .filter(|e| !e.is_empty())
                .collect();
        } else if let Some(rest) = token.strip_prefix("size:") {
            parse_size_filter(rest, &mut parsed);
        } else if let Some(rest) = token.strip_prefix("modified:") {
            parse_date_filter(rest, &mut parsed.modified_after, &mut parsed.modified_before);
        } else if let Some(rest) = token.strip_prefix("created:") {
            parse_date_filter(rest, &mut parsed.created_after, &mut parsed.created_before);
        } else if let Some(rest) = token.strip_prefix("path:") {
            parsed.path_prefix = Some(rest.to_string());
        } else {
            remaining_parts.push(token);
        }
    }

    parsed.text = remaining_parts.join(" ");
    parsed
}

/// Boyut filtresini ayrıştır
/// Örnekler: >10mb, <1gb, 100kb-500kb, 50mb
fn parse_size_filter(s: &str, parsed: &mut ParsedQuery) {
    if let Some(rest) = s.strip_prefix('>') {
        parsed.size_min = parse_size_value(rest);
    } else if let Some(rest) = s.strip_prefix('<') {
        parsed.size_max = parse_size_value(rest);
    } else if let Some(idx) = s.find('-') {
        parsed.size_min = parse_size_value(&s[..idx]);
        parsed.size_max = parse_size_value(&s[idx + 1..]);
    } else {
        // Tam eşleşme (yaklaşık)
        let exact = parse_size_value(s);
        parsed.size_min = exact.map(|v| v.saturating_sub(v / 10));
        parsed.size_max = exact.map(|v| v + v / 10);
    }
}

/// "10mb" → Some(10 * 1024 * 1024)
fn parse_size_value(s: &str) -> Option<u64> {
    let s = s.to_lowercase();
    let (num_str, multiplier) = if let Some(n) = s.strip_suffix("gb") {
        (n, 1024u64 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("mb") {
        (n, 1024u64 * 1024)
    } else if let Some(n) = s.strip_suffix("kb") {
        (n, 1024u64)
    } else if let Some(n) = s.strip_suffix('b') {
        (n, 1u64)
    } else {
        (s.as_str(), 1u64)
    };

    num_str.trim().parse::<u64>().ok().map(|n| n * multiplier)
}

/// Tarih filtresini ayrıştır
/// Örnekler: today, yesterday, <7d, >30d, 2024-01-01, 2024-01-01..2024-12-31
fn parse_date_filter(s: &str, after: &mut Option<i64>, before: &mut Option<i64>) {
    let now = Utc::now().timestamp();
    let today_start = {
        let d = Local::now().date_naive();
        d.and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    };

    let s_lower = s.to_lowercase();

    if s_lower == "today" {
        *after = Some(today_start);
        *before = Some(now);
    } else if s_lower == "yesterday" {
        *after = Some(today_start - 86400);
        *before = Some(today_start);
    } else if let Some(rest) = s_lower.strip_prefix('<') {
        // <7d: son 7 gün
        if let Some(days) = parse_day_count(rest) {
            *after = Some(now - days * 86400);
        }
    } else if let Some(rest) = s_lower.strip_prefix('>') {
        // >30d: 30 günden önce
        if let Some(days) = parse_day_count(rest) {
            *before = Some(now - days * 86400);
        }
    } else if let Some(idx) = s.find("..") {
        // 2024-01-01..2024-12-31
        let start_str = &s[..idx];
        let end_str = &s[idx + 2..];
        *after = parse_date_str(start_str);
        *before = parse_date_str(end_str).map(|d| d + 86400); // bitiş gününü dahil et
    } else {
        // Tek tarih: modified:2024-01-15 → o günün tamamı
        if let Some(ts) = parse_date_str(s) {
            *after = Some(ts);
            *before = Some(ts + 86400);
        }
    }
}

fn parse_day_count(s: &str) -> Option<i64> {
    s.strip_suffix('d')
        .and_then(|n| n.parse::<i64>().ok())
}

fn parse_date_str(s: &str) -> Option<i64> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extensions() {
        let q = parse("report ext:pdf,docx");
        assert_eq!(q.text, "report");
        assert_eq!(q.extensions, vec!["pdf", "docx"]);
    }

    #[test]
    fn test_parse_size() {
        let q = parse("file size:>10mb");
        assert_eq!(q.text, "file");
        assert_eq!(q.size_min, Some(10 * 1024 * 1024));
    }

    #[test]
    fn test_parse_regex() {
        let q = parse(r"/^IMG_\d+\.jpg$/");
        assert!(q.is_regex);
        assert_eq!(q.regex_pattern.as_deref(), Some(r"^IMG_\d+\.jpg$"));
    }

    #[test]
    fn test_parse_combined() {
        let q = parse("invoice ext:pdf size:>1mb modified:<30d path:C:\\Documents");
        assert_eq!(q.text, "invoice");
        assert_eq!(q.extensions, vec!["pdf"]);
        assert!(q.size_min.is_some());
        assert!(q.modified_after.is_some());
        assert_eq!(q.path_prefix.as_deref(), Some("C:\\Documents"));
    }
}
