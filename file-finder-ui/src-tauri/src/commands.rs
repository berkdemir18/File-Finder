use anyhow::Result;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;

use crate::db::queries::{AppRecord, FileRecord};
use crate::search::{query_parser, SearchResponse, SearchResult, ResultType};
use crate::state::AppState;

// ─── Arama ─────────────────────────────────────────────────────────────────

/// Rust tarafında skorlanacak aday sayısı. SQL yalnız aday toplar; sıralamayı
/// fuzzy skor yapar, o yüzden havuzun sayfa boyutundan çok daha geniş olması
/// gerekir — yoksa alaka sıralaması yalnız havuzun içinde doğru olur.
const CANDIDATE_POOL: u32 = 2000;

#[tauri::command]
pub async fn search(
    query: String,
    page: u32,
    page_size: u32,
    state: State<'_, AppState>,
) -> Result<SearchResponse, String> {
    let start = std::time::Instant::now();

    // Sorguyu ayrıştır
    let parsed = query_parser::parse(&query);

    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Veritabanı henüz hazır değil")?;

    // Metin de filtre de yoksa arayacak bir şey yok. Ama yalnız filtre
    // verilmişse ("modified:today", "size:>1gb") bu geçerli bir aramadır —
    // eskiden bu sorgular sessizce boş dönüyordu.
    let has_filter = !parsed.extensions.is_empty()
        || parsed.size_min.is_some()
        || parsed.size_max.is_some()
        || parsed.modified_after.is_some()
        || parsed.modified_before.is_some()
        || parsed.created_after.is_some()
        || parsed.created_before.is_some()
        || parsed.path_prefix.is_some();

    if parsed.text.is_empty() && !parsed.is_regex && !has_filter {
        return Ok(SearchResponse {
            items: vec![],
            total: 0,
            ranked: 0,
            pool_full: false,
            has_more: false,
            page,
            elapsed_ms: start.elapsed().as_millis() as u64,
            query_parsed: parsed,
        });
    }

    // SQLite araması
    let search_text = if parsed.is_regex {
        // Regex araması için tüm kayıtları al ve Rust tarafında filtrele
        // (SQLite regex desteği yok — opsiyonel extension eklenebilir)
        String::new()
    } else {
        parsed.text.clone()
    };

    let extension = parsed.extensions.first().map(|s| s.as_str());

    // Ek filtre yoksa isim indeksi tek başına yetiyor; 3 GB'lık tabloyu hiç
    // okumadan aday toplanabiliyor (2,6 sn → 0,9 sn). Filtre varsa o sütunlar
    // indekste olmadığı için tabloya inmek zorundayız.
    let can_use_name_index = !parsed.is_regex && !has_filter && !parsed.text.is_empty();

    // Aday havuzu her sayfada aynıdır; sayfalama sıralamadan SONRA yapılır,
    // yoksa 2. sayfa bambaşka bir kümeyi yeniden sıralar.
    let results: Vec<FileRecord> = if can_use_name_index {
        let names =
            crate::db::queries::search_candidate_names(&db.conn, &search_text, CANDIDATE_POOL)
                .map_err(|e| e.to_string())?;

        // Satırların tamamı yalnız aday havuzu için, birincil anahtarla
        // çekilir. Skorlama ve sıralama aşağıda ortak yoldan yürür.
        let ids: Vec<i64> = names.into_iter().map(|(id, _)| id).collect();
        crate::db::queries::fetch_files_by_ids(&db.conn, &ids).map_err(|e| e.to_string())?
    } else {
        crate::db::queries::search_files(
            &db.conn,
            &search_text,
            extension,
            parsed.size_min.map(|v| v as i64),
            parsed.size_max.map(|v| v as i64),
            parsed.modified_after,
            parsed.modified_before,
            parsed.path_prefix.as_deref(),
            CANDIDATE_POOL,
            0,
        )
        .map_err(|e| e.to_string())?
    };

    // Havuz dolduysa daha fazla eşleşme var demektir; arayüz sayıyı "2000+"
    // diye yazsın diye bunu ayrıca bildiriyoruz. Gerçek COUNT(*) ikinci bir
    // tam tarama demekti — arama süresini iki katına çıkarıyordu.
    let pool_full = results.len() as u32 >= CANDIDATE_POOL;

    // Erişim sayılarını al (ranking için)
    let access_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    // Regex filtresi
    let results = if parsed.is_regex {
        if let Some(pattern) = &parsed.regex_pattern {
            match regex::Regex::new(pattern) {
                Ok(re) => results.into_iter().filter(|r| re.is_match(&r.name)).collect(),
                Err(_) => vec![],
            }
        } else {
            vec![]
        }
    } else {
        results
    };

    // Fuzzy ranking
    let ranked = crate::search::ranking::rank_results(results, &parsed, &access_counts);

    // Eşik altını at. Eşik ayarlardan gelir — kısa sorgularda (1-2 harf) fuzzy
    // skoru anlamsız olduğu için eşik uygulanmaz.
    let threshold = if parsed.text.chars().count() > 2 {
        settings_f64(&state, "fuzzy_threshold", 0.3)
    } else {
        0.0
    };
    let mut filtered: Vec<FileRecord> =
        crate::search::ranking::filter_by_threshold(ranked, threshold);

    // Yalnız filtre verilmişse skorların hepsi sıfırdır; böyle sorgularda
    // anlamlı sıralama "en son değişen önce" olur.
    if parsed.text.is_empty() {
        filtered.sort_by(|a, b| b.modified_time.cmp(&a.modified_time));
    }

    let ranked_count = filtered.len() as u64;
    let skip = (page as usize).saturating_mul(page_size as usize);
    let has_more = ranked_count as usize > skip + page_size as usize;

    // Sayfalandır — sıralanmış listeden dilim al
    let paged: Vec<FileRecord> = filtered
        .into_iter()
        .skip(skip)
        .take(page_size as usize)
        .collect();

    // SearchResult'a dönüştür (match pozisyonları ile)
    let items: Vec<SearchResult> = paged
        .into_iter()
        .map(|r| {
            let match_positions = if !parsed.text.is_empty() {
                crate::search::fuzzy::fuzzy_match(&r.name.to_lowercase(), &parsed.text.to_lowercase())
                    .map(|m| m.positions)
                    .unwrap_or_default()
            } else {
                vec![]
            };

            let result_type = if r.is_directory {
                ResultType::Directory
            } else {
                ResultType::File
            };

            SearchResult {
                id: r.id,
                name: r.name,
                extension: r.extension,
                parent_path: r.parent_path,
                full_path: r.full_path,
                size: r.size,
                modified_time: r.modified_time,
                created_time: r.created_time,
                is_directory: r.is_directory,
                score: r.score,
                match_positions,
                result_type,
            }
        })
        .collect();

    Ok(SearchResponse {
        items,
        total: ranked_count,
        ranked: ranked_count,
        pool_full,
        has_more,
        page,
        elapsed_ms: start.elapsed().as_millis() as u64,
        query_parsed: parsed,
    })
}

/// Ayar cache'inden sayısal değer oku; yoksa varsayılana düş.
fn settings_f64(state: &State<'_, AppState>, key: &str, fallback: f64) -> f64 {
    state
        .settings
        .lock()
        .ok()
        .and_then(|s| s.get(key).and_then(|v| v.parse::<f64>().ok()))
        .unwrap_or(fallback)
}

// ─── Uygulama Arama ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_apps(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<AppRecord>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("DB hazır değil")?;
    crate::db::queries::search_apps(&db.conn, &query, 10).map_err(|e| e.to_string())
}

// ─── Dosya İşlemleri ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn open_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Son kullanılanları güncelle
    {
        let db_guard = state.db.lock().unwrap();
        if let Some(db) = db_guard.as_ref() {
            let _ = crate::db::queries::update_recent(&db.conn, &path);
        }
    }

    crate::fs_ops::open_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_file_location(path: String) -> Result<(), String> {
    crate::fs_ops::open_file_location(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_as_admin(path: String) -> Result<(), String> {
    crate::fs_ops::run_as_admin(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    crate::fs_ops::delete_to_recycle_bin(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_file(old_path: String, new_name: String) -> Result<String, String> {
    crate::fs_ops::rename_file(&old_path, &new_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn copy_to_clipboard(path: String) -> Result<(), String> {
    crate::fs_ops::copy_path_to_clipboard(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn move_file(source: String, destination: String) -> Result<(), String> {
    crate::fs_ops::move_file(&source, &destination).map_err(|e| e.to_string())
}

// ─── Son Kullanılanlar & Favoriler ──────────────────────────────────────────

#[tauri::command]
pub async fn get_recent(
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<crate::db::queries::FileRecord>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("DB hazır değil")?;
    crate::db::queries::get_recent(&db.conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_favorites(
    state: State<'_, AppState>,
) -> Result<Vec<crate::db::queries::FileRecord>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("DB hazır değil")?;
    crate::db::queries::get_favorites(&db.conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_favorite(
    path: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("DB hazır değil")?;
    crate::db::queries::toggle_favorite(&db.conn, &path).map_err(|e| e.to_string())
}

// ─── Ayarlar ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("DB hazır değil")?;

    let mut map = std::collections::HashMap::new();
    for key in crate::config::Config::SETTING_KEYS {
        if let Ok(Some(val)) = crate::db::queries::get_setting(&db.conn, key) {
            map.insert(key.to_string(), val);
        }
    }

    // Varsayılanları doldur
    map.entry("theme".to_string()).or_insert("dark".to_string());
    map.entry("hotkey".to_string())
        .or_insert(crate::config::Config::DEFAULT_HOTKEY.to_string());
    map.entry("max_results".to_string()).or_insert("50".to_string());
    map.entry("fuzzy_threshold".to_string()).or_insert("0.3".to_string());
    map.entry("animation_speed".to_string()).or_insert("normal".to_string());
    map.entry("hide_on_blur".to_string()).or_insert("false".to_string());
    map.entry("startup_with_windows".to_string()).or_insert("false".to_string());
    map.entry("show_preview".to_string()).or_insert("true".to_string());

    // Arama tarafının okuyabilmesi için cache'i tazele
    if let Ok(mut cache) = state.settings.lock() {
        *cache = map.clone();
    }

    Ok(map)
}

#[tauri::command]
pub async fn update_setting(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let db_guard = state.db.lock().unwrap();
        let db = db_guard.as_ref().ok_or("DB hazır değil")?;
        crate::db::queries::set_setting(&db.conn, &key, &value).map_err(|e| e.to_string())?;
    }

    if let Ok(mut cache) = state.settings.lock() {
        cache.insert(key.clone(), value.clone());
    }

    // Yan etkisi olan ayarları hemen uygula — kaydedip unutmuyoruz.
    if key == "startup_with_windows" {
        crate::fs_ops::set_run_at_startup(value == "true").map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ─── İndeks İstatistikleri ──────────────────────────────────────────────────

#[tauri::command]
pub async fn get_index_stats(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("DB hazır değil")?;

    let total = crate::db::queries::count_files(&db.conn).unwrap_or(0);
    let connected = *state.service_connected.lock().unwrap();
    let mode = state.index_mode.lock().unwrap().clone();

    Ok(serde_json::json!({
        "total_files": total,
        "service_connected": connected,
        "mode": format!("{:?}", mode),
        "last_updated": chrono::Utc::now().timestamp(),
    }))
}

// ─── Servis Yönetimi ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_service_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let connected = *state.service_connected.lock().unwrap();
    let mode = state.index_mode.lock().unwrap().clone();
    Ok(serde_json::json!({
        "connected": connected,
        "mode": format!("{:?}", mode),
    }))
}

#[tauri::command]
pub async fn install_service() -> Result<(), String> {
    // Servis binary yolunu bul ve install komutunu çalıştır
    let service_exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Dizin bulunamadı")?
        .join("file-finder-service.exe");

    if !service_exe.exists() {
        return Err("Servis binary bulunamadı. Kurulumu yeniden yapın.".to_string());
    }

    std::process::Command::new(&service_exe)
        .arg("install")
        .spawn()
        .map_err(|e| e.to_string())?
        .wait()
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ─── Arama Geçmişi ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_search_history(
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("DB hazır değil")?;
    crate::db::queries::get_search_history(&db.conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_search_history(
    query: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("DB hazır değil")?;
    crate::db::queries::add_search_history(&db.conn, &query).map_err(|e| e.to_string())
}

// ─── Pencere Yönetimi ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn hide_window(window: tauri::Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn show_window(window: tauri::Window) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
