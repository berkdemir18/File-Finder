use serde::{Deserialize, Serialize};

/// Dosya veya dizin kaydı (indeks girişi)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: i64,
    pub name: String,
    pub extension: Option<String>,
    pub parent_path: String,
    pub full_path: String,
    pub size: Option<u64>,
    pub created_time: Option<i64>,
    pub modified_time: Option<i64>,
    pub is_directory: bool,
    pub volume_id: Option<String>,
    /// MFT File Reference Number
    pub frn: Option<i64>,
    /// MFT Parent File Reference Number
    pub parent_frn: Option<i64>,
    /// Arama relevans skoru (0.0 - 1.0)
    pub score: f64,
}

/// Uygulama kaydı (launcher için)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRecord {
    pub id: i64,
    pub name: String,
    pub exe_path: String,
    pub icon_path: Option<String>,
    /// 'start_menu' | 'uwp' | 'program_files'
    pub source: String,
    pub score: f64,
}

/// Favori kaydı
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteRecord {
    pub id: i64,
    pub file_path: String,
    pub added_at: i64,
}

/// Son kullanılan dosya kaydı
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRecord {
    pub id: i64,
    pub file_path: String,
    pub accessed_at: i64,
    pub access_count: i64,
}

/// Birleşik arama sonucu (dosya veya uygulama)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SearchResultItem {
    File(FileRecord),
    App(AppRecord),
}

impl SearchResultItem {
    pub fn score(&self) -> f64 {
        match self {
            SearchResultItem::File(f) => f.score,
            SearchResultItem::App(a) => a.score,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            SearchResultItem::File(f) => &f.name,
            SearchResultItem::App(a) => &a.name,
        }
    }
}
