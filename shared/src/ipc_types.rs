use serde::{Deserialize, Serialize};

/// Mesajlar UI'dan Servise gönderilir
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// Dosya arama isteği
    Search {
        query: String,
        filters: SearchFilters,
        page: u32,
        page_size: u32,
    },
    /// Son kullanılan dosyaları getir
    GetRecent { limit: u32 },
    /// İndeks istatistiklerini getir
    GetIndexStats,
    /// Bağlantı testi
    Ping,
    /// Yeniden indeksleme başlat
    ReIndex,
}

/// Mesajlar Servisten UI'a gönderilir
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    /// Arama sonuçları
    SearchResults {
        items: Vec<crate::file_types::FileRecord>,
        total: u64,
        page: u32,
        elapsed_ms: u64,
    },
    /// İndeks istatistikleri
    IndexStats {
        total_files: u64,
        last_updated: i64,
        mode: IndexMode,
        indexing_progress: Option<IndexingProgress>,
    },
    /// Dosya sistemi değişiklik olayı
    IndexUpdated {
        path: String,
        event: ChangeEvent,
    },
    /// İndeksleme ilerleme durumu
    IndexingProgress(IndexingProgress),
    /// Ping yanıtı
    Pong,
    /// Hata mesajı
    Error { message: String },
}

/// Arama filtreleri
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    /// Dosya uzantısı filtresi (örn. ["pdf", "docx"])
    pub extensions: Vec<String>,
    /// Minimum dosya boyutu (byte)
    pub size_min: Option<u64>,
    /// Maksimum dosya boyutu (byte)
    pub size_max: Option<u64>,
    /// Değiştirilme tarihi başlangıcı (Unix timestamp saniye)
    pub modified_after: Option<i64>,
    /// Değiştirilme tarihi bitişi (Unix timestamp saniye)
    pub modified_before: Option<i64>,
    /// Oluşturulma tarihi başlangıcı
    pub created_after: Option<i64>,
    /// Oluşturulma tarihi bitişi
    pub created_before: Option<i64>,
    /// Belirli bir klasör altında ara
    pub path_prefix: Option<String>,
    /// Sadece dizinleri göster
    pub only_dirs: bool,
    /// Sadece dosyaları göster
    pub only_files: bool,
}

/// İndeks çalışma modu
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndexMode {
    /// Tam MFT erişimi (servis aktif)
    MftFull,
    /// ReadDirectoryChangesW fallback (servis yok)
    Fallback,
}

/// İndeksleme ilerleme durumu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingProgress {
    pub current: u64,
    pub total: u64,
    pub volume: String,
    pub phase: IndexingPhase,
}

/// İndeksleme aşaması
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexingPhase {
    Scanning,
    Inserting,
    Complete,
}

/// Dosya sistemi değişiklik olayı türü
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeEvent {
    Created,
    Deleted,
    Renamed { old_name: String },
    Modified,
}
