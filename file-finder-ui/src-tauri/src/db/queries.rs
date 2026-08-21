use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Dosya kaydı (UI tarafı)
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
    pub frn: Option<i64>,
    pub score: f64,
}

/// Toplu dosya ekle
pub fn bulk_insert_files(conn: &Connection, records: &[FileRecord]) -> Result<usize> {
    let tx = conn.unchecked_transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO files
             (name, extension, parent_path, full_path, size, created_time,
              modified_time, is_directory, volume_id, frn)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
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
            count += 1;
        }
    }

    tx.commit()?;
    Ok(count)
}

/// Arama filtrelerinin ortak WHERE parçası.
/// `search_files` ile `count_matching_files` aynı sonuç kümesini görsün diye
/// tek yerden üretilir.
fn build_search_filter(
    query: &str,
    extension: Option<&str>,
    size_min: Option<i64>,
    size_max: Option<i64>,
    modified_after: Option<i64>,
    modified_before: Option<i64>,
    path_prefix: Option<&str>,
) -> (String, Vec<rusqlite::types::Value>) {
    use rusqlite::types::Value;

    // Sorgu kelimelere ayrılır ve her kelime ayrı bir LIKE olur. Tek parça
    // aransaydı "football manager 24" yazan kullanıcı hiçbir şey bulamazdı:
    // dosya adı "FootballManager24" olduğu için aradaki boşluklar eşleşmez.
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| t.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"))
        .collect();

    let mut params: Vec<Value> = Vec::new();
    let mut sql = String::from(" WHERE 1=1");

    if terms.is_empty() {
        // Metin yok, yalnız filtre var — tüm kayıtlar aday.
    } else {
        for term in &terms {
            sql.push_str(" AND name LIKE ? ESCAPE '\\'");
            params.push(Value::Text(format!("%{}%", term)));
        }
    }

    if let Some(ext) = extension {
        sql.push_str(" AND extension = ?");
        params.push(Value::Text(ext.to_string()));
    }
    if let Some(s_min) = size_min {
        sql.push_str(" AND size >= ?");
        params.push(Value::Integer(s_min));
    }
    if let Some(s_max) = size_max {
        sql.push_str(" AND size <= ?");
        params.push(Value::Integer(s_max));
    }
    if let Some(m_after) = modified_after {
        sql.push_str(" AND modified_time >= ?");
        params.push(Value::Integer(m_after));
    }
    if let Some(m_before) = modified_before {
        sql.push_str(" AND modified_time <= ?");
        params.push(Value::Integer(m_before));
    }
    if let Some(prefix) = path_prefix {
        let esc = prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        sql.push_str(" AND parent_path LIKE ? ESCAPE '\\'");
        params.push(Value::Text(format!("{}%", esc)));
    }

    (sql, params)
}

/// Aday sıralaması — yalnız `name` sütununu kullanır, o yüzden
/// `idx_files_name` kapsayıcı indeks olarak taranabilir.
const CANDIDATE_ORDER: &str = " ORDER BY CASE
             WHEN name = ? COLLATE NOCASE THEN 0
             WHEN name LIKE ? ESCAPE '\\' THEN 1
             ELSE 2 END,
           length(name), name COLLATE NOCASE
         LIMIT ? OFFSET ?";

fn order_params(query: &str) -> (rusqlite::types::Value, rusqlite::types::Value) {
    use rusqlite::types::Value;
    let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    (Value::Text(query.to_string()), Value::Text(format!("{}%", escaped)))
}

/// Hızlı yol: yalnız `(id, name)` çeker.
///
/// Ölçüm (4,98 milyon kayıtlı gerçek indeks, "%footer%"):
///   tüm sütunlar → 2,62 sn · yalnız id+name → 0,86 sn
/// Fark, 3 GB'lık tabloyu okumak yerine sadece isim indeksini taramaktan
/// geliyor. Sıralama da yalnız `name` kullandığı için indeks kapsayıcı kalır.
/// Bu yüzden ek filtre (uzantı/boyut/tarih/yol) varsa bu yol KULLANILAMAZ —
/// o sütunlar indekste yok, SQLite yine tabloya inmek zorunda kalır.
pub fn search_candidate_names(
    conn: &Connection,
    query: &str,
    limit: u32,
) -> Result<Vec<(i64, String)>> {
    use rusqlite::types::Value;

    let (where_sql, mut params) =
        build_search_filter(query, None, None, None, None, None, None);

    let sql = format!("SELECT id, name FROM files{}{}", where_sql, CANDIDATE_ORDER);

    let (exact, prefix) = order_params(query);
    params.push(exact);
    params.push(prefix);
    params.push(Value::Integer(limit as i64));
    params.push(Value::Integer(0));

    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Sıralama bittikten sonra yalnız gösterilecek satırları doldurur.
/// Gelen sıra korunur.
pub fn fetch_files_by_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<FileRecord>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = std::iter::repeat("?").take(ids.len()).collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id,name,extension,parent_path,full_path,size,
                created_time,modified_time,is_directory,volume_id,frn
         FROM files WHERE id IN ({})",
        placeholders
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), row_to_file_record)?;

    let mut by_id = std::collections::HashMap::new();
    for r in rows {
        let rec: FileRecord = r?;
        by_id.insert(rec.id, rec);
    }

    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

fn row_to_file_record(row: &rusqlite::Row) -> rusqlite::Result<FileRecord> {
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
        score: 0.0,
    })
}

/// Dosyaları arama.
///
/// SQLite yalnız *aday* toplar; asıl sıralama Rust tarafındaki fuzzy skorla
/// yapılır. Adaylar alfabetik değil, alaka ihtimaline göre seçilir (tam
/// eşleşme → baştan eşleşme → kısa isim), yoksa "footer" araması alfabenin
/// başındaki ilk N dosyada boğulur ve gerçek eşleşme hiç görünmez.
pub fn search_files(
    conn: &Connection,
    query: &str,
    extension: Option<&str>,
    size_min: Option<i64>,
    size_max: Option<i64>,
    modified_after: Option<i64>,
    modified_before: Option<i64>,
    path_prefix: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<Vec<FileRecord>> {
    use rusqlite::types::Value;

    let (where_sql, mut params) = build_search_filter(
        query, extension, size_min, size_max, modified_after, modified_before, path_prefix,
    );

    let mut sql = String::from(
        "SELECT id,name,extension,parent_path,full_path,size,
                created_time,modified_time,is_directory,volume_id,frn
         FROM files",
    );
    sql.push_str(&where_sql);
    sql.push_str(CANDIDATE_ORDER);

    let (exact, prefix) = order_params(query);
    params.push(exact);
    params.push(prefix);
    params.push(Value::Integer(limit as i64));
    params.push(Value::Integer(offset as i64));

    let mut stmt = conn.prepare_cached(&sql)?;

    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
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
            score: 0.0,
        })
    })?;

    let mut results = Vec::new();
    for r in rows {
        results.push(r?);
    }
    Ok(results)
}

/// Toplu uygulama ekle
pub fn bulk_insert_apps(conn: &Connection, records: &[AppRecord]) -> Result<usize> {
    let tx = conn.unchecked_transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO apps
             (name, exe_path, icon_path, source)
             VALUES (?1, ?2, ?3, ?4)",
        )?;

        for r in records {
            stmt.execute(params![
                r.name,
                r.exe_path,
                r.icon_path,
                r.source,
            ])?;
            count += 1;
        }
    }

    tx.commit()?;
    Ok(count)
}

/// Toplam dosya sayısı
pub fn count_files(conn: &Connection) -> Result<u64> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
    Ok(n as u64)
}

/// Son kullanılanlar
pub fn get_recent(conn: &Connection, limit: u32) -> Result<Vec<FileRecord>> {
    let mut stmt = conn.prepare_cached(
        "SELECT r.file_path, f.name, f.extension, f.parent_path, f.size,
                f.created_time, f.modified_time, f.is_directory
         FROM recent_files r
         LEFT JOIN files f ON f.full_path = r.file_path
         ORDER BY r.accessed_at DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit], |row| {
        let full_path: String = row.get(0)?;
        let name: Option<String> = row.get(1)?;
        Ok(FileRecord {
            id: 0,
            name: name.unwrap_or_else(|| {
                std::path::Path::new(&full_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&full_path)
                    .to_string()
            }),
            extension: row.get(2)?,
            parent_path: row.get(3).unwrap_or_default(),
            full_path,
            size: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
            created_time: row.get(5)?,
            modified_time: row.get(6)?,
            is_directory: row.get::<_, Option<i32>>(7)?.unwrap_or(0) != 0,
            volume_id: None,
            frn: None,
            score: 0.0,
        })
    })?;

    let mut results = Vec::new();
    for r in rows {
        results.push(r?);
    }
    Ok(results)
}

/// Son kullanılan güncelle
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

/// Favorileri getir
pub fn get_favorites(conn: &Connection) -> Result<Vec<FileRecord>> {
    let mut stmt = conn.prepare_cached(
        "SELECT f.file_path, fl.name, fl.extension, fl.parent_path, fl.size,
                fl.created_time, fl.modified_time, fl.is_directory
         FROM favorites f
         LEFT JOIN files fl ON fl.full_path = f.file_path
         ORDER BY f.added_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        let full_path: String = row.get(0)?;
        let name: Option<String> = row.get(1)?;
        Ok(FileRecord {
            id: 0,
            name: name.unwrap_or_else(|| {
                std::path::Path::new(&full_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&full_path)
                    .to_string()
            }),
            extension: row.get(2)?,
            parent_path: row.get(3).unwrap_or_default(),
            full_path,
            size: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
            created_time: row.get(5)?,
            modified_time: row.get(6)?,
            is_directory: row.get::<_, Option<i32>>(7)?.unwrap_or(0) != 0,
            volume_id: None,
            frn: None,
            score: 0.0,
        })
    })?;

    let mut results = Vec::new();
    for r in rows {
        results.push(r?);
    }
    Ok(results)
}

/// Favori ekle/çıkar
pub fn toggle_favorite(conn: &Connection, path: &str) -> Result<bool> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM favorites WHERE file_path = ?1",
            params![path],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)?;

    if exists {
        conn.execute("DELETE FROM favorites WHERE file_path = ?1", params![path])?;
        Ok(false)
    } else {
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT OR IGNORE INTO favorites(file_path, added_at) VALUES(?1, ?2)",
            params![path, now],
        )?;
        Ok(true)
    }
}

/// Ayar getir
pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get(0),
    );
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Ayar güncelle
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings(key, value) VALUES(?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

/// Uygulama ara
pub fn search_apps(conn: &Connection, query: &str, limit: u32) -> Result<Vec<AppRecord>> {
    let like_pat = format!("%{}%", query);
    let mut stmt = conn.prepare_cached(
        "SELECT id, name, exe_path, icon_path, source
         FROM apps WHERE name LIKE ?1
         ORDER BY name LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![like_pat, limit], |row| {
        Ok(AppRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            exe_path: row.get(2)?,
            icon_path: row.get(3)?,
            source: row.get(4)?,
            score: 0.0,
        })
    })?;

    let mut results = Vec::new();
    for r in rows {
        results.push(r?);
    }
    Ok(results)
}

/// Arama geçmişi getir
pub fn get_search_history(conn: &Connection, limit: u32) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT DISTINCT query FROM search_history
         ORDER BY searched_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |r| r.get(0))?;
    let mut results = Vec::new();
    for r in rows {
        results.push(r?);
    }
    Ok(results)
}

/// Arama geçmişi ekle
pub fn add_search_history(conn: &Connection, query: &str) -> Result<()> {
    if query.trim().is_empty() {
        return Ok(());
    }
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO search_history(query, searched_at) VALUES(?1, ?2)",
        params![query, now],
    )?;
    Ok(())
}

/// Uygulama kaydı
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRecord {
    pub id: i64,
    pub name: String,
    pub exe_path: String,
    pub icon_path: Option<String>,
    pub source: String,
    pub score: f64,
}
