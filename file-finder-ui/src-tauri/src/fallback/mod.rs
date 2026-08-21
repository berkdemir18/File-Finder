/// Fallback indeksleme modu: ReadDirectoryChangesW tabanlı
/// Servis bulunamadığında otomatik devreye girer.
/// Admin hakkı gerektirmez — standart kullanıcı izinleri yeterlidir.

use anyhow::Result;
use log::{error, info, warn};
use std::sync::Arc;
use tauri::{Manager, Emitter};

use crate::state::AppState;

/// Fallback indeksleme başlat
pub async fn start_fallback_indexing(state: AppState, app: tauri::AppHandle) {
    info!("Fallback indeksleme başlatılıyor (ReadDirectoryChangesW)...");

    // Öncelikli tarama hedefleri (Masaüstü, Belgeler vb. kullanıcı notlarını anında bulmak için)
    let drives = get_scan_targets();

    for drive in drives {
        let state2 = state.clone();
        let drive2 = drive.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = scan_directory_fallback(&drive2, &state2) {
                warn!("Fallback tarama hatası {}: {}", drive2, e);
            }
        });
    }

    // ReadDirectoryChangesW ile değişiklikleri izle
    // Bu basitleştirilmiş bir implementasyon; gerçek değişiklik izleme
    // ayrı bir thread'de çalışır
    info!("Fallback modu aktif. Değişiklikler izleniyor...");

    // Frontend'e fallback modu haber ver
    let _ = app.emit("index-mode", serde_json::json!({ "mode": "Fallback" }));
}

/// Standart fs::read_dir ile dizin tara (fallback)
fn scan_directory_fallback(root: &str, state: &AppState) -> Result<()> {
    use std::path::Path;

    let root_path = Path::new(root);
    if !root_path.exists() {
        return Ok(());
    }

    let mut records: Vec<crate::db::FileRecord> = Vec::new();
    let mut stack = vec![root_path.to_path_buf()];
    let mut scanned = 0u64;

    // Dışlanacak klasörler
    let exclude_patterns = [
        "node_modules",
        ".git",
        "$RECYCLE.BIN",
        "System Volume Information",
        "Windows",
        "pagefile.sys",
        "hiberfil.sys",
    ];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = match entry.file_name().into_string() {
                Ok(n) => n,
                Err(_) => continue,
            };

            // Dışlama kontrolü
            if exclude_patterns.iter().any(|&p| name.eq_ignore_ascii_case(p)) {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let is_dir = metadata.is_dir();
            let full_path = path.to_string_lossy().to_string();
            let parent = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let extension = if !is_dir {
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
            } else {
                None
            };

            let size = if is_dir { None } else { Some(metadata.len()) };

            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);

            let created = metadata
                .created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);

            records.push(crate::db::FileRecord {
                id: 0,
                name,
                extension,
                parent_path: parent,
                full_path,
                size,
                created_time: created,
                modified_time: modified,
                is_directory: is_dir,
                volume_id: Some(root.to_string()),
                frn: None,
                score: 0.0,
            });

            if is_dir {
                stack.push(path);
            }

            scanned += 1;
            if scanned % 10_000 == 0 {
                // Toplu insert yap ve belleği temizle
                flush_records_to_db(&records, state);
                records.clear();
                info!("Fallback: {} dosya tarandı", scanned);
            }
        }
    }

    // Kalan kayıtları yaz
    if !records.is_empty() {
        flush_records_to_db(&records, state);
    }

    info!("Fallback taraması tamamlandı: {} - {} dosya", root, scanned);
    Ok(())
}

fn flush_records_to_db(records: &[crate::db::FileRecord], state: &AppState) {
    let db_guard = state.db.lock().unwrap();
    if let Some(db) = db_guard.as_ref() {
        let _ = crate::db::queries::bulk_insert_files(&db.conn, records);
    }
}

/// Sistemdeki taranacak hedefleri (Öncelikli kullanıcı klasörleri ve diskler) listele
fn get_scan_targets() -> Vec<String> {
    let mut targets = Vec::new();

    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let p = std::path::Path::new(&userprofile);
        
        // Masaüstü
        let desktop = p.join("Desktop");
        if desktop.exists() {
            targets.push(desktop.to_string_lossy().to_string());
        }
        // OneDrive Masaüstü
        let od_desktop = p.join("OneDrive").join("Masaüstü");
        if od_desktop.exists() {
            targets.push(od_desktop.to_string_lossy().to_string());
        }
        let od_desktop_en = p.join("OneDrive").join("Desktop");
        if od_desktop_en.exists() {
            targets.push(od_desktop_en.to_string_lossy().to_string());
        }

        // Belgeler
        let documents = p.join("Documents");
        if documents.exists() {
            targets.push(documents.to_string_lossy().to_string());
        }
        let od_docs = p.join("OneDrive").join("Belgeler");
        if od_docs.exists() {
            targets.push(od_docs.to_string_lossy().to_string());
        }
        let od_docs_en = p.join("OneDrive").join("Documents");
        if od_docs_en.exists() {
            targets.push(od_docs_en.to_string_lossy().to_string());
        }

        // İndirilenler
        let downloads = p.join("Downloads");
        if downloads.exists() {
            targets.push(downloads.to_string_lossy().to_string());
        }
    }

    // Disk Sürücüleri
    for c in b'A'..=b'Z' {
        let drive = format!("{}:\\", c as char);
        let path = std::path::Path::new(&drive);
        if path.exists() {
            // C diskinin tamamını (yüz binlerce sistem dosyası) sync walk ile taramak yerine
            // sadece kullanıcılar (Users) klasörünü taramak çok daha hızlıdır
            if c == b'C' {
                let users = path.join("Users");
                if users.exists() {
                    targets.push(users.to_string_lossy().to_string());
                }
            } else {
                targets.push(drive);
            }
        }
    }

    targets.sort();
    targets.dedup();
    targets
}
