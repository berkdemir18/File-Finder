use anyhow::Result;
use rusqlite::Connection;

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self> {
        let db_path = crate::config::Config::db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -32768;
            PRAGMA temp_store = MEMORY;

            -- WAL dosyasını 64 MB'ta tut. Varsayılanda sınır YOKTUR: milyonlarca
            -- satır yazan ilk indeksleme turunda WAL gigabaytlara çıkıp diski
            -- dolduruyordu (gerçekte 3,9 GB'a ulaştı). journal_size_limit
            -- verilmezse SQLite checkpoint sonrası dosyayı küçültmez.
            PRAGMA journal_size_limit = 67108864;
        ")?;

        // Önceki sürümlerden kalan şişkin WAL'ı bir kez topla. Veri kaybı riski
        // yok: checkpoint WAL içeriğini ana dosyaya taşıyıp dosyayı keser.
        // Disk doluyken başarısız olabilir, o yüzden hatası yutuluyor.
        if let Err(e) = conn.pragma_update(None, "wal_checkpoint", "TRUNCATE") {
            log::warn!("WAL checkpoint yapılamadı: {}", e);
        }

        Ok(Self { conn })
    }

    pub fn init_schema(&self) -> Result<()> {
        // Servis DB şemasıyla aynı — paylaşılan SQLite dosyası
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS files (
                id            INTEGER PRIMARY KEY,
                name          TEXT NOT NULL,
                extension     TEXT,
                parent_path   TEXT NOT NULL,
                full_path     TEXT NOT NULL,
                size          INTEGER,
                created_time  INTEGER,
                modified_time INTEGER,
                is_directory  INTEGER NOT NULL DEFAULT 0,
                volume_id     TEXT,
                frn           INTEGER
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_files_full_path ON files(full_path);
            CREATE INDEX IF NOT EXISTS idx_files_name ON files(name COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_files_extension ON files(extension);
            CREATE INDEX IF NOT EXISTS idx_files_parent ON files(parent_path);
            CREATE INDEX IF NOT EXISTS idx_files_modified ON files(modified_time);

            CREATE TABLE IF NOT EXISTS apps (
                id         INTEGER PRIMARY KEY,
                name       TEXT NOT NULL,
                exe_path   TEXT NOT NULL UNIQUE,
                icon_path  TEXT,
                source     TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_apps_name ON apps(name COLLATE NOCASE);

            CREATE TABLE IF NOT EXISTS favorites (
                id        INTEGER PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                added_at  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS recent_files (
                id           INTEGER PRIMARY KEY,
                file_path    TEXT NOT NULL UNIQUE,
                accessed_at  INTEGER NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_recent_accessed ON recent_files(accessed_at DESC);

            CREATE TABLE IF NOT EXISTS search_history (
                id          INTEGER PRIMARY KEY,
                query       TEXT NOT NULL,
                searched_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_searched ON search_history(searched_at DESC);

            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        ")?;
        Ok(())
    }
}
