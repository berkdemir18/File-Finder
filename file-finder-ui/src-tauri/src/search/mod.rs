pub mod query_parser;
pub mod fuzzy;
pub mod ranking;

use query_parser::ParsedQuery;
use serde::{Deserialize, Serialize};

/// Arama isteği (frontend'den gelir)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub page: u32,
    pub page_size: u32,
}

/// Arama sonucu öğesi (frontend'e gider)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: i64,
    pub name: String,
    pub extension: Option<String>,
    pub parent_path: String,
    pub full_path: String,
    pub size: Option<u64>,
    pub modified_time: Option<i64>,
    pub created_time: Option<i64>,
    pub is_directory: bool,
    pub score: f64,
    /// Eşleşen karakter pozisyonları (highlight için)
    pub match_positions: Vec<usize>,
    /// Sonuç türü
    pub result_type: ResultType,
}

/// Sonuç türü
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResultType {
    File,
    Directory,
    App,
}

/// Toplam arama yanıtı
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub items: Vec<SearchResult>,
    /// Elde edilen eşleşme sayısı. `pool_full` true ise bu bir ALT sınırdır.
    pub total: u64,
    /// Skorlanıp sıralanan aday sayısı.
    pub ranked: u64,
    /// Aday havuzu doldu mu? Doluysa gerçek eşleşme sayısı daha fazladır ve
    /// arayüz sayıyı "2000+" diye yazar. Gerçek COUNT(*) ikinci bir tam
    /// tarama demek olurdu; arama süresini iki katına çıkarıyordu.
    pub pool_full: bool,
    /// Bu sayfadan sonra gösterilecek sonuç kaldı mı?
    pub has_more: bool,
    pub page: u32,
    pub elapsed_ms: u64,
    pub query_parsed: ParsedQuery,
}
