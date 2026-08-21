use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;

/// Veritabanı yöneticisi
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Veritabanı dosyasını aç veya oluştur
    pub fn open() -> Result<Self> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        // WAL modu — çok daha hızlı eşzamanlı okuma/yazma
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -65536;  -- 64MB önbellek
            PRAGMA page_size = 4096;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 268435456; -- 256MB mmap
        ")?;

        Ok(Self { conn })
    }

    /// Veritabanı dosya yolu: %APPDATA%\FileFinder\index.db
    pub fn db_path() -> Result<PathBuf> {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| anyhow::anyhow!("APPDATA ortam değişkeni bulunamadı"))?;
        Ok(PathBuf::from(appdata).join("FileFinder").join("index.db"))
    }

    /// Şemayı başlat / migrate et
    pub fn init_schema(&self) -> Result<()> {
        crate::db::schema::create_tables(&self.conn)
    }
}
