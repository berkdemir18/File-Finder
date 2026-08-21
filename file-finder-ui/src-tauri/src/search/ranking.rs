use crate::db::FileRecord;
use crate::search::query_parser::ParsedQuery;
use super::fuzzy;

/// Arama sonuçlarını relevans skoruna göre sırala
/// Sıralama önceliği:
/// 1. Tam isim eşleşmesi (score = 1.0)
/// 2. İsim başından eşleşme (score ≥ 0.9)
/// 3. İsim içinde eşleşme (score ≥ 0.75)
/// 4. Fuzzy eşleşme (score < 0.75)
/// Aynı skor grubunda: daha kısa isim önce, sonra alfabetik
pub fn rank_results(
    mut results: Vec<FileRecord>,
    query: &ParsedQuery,
    access_counts: &std::collections::HashMap<String, i64>,
) -> Vec<FileRecord> {
    let search_text = &query.text;

    // Skor ata
    for r in &mut results {
        let fuzzy_score = fuzzy::score_with_fallback(&r.name, search_text);

        // Erişim sıklığı bonusu (logaritmik ölçek)
        let access_bonus = access_counts
            .get(&r.full_path)
            .copied()
            .unwrap_or(0);
        let access_score = if access_bonus > 0 {
            (access_bonus as f64).ln() * 0.02
        } else {
            0.0
        };

        // Yakın tarihte değiştirilme bonusu
        let recency_score = if let Some(modified) = r.modified_time {
            let age_days = (chrono::Utc::now().timestamp() - modified) / 86400;
            if age_days < 1 { 0.05 }
            else if age_days < 7 { 0.03 }
            else if age_days < 30 { 0.01 }
            else { 0.0 }
        } else {
            0.0
        };

        r.score = (fuzzy_score + access_score + recency_score).min(1.0);
    }

    // Sırala: skor azalan, eşit skorlarda isim uzunluğu artan
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.len().cmp(&b.name.len()))
            .then_with(|| a.name.cmp(&b.name))
    });

    results
}

/// Skor eşiğinin altındaki sonuçları filtrele
pub fn filter_by_threshold(results: Vec<FileRecord>, threshold: f64) -> Vec<FileRecord> {
    results.into_iter().filter(|r| r.score >= threshold).collect()
}
