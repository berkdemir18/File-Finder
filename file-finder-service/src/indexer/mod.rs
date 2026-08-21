pub mod ntfs_scanner;
pub mod usn_journal;
pub mod mft_reader;

use anyhow::Result;
use log::{error, info, warn};
use shared::file_types::FileRecord;
use shared::ipc_types::{ChangeEvent, IndexingPhase, IndexingProgress, ServerMessage};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::db::Database;

/// Sistem genelinde indeksleme yöneticisi
pub struct Indexer {
    db: Arc<Mutex<Database>>,
    /// UI'ya event gönderimi için broadcast kanalı
    pub event_tx: broadcast::Sender<ServerMessage>,
    pub is_indexing: bool,
}

impl Indexer {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            db,
            event_tx: tx,
            is_indexing: false,
        }
    }

    /// Sistemdeki tüm NTFS ciltlerini tara ve indeksle
    pub async fn start_initial_indexing(&mut self) -> Result<()> {
        if self.is_indexing {
            warn!("İndeksleme zaten devam ediyor.");
            return Ok(());
        }
        self.is_indexing = true;

        let volumes = ntfs_scanner::get_ntfs_volumes()?;
        info!("Taranacak NTFS ciltleri: {:?}", volumes);

        for volume in &volumes {
            info!("Cilt taranıyor: {}", volume);
            let tx = self.event_tx.clone();
            let db = self.db.clone();
            let volume = volume.clone();

            tokio::task::spawn_blocking(move || {
                let result = ntfs_scanner::scan_volume_mft(&volume, |progress| {
                    let _ = tx.send(ServerMessage::IndexingProgress(progress));
                });

                match result {
                    Ok(records) => {
                        info!(
                            "{}: {} dosya tarandı, SQLite'a yazılıyor...",
                            volume,
                            records.len()
                        );

                        // Bloklanmayan tokio ortamında senkron SQLite işlemi
                        let db_guard = futures::executor::block_on(db.lock());
                        match crate::db::queries::bulk_insert_files(&db_guard.conn, &records) {
                            Ok(n) => info!("{}: {} kayıt eklendi.", volume, n),
                            Err(e) => error!("{}: SQLite insert hatası: {}", volume, e),
                        }
                    }
                    Err(e) => {
                        error!("{}: MFT tarama hatası: {}", volume, e);
                    }
                }
            })
            .await?;
        }

        self.is_indexing = false;
        info!("İlk indeksleme tamamlandı.");

        // USN Journal dinlemeyi başlat
        let volumes_for_usn = volumes;
        let db = self.db.clone();
        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            usn_journal::start_watching(volumes_for_usn, db, tx).await;
        });

        Ok(())
    }
}
