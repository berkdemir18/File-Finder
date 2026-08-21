/// Fuzzy arama motoru
/// Sublime Text / VSCode command palette benzeri subsequence tabanlı skorlama
/// Örnek: "rprt q3" → "Report_Q3_Financials.xlsx" eşleşir

/// Fuzzy eşleşme sonucu
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    pub score: f64,
    /// Eşleşen karakter indeksleri (highlight için)
    pub positions: Vec<usize>,
}

/// Fuzzy eşleşme gerçekleştir
/// Döner: None eğer tüm karakterler bulunamazsa, Some(FuzzyMatch) bulunursa
pub fn fuzzy_match(text: &str, pattern: &str) -> Option<FuzzyMatch> {
    if pattern.is_empty() {
        return Some(FuzzyMatch {
            score: 0.5,
            positions: vec![],
        });
    }

    let text_lower: Vec<char> = text.to_lowercase().chars().collect();
    // Desendeki boşluklar ayraçtır, aranacak karakter değil. "rprt q3" deseni
    // "Report_Q3_Financials.xlsx" ile eşleşsin diye atılırlar — dosya adlarında
    // boşluğun yerini çoğu zaman alt çizgi veya CamelCase alır.
    let pattern_lower: Vec<char> = pattern
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let text_chars: Vec<char> = text.chars().collect();

    if pattern_lower.is_empty() {
        return Some(FuzzyMatch {
            score: 0.5,
            positions: vec![],
        });
    }

    let mut positions: Vec<usize> = Vec::with_capacity(pattern_lower.len());
    let mut ti = 0usize;
    let mut pi = 0usize;

    // Subsequence tarama
    while ti < text_lower.len() && pi < pattern_lower.len() {
        if text_lower[ti] == pattern_lower[pi] {
            positions.push(ti);
            pi += 1;
        }
        ti += 1;
    }

    // Tüm pattern karakterleri bulunamadıysa eşleşme yok
    if pi < pattern_lower.len() {
        return None;
    }

    // Skor hesapla
    let score = calculate_score(&text_chars, &text_lower, &pattern_lower, &positions);

    Some(FuzzyMatch { score, positions })
}

/// Kapsamlı skor hesaplama
/// Faktörler:
/// 1. Tam eşleşme: 1.0
/// 2. Başta eşleşme: 0.9
/// 3. Kelime başı eşleşme (CamelCase, alt çizgi, boşluk sonrası): bonus
/// 4. Ardışık karakter eşleşmesi: bonus
/// 5. Toplam metin uzunluğu cezası
fn calculate_score(
    text_chars: &[char],
    text_lower: &[char],
    pattern: &[char],
    positions: &[usize],
) -> f64 {
    let text_len = text_chars.len() as f64;
    let pattern_len = pattern.len() as f64;

    // Tam eşleşme
    if text_len == pattern_len && positions.iter().enumerate().all(|(i, &p)| p == i) {
        return 1.0;
    }

    // Baştan başlayan eşleşme
    let starts_at_zero = positions.first() == Some(&0);
    let all_consecutive = is_consecutive(positions);

    if starts_at_zero && all_consecutive {
        return 0.9;
    }

    let mut score = 0.0;
    let mut consecutive_bonus = 0.0;
    let mut prev_pos: Option<usize> = None;

    for (i, &pos) in positions.iter().enumerate() {
        let mut char_score = 1.0;

        // Ardışık karakter bonusu
        if let Some(prev) = prev_pos {
            if pos == prev + 1 {
                consecutive_bonus += 0.15;
                char_score += consecutive_bonus;
            } else {
                consecutive_bonus = 0.0;
            }
        }

        // Kelime başı bonusu (CamelCase, alt çizgi, boşluk, nokta sonrası)
        if pos == 0 || is_word_boundary(text_chars, pos) {
            char_score += 0.3;
        }

        // İlk pozisyon bonusu
        if pos == 0 {
            char_score += 0.2;
        }

        score += char_score;
        prev_pos = Some(pos);
    }

    // Normalize et
    let max_possible = pattern_len * (1.0 + 0.15 * pattern_len + 0.3 + 0.2);
    let normalized = score / max_possible;

    // Uzunluk cezası: daha kısa metin = daha iyi
    let length_penalty = (pattern_len / text_len).sqrt();

    (normalized * 0.7 + length_penalty * 0.3).min(0.85)
}

/// Tüm pozisyonlar ardışık mı?
fn is_consecutive(positions: &[usize]) -> bool {
    positions
        .windows(2)
        .all(|w| w[1] == w[0] + 1)
}

/// Kelime sınırı kontrolü (CamelCase, ayraç karakterleri)
fn is_word_boundary(chars: &[char], pos: usize) -> bool {
    if pos == 0 {
        return true;
    }
    let prev = chars[pos - 1];
    let curr = chars[pos];

    // Ayraç sonrası
    if matches!(prev, '_' | '-' | '.' | ' ' | '/' | '\\') {
        return true;
    }

    // CamelCase: küçükten büyüğe geçiş
    if prev.is_lowercase() && curr.is_uppercase() {
        return true;
    }

    false
}

/// Birden fazla sonucu fuzzy skoruna göre sırala
pub fn sort_by_fuzzy<T, F>(items: &mut Vec<T>, query: &str, name_fn: F)
where
    F: Fn(&T) -> &str,
{
    if query.is_empty() {
        return;
    }

    // Skor hesapla ve sakla
    let scores: Vec<f64> = items
        .iter()
        .map(|item| {
            let name = name_fn(item);
            score_with_fallback(name, query)
        })
        .collect();

    // İndekslerle birlikte sırala
    let mut indexed: Vec<(usize, f64)> = scores.into_iter().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let original: Vec<T> = std::mem::take(items);
    *items = indexed.into_iter().map(|(i, _)| {
        // SAFETY: her indeks bir kez kullanılır
        unsafe { std::ptr::read(&original[i]) }
    }).collect();
    std::mem::forget(original);
}

/// Skor hesapla, fuzzy olmayan öncelikli
pub fn score_with_fallback(text: &str, query: &str) -> f64 {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    // Tam eşleşme
    if text_lower == query_lower {
        return 1.0;
    }
    // Başta eşleşme
    if text_lower.starts_with(&query_lower) {
        return 0.9;
    }
    // İçeriyor
    if text_lower.contains(&query_lower) {
        return 0.75;
    }
    // Fuzzy
    fuzzy_match(&text_lower, &query_lower)
        .map(|m| m.score)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let m = fuzzy_match("report", "report");
        assert!(m.is_some());
        assert!((m.unwrap().score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_subsequence() {
        let m = fuzzy_match("Report_Q3_Financials.xlsx", "rprt q3");
        assert!(m.is_some());
    }

    #[test]
    fn test_no_match() {
        let m = fuzzy_match("hello", "xyz");
        assert!(m.is_none());
    }

    #[test]
    fn test_word_boundary_bonus() {
        let score1 = score_with_fallback("FileFinderApp", "ffa");
        let score2 = score_with_fallback("xfileyfinderapp", "ffa");
        assert!(score1 > score2, "Word boundary should give higher score");
    }
}
