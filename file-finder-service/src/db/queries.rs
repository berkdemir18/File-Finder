use anyhow::Result;
use rusqlite::{params, Connection};
use shared::file_types::FileRecord;

/// Toplu dosya kaydı ekle (transaction içinde)
pub fn bulk_insert_files(conn: &Connection, records: &[FileRecord]) -> Result<usize> {
    let mut inserted = 0;
    let tx = conn.unchecked_transaction()?;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO files
             (name, extension, parent_path, full_path, size, created_time, modified_time,
              is_directory, volume_id, frn)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;

        for r in records {
            stmt.execute(params![
                r.name,
                r.extension,
                r.parent_path,
                r.full_path,
                r.size.map(|s| s as i64),
                r.created_time,
                r.modified_time,
                r.is_directory as i32,
                r.volume_id,
                r.frn,
            ])?;
            inserted += 1;
        }
    }

    tx.commit()?;
    Ok(inserted)
}

/// Dosya kaydını güncelle (USN Journal değişikliği)
pub fn upsert_file(conn: &Connection, record: &FileRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO files
         (name, extension, parent_path, full_path, size, created_time, modified_time,
          is_directory, volume_id, frn)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            record.name,
            record.extension,
            record.parent_path,
            record.full_path,
            record.size.map(|s| s as i64),
            record.created_time,
            record.modified_time,
            record.is_directory as i32,
            record.volume_id,
            record.frn,
        ],
    )?;
    Ok(())
}

/// Dosya kaydını sil (full_path ile)
pub fn delete_file_by_path(conn: &Connection, full_path: &str) -> Result<()> {
    conn.execute("DELETE FROM files WHERE full_path = ?1", params![full_path])?;
    Ok(())
}

/// Dosyayı yeniden adlandır (eski yol → yeni yol)
pub fn rename_file(conn: &Connection, old_path: &str, new_path: &str, new_name: &str) -> Result<()> {
    conn.execute(
        "UPDATE files SET full_path = ?1, name = ?2 WHERE full_path = ?3",
        params![new_path, new_name, old_path],
    )?;
    Ok(())
}

/// Basit metin arama (LIKE tabanlı — fuzzy için Rust tarafında işlenecek)
pub fn search_files_like(
    conn: &Connection,
    pattern: &str,
    extension_filter: Option<&str>,
    size_min: Option<i64>,
    size_max: Option<i64>,
    modified_after: Option<i64>,
    modified_before: Option<i64>,
    path_prefix: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<Vec<FileRecord>> {
    let like_pattern = format!("%{}%", pattern);

    // Dinamik SQL oluştur
    let mut sql = String::from(
        "SELECT id, name, extension, parent_path, full_path, size,
                created_time, modified_time, is_directory, volume_id, frn
         FROM files WHERE name LIKE ?1 ESCAPE '\\'",
    );

    let mut conditions: Vec<String> = Vec::new();
    if extension_filter.is_some() {
        conditions.push("extension = ?2".to_string());
    }
    if size_min.is_some() {
        conditions.push("size >= ?3".to_string());
    }
    if size_max.is_some() {
        conditions.push("size <= ?4".to_string());
    }
    if modified_after.is_some() {
        conditions.push("modified_time >= ?5".to_string());
    }
    if modified_before.is_some() {
        conditions.push("modified_time <= ?6".to_string());
    }
    if path_prefix.is_some() {
        conditions.push("parent_path LIKE ?7".to_string());
    }

    for cond in &conditions {
        sql.push_str(" AND ");
        sql.push_str(cond);
    }

    sql.push_str(" ORDER BY name LIMIT ?8 OFFSET ?9");

    let mut stmt = conn.prepare_cached(&sql)?;

    let path_prefix_pattern = path_prefix.map(|p| format!("{}%", p));

    let rows = stmt.query_map(
        rusqlite::params_from_iter([
            Some(like_pattern.clone()),
            extension_filter.map(|s| s.to_string()),
            size_min.map(|v| v.to_string()),
            size_max.map(|v| v.to_string()),
            modified_after.map(|v| v.to_string()),
            modified_before.map(|v| v.to_string()),
            path_prefix_pattern,
            Some(limit.to_string()),
            Some(offset.to_string()),
        ]
        .iter()
        .filter_map(|v| v.clone())),
        |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                extension: row.get(2)?,
                parent_path: row.get(3)?,
                full_path: row.get(4)?,
                size: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                created_time: row.get(6)?,
                modified_time: row.get(7)?,
                is_directory: row.get::<_, i32>(8)? != 0,
                volume_id: row.get(9)?,
                frn: row.get(10)?,
                parent_frn: None,
                score: 0.0,
            })
        },
    )?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }

    Ok(results)
}

/// Toplam dosya sayısını getir
pub fn count_files(conn: &Connection) -> Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files",
        [],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

/// Son kullanılan dosyaları getir
pub fn get_recent(conn: &Connection, limit: u32) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT file_path FROM recent_files ORDER BY accessed_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| row.get(0))?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Son kullanılan dosya güncelle
pub fn update_recent(conn: &Connection, path: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO recent_files(file_path, accessed_at, access_count)
         VALUES(?1, ?2, 1)
         ON CONFLICT(file_path) DO UPDATE SET
             accessed_at = excluded.accessed_at,
             access_count = access_count + 1",
        params![path, now],
    )?;
    Ok(())
}
