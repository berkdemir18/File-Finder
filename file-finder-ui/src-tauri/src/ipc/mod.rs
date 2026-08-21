pub mod pipe_client;
pub use pipe_client::ServiceClient;

use serde::{Deserialize, Serialize};
use tauri::Emitter;

/// İndeks çalışma modu (UI tarafı mirror)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndexMode {
    MftFull,
    Fallback,
}

/// Servis bağlantısını başlat
pub async fn init_service_connection(
    app: tauri::AppHandle,
    state: crate::state::AppState,
) {
    log::info!("Servis bağlantısı deneniyor...");

    // Named pipe'a bağlanmayı dene
    match ServiceClient::connect(crate::config::Config::PIPE_NAME) {
        Ok(client) => {
            log::info!("Servise bağlandı! MFT modu aktif.");
            *state.service_connected.lock().unwrap() = true;
            *state.index_mode.lock().unwrap() = IndexMode::MftFull;
        }
        Err(e) => {
            log::warn!("Servis bağlantısı başarısız: {}. Fallback moduna geçiliyor.", e);
            *state.service_connected.lock().unwrap() = false;
            *state.index_mode.lock().unwrap() = IndexMode::Fallback;

            // Fallback modu başlat
            let state2 = state.clone();
            let app2 = app.clone();
            tokio::spawn(async move {
                crate::fallback::start_fallback_indexing(state2, app2).await;
            });
        }
    }

    // Servis bağlantısı event'ini frontend'e gönder
    let connected = *state.service_connected.lock().unwrap();
    let _ = app.emit("service-status-changed", serde_json::json!({
        "connected": connected,
        "mode": if connected { "MftFull" } else { "Fallback" }
    }));
}
