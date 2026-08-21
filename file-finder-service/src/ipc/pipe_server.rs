use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use shared::ipc_types::{ClientMessage, IndexMode, IndexingPhase, IndexingProgress, ServerMessage};
use std::sync::Arc;
use tokio::sync::Mutex;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    FlushFileBuffers, ReadFile, WriteFile, PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe,
    PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
    PIPE_WAIT,
};
use windows::core::PCWSTR;

use crate::db::Database;
use crate::indexer::Indexer;

pub const PIPE_NAME: &str = r"\\.\pipe\FileFinderService";
const MAX_CONCURRENT_CLIENTS: usize = 8;

/// Named Pipe sunucusu
pub struct PipeServer {
    db: Arc<Mutex<Database>>,
    indexer: Arc<Mutex<Indexer>>,
}

impl PipeServer {
    pub fn new(db: Arc<Mutex<Database>>, indexer: Arc<Mutex<Indexer>>) -> Self {
        Self { db, indexer }
    }

    /// Sunucu döngüsünü başlat — her bağlantı için ayrı task
    pub async fn run(self) -> Result<()> {
        info!("Named Pipe sunucusu başlatıldı: {}", PIPE_NAME);

        let db = self.db.clone();
        let indexer = self.indexer.clone();

        loop {
            // Yeni pipe instance oluştur
            let pipe_handle = unsafe { create_pipe_instance()? };

            // Bağlantı bekle (blocking)
            let db_clone = db.clone();
            let indexer_clone = indexer.clone();

            let raw_handle = pipe_handle.0 as isize;
            let result = tokio::task::spawn_blocking(move || {
                let pipe_handle = windows::Win32::Foundation::HANDLE(raw_handle as *mut core::ffi::c_void);
                unsafe {
                    // ConnectNamedPipe bloklar — istemci bağlanana kadar bekler
                    let connect_result = ConnectNamedPipe(pipe_handle, None);
                    if connect_result.is_err() {
                        warn!("ConnectNamedPipe hatası, atlanıyor.");
                        let _ = CloseHandle(pipe_handle);
                        return;
                    }

                    info!("Yeni pipe istemcisi bağlandı.");

                    // İstemciyi işle (blocking loop)
                    handle_client(pipe_handle, db_clone, indexer_clone);

                    let _ = DisconnectNamedPipe(pipe_handle);
                    let _ = CloseHandle(pipe_handle);
                }
            });

            if let Err(e) = result.await {
                error!("Pipe client task hatası: {}", e);
            }
        }
    }
}

/// Pipe instance oluştur
unsafe fn create_pipe_instance() -> Result<HANDLE> {
    let pipe_name: Vec<u16> = PIPE_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let handle = CreateNamedPipeW(
        PCWSTR(pipe_name.as_ptr()),
        PIPE_ACCESS_DUPLEX,
        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
        PIPE_UNLIMITED_INSTANCES,
        64 * 1024,  // output buffer: 64KB
        64 * 1024,  // input buffer: 64KB
        0,          // default timeout
        None,       // security attributes (varsayılan = mevcut kullanıcı)
    );

    if handle == INVALID_HANDLE_VALUE {
        return Err(anyhow!("CreateNamedPipeW başarısız"));
    }

    Ok(handle)
}

/// İstemci bağlantısını işle (mesaj döngüsü)
fn handle_client(
    pipe: HANDLE,
    db: Arc<Mutex<Database>>,
    indexer: Arc<Mutex<Indexer>>,
) {
    let event_rx = {
        let idx = futures::executor::block_on(indexer.lock());
        idx.event_tx.subscribe()
    };

    loop {
        // Mesaj oku (4 byte uzunluk + JSON body)
        let msg = match read_message(pipe) {
            Ok(m) => m,
            Err(e) => {
                debug!("Pipe okuma hatası: {}", e);
                break;
            }
        };

        let client_msg: ClientMessage = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                warn!("Geçersiz istemci mesajı: {}", e);
                let err = ServerMessage::Error {
                    message: format!("Geçersiz mesaj formatı: {}", e),
                };
                let _ = write_message(pipe, &serde_json::to_string(&err).unwrap());
                continue;
            }
        };

        let response = handle_message(client_msg, &db, &indexer);

        match serde_json::to_string(&response) {
            Ok(json) => {
                if write_message(pipe, &json).is_err() {
                    break;
                }
            }
            Err(e) => {
                error!("Yanıt serileştirme hatası: {}", e);
                break;
            }
        }
    }
}

/// İstemci mesajını işle ve yanıt üret
fn handle_message(
    msg: ClientMessage,
    db: &Arc<Mutex<Database>>,
    indexer: &Arc<Mutex<Indexer>>,
) -> ServerMessage {
    match msg {
        ClientMessage::Ping => ServerMessage::Pong,

        ClientMessage::Search { query, filters, page, page_size } => {
            let db_guard = futures::executor::block_on(db.lock());
            let offset = page * page_size;

            let results = crate::db::queries::search_files_like(
                &db_guard.conn,
                &query,
                if filters.extensions.is_empty() {
                    None
                } else {
                    filters.extensions.first().map(|s| s.as_str())
                },
                filters.size_min.map(|v| v as i64),
                filters.size_max.map(|v| v as i64),
                filters.modified_after,
                filters.modified_before,
                filters.path_prefix.as_deref(),
                page_size,
                offset,
            );

            match results {
                Ok(mut items) => {
                    let total = items.len() as u64;
                    // Skor hesapla
                    for item in &mut items {
                        item.score = score_result(&item.name, &query);
                    }
                    // Skorlara göre sırala
                    items.sort_by(|a, b| {
                        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
                    });

                    ServerMessage::SearchResults {
                        items,
                        total,
                        page,
                        elapsed_ms: 0,
                    }
                }
                Err(e) => ServerMessage::Error {
                    message: format!("Arama hatası: {}", e),
                },
            }
        }

        ClientMessage::GetRecent { limit } => {
            let db_guard = futures::executor::block_on(db.lock());
            match crate::db::queries::get_recent(&db_guard.conn, limit) {
                Ok(paths) => {
                    let items: Vec<shared::file_types::FileRecord> = paths
                        .into_iter()
                        .map(|p| {
                            let name = std::path::Path::new(&p)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&p)
                                .to_string();
                            shared::file_types::FileRecord {
                                id: 0,
                                name,
                                extension: None,
                                parent_path: String::new(),
                                full_path: p,
                                size: None,
                                created_time: None,
                                modified_time: None,
                                is_directory: false,
                                volume_id: None,
                                frn: None,
                                parent_frn: None,
                                score: 0.0,
                            }
                        })
                        .collect();
                    ServerMessage::SearchResults {
                        items,
                        total: 0,
                        page: 0,
                        elapsed_ms: 0,
                    }
                }
                Err(e) => ServerMessage::Error {
                    message: format!("Son kullanılanlar hatası: {}", e),
                },
            }
        }

        ClientMessage::GetIndexStats => {
            let db_guard = futures::executor::block_on(db.lock());
            let total = crate::db::queries::count_files(&db_guard.conn).unwrap_or(0);
            let idx = futures::executor::block_on(indexer.lock());
            ServerMessage::IndexStats {
                total_files: total,
                last_updated: chrono::Utc::now().timestamp(),
                mode: IndexMode::MftFull,
                indexing_progress: None,
            }
        }

        ClientMessage::ReIndex => {
            let idx = indexer.clone();
            std::thread::spawn(move || {
                let mut idx_guard = futures::executor::block_on(idx.lock());
                let rt = tokio::runtime::Handle::current();
                let _ = rt.block_on(idx_guard.start_initial_indexing());
            });
            ServerMessage::Pong
        }
    }
}

/// Basit relevans skoru hesapla
fn score_result(name: &str, query: &str) -> f64 {
    let name_lower = name.to_lowercase();
    let query_lower = query.to_lowercase();

    if name_lower == query_lower {
        return 1.0;
    }
    if name_lower.starts_with(&query_lower) {
        return 0.9;
    }
    if name_lower.contains(&query_lower) {
        return 0.7;
    }
    // Fuzzy: subsequence matching
    fuzzy_score(&name_lower, &query_lower)
}

/// Subsequence tabanlı fuzzy skor (0.0 - 0.6 aralığında döner)
fn fuzzy_score(text: &str, pattern: &str) -> f64 {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    let mut ti = 0;
    let mut pi = 0;
    let mut matches = 0;
    let mut consecutive = 0;
    let mut score = 0.0;

    while ti < text_chars.len() && pi < pattern_chars.len() {
        if text_chars[ti] == pattern_chars[pi] {
            matches += 1;
            consecutive += 1;
            score += 1.0 + consecutive as f64 * 0.1;
            pi += 1;
        } else {
            consecutive = 0;
        }
        ti += 1;
    }

    if pi < pattern_chars.len() {
        return 0.0; // Tüm karakterler bulunamadı
    }

    let max_score = pattern_chars.len() as f64 * 1.5;
    (score / max_score * 0.6).min(0.6)
}

/// Pipe'dan 4-byte length-prefixed mesaj oku
fn read_message(pipe: HANDLE) -> Result<String> {
    let mut len_buf = [0u8; 4];
    let mut bytes_read: u32 = 0;

    unsafe {
        ReadFile(
            pipe,
            Some(&mut len_buf),
            Some(&mut bytes_read),
            None,
        )?;
    }

    if bytes_read != 4 {
        return Err(anyhow!("Eksik uzunluk header'ı"));
    }

    let len = u32::from_le_bytes(len_buf) as usize;
    if len > 10 * 1024 * 1024 {
        return Err(anyhow!("Mesaj çok büyük: {} byte", len));
    }

    let mut body = vec![0u8; len];
    let mut total_read = 0usize;

    while total_read < len {
        let mut chunk_read: u32 = 0;
        unsafe {
            ReadFile(
                pipe,
                Some(&mut body[total_read..]),
                Some(&mut chunk_read),
                None,
            )?;
        }
        total_read += chunk_read as usize;
    }

    Ok(String::from_utf8(body)?)
}

/// Pipe'a 4-byte length-prefixed mesaj yaz
fn write_message(pipe: HANDLE, msg: &str) -> Result<()> {
    let bytes = msg.as_bytes();
    let len = bytes.len() as u32;
    let len_buf = len.to_le_bytes();

    let mut bytes_written: u32 = 0;

    unsafe {
        WriteFile(pipe, Some(&len_buf), Some(&mut bytes_written), None)?;
        WriteFile(pipe, Some(bytes), Some(&mut bytes_written), None)?;
        FlushFileBuffers(pipe)?;
    }

    Ok(())
}
