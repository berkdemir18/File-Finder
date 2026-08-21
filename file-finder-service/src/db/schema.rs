use anyhow::Result;
use rusqlite::Connection;

/// Tüm tabloları ve indeksleri oluştur
pub fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch("
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

        CREATE UNIQUE INDEX IF NOT EXISTS idx_files_full_path
            ON files(full_path);

        CREATE INDEX IF NOT EXISTS idx_files_name
            ON files(name COLLATE NOCASE);

        CREATE INDEX IF NOT EXISTS idx_files_extension
            ON files(extension);

        CREATE INDEX IF NOT EXISTS idx_files_parent
            ON files(parent_path);

        CREATE INDEX IF NOT EXISTS idx_files_modified
            ON files(modified_time);

        CREATE INDEX IF NOT EXISTS idx_files_name_ext
            ON files(name, extension);

        CREATE TABLE IF NOT EXISTS apps (
            id         INTEGER PRIMARY KEY,
            name       TEXT NOT NULL,
            exe_path   TEXT NOT NULL,
            icon_path  TEXT,
            source     TEXT NOT NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_apps_exe
            ON apps(exe_path);

        CREATE INDEX IF NOT EXISTS idx_apps_name
            ON apps(name COLLATE NOCASE);

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

        CREATE INDEX IF NOT EXISTS idx_recent_accessed
            ON recent_files(accessed_at DESC);

        CREATE TABLE IF NOT EXISTS search_history (
            id          INTEGER PRIMARY KEY,
            query       TEXT NOT NULL,
            searched_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_history_searched
            ON search_history(searched_at DESC);

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );

        INSERT OR IGNORE INTO schema_version(version) VALUES(1);
    ")?;

    Ok(())
}
