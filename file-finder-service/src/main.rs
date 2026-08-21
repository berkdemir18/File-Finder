mod service;
mod indexer;
mod ipc;
mod db;

use anyhow::Result;
use log::{error, info};

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    // Eğer komut satırından "install" veya "uninstall" argümanı gelirse servis yönetimi yap
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "install" => {
                info!("Servis kuruluyor...");
                return service::install_service();
            }
            "uninstall" => {
                info!("Servis kaldırılıyor...");
                return service::uninstall_service();
            }
            "run-direct" => {
                // Geliştirme/test için doğrudan çalıştır (servis olmadan)
                info!("Doğrudan mod başlatılıyor...");
                return run_service_logic();
            }
            _ => {}
        }
    }

    // Normal Windows Service başlatma
    info!("FileFinderService başlatılıyor...");
    service::run()
}

/// Servis ana mantığı — hem Windows Service hem doğrudan mod için
pub fn run_service_logic() -> Result<()> {
    // Tokio runtime oluştur
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?;

    rt.block_on(async {
        // Veritabanı başlat
        let db = db::Database::open()?;
        db.init_schema()?;
        info!("Veritabanı hazır.");

        // Paylaşımlı durum
        let db = std::sync::Arc::new(tokio::sync::Mutex::new(db));

        // İndeksleme motorunu başlat
        let indexer = indexer::Indexer::new(db.clone());
        let indexer = std::sync::Arc::new(tokio::sync::Mutex::new(indexer));

        // İlk indekslemeyi başlat
        {
            let idx = indexer.clone();
            tokio::spawn(async move {
                let mut idx = idx.lock().await;
                if let Err(e) = idx.start_initial_indexing().await {
                    error!("İlk indeksleme hatası: {}", e);
                }
            });
        }

        // Named Pipe sunucusunu başlat
        let pipe_server = ipc::PipeServer::new(db.clone(), indexer.clone());
        pipe_server.run().await
    })
}
