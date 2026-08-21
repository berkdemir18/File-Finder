mod commands;
mod state;
mod config;
mod ipc;
mod fallback;
mod search;
mod fs_ops;
mod apps;
mod hotkey;
mod tray;
mod db;

use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager, WindowEvent};
use log::{info, warn};

pub use commands::*;

/// Arayüz hazır sinyali gelmezse pencereyi yine de bu süre sonunda göster.
/// Böylece frontend patlasa bile kullanıcı görünmez bir sürece mahkûm kalmaz.
const STARTUP_SHOW_TIMEOUT_MS: u64 = 2_500;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let app_state = state::AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::open_file,
            commands::open_file_location,
            commands::run_as_admin,
            commands::delete_file,
            commands::rename_file,
            commands::copy_to_clipboard,
            commands::move_file,
            commands::get_recent,
            commands::get_favorites,
            commands::toggle_favorite,
            commands::get_settings,
            commands::update_setting,
            commands::get_index_stats,
            commands::get_search_history,
            commands::add_search_history,
            commands::get_apps,
            commands::hide_window,
            commands::show_window,
            commands::app_ready,
            commands::get_service_status,
            commands::install_service,
        ])
        .setup(|app| {
            info!("File Finder başlatılıyor...");

            // Kurulum ana iş parçacığında (pencere mesaj döngüsüyle aynı yerde)
            // çalışır. Burada yapılan her yavaş iş, pencerenin çizilip ama
            // taşınamadığı / küçültülemediği / kapatılamadığı o "donmuş
            // uygulama" hâline dönüşür. Bu yüzden burada SADECE hızlı işler
            // var: veritabanını aç, ayarları oku, tepsiyi kur. Uygulama
            // taraması ve indeksleme arka plana taşındı.
            {
                let state = app.state::<state::AppState>();

                let opened_db = db::Database::open()
                    .map_err(|e| format!("Veritabanı açılamadı: {e}"))?;
                opened_db
                    .init_schema()
                    .map_err(|e| format!("Veritabanı şeması kurulamadı: {e}"))?;

                // Ayar cache'ini doldur — arama ve pencere davranışı bunu okur.
                if let Ok(mut cache) = state.settings.lock() {
                    for key in config::Config::SETTING_KEYS {
                        if let Ok(Some(val)) = db::queries::get_setting(&opened_db.conn, key) {
                            cache.insert(key.to_string(), val);
                        }
                    }
                }

                *state
                    .db
                    .lock()
                    .map_err(|_| "Veritabanı kilidi bozuk".to_string())? = Some(opened_db);
            }

            let hotkey_spec = {
                let state = app.state::<state::AppState>();
                state
                    .settings
                    .lock()
                    .ok()
                    .and_then(|c| c.get("hotkey").cloned())
                    .unwrap_or_else(|| config::Config::DEFAULT_HOTKEY.to_string())
            };

            // Tray icon kur (hızlı, ana iş parçacığında olmalı)
            tray::setup_tray(app, &hotkey_spec)?;

            // Uygulama listesi taraması: Başlat menüsü yürüyüşü + PowerShell
            // Get-AppxPackage saniyeler sürüyor. Arka planda yapılır, sonuç
            // hazır olunca tek seferde yazılır.
            let scan_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut records = Vec::new();
                match apps::scan_start_menu() {
                    Ok(v) => records.extend(v),
                    Err(e) => warn!("Başlat menüsü taranamadı: {e}"),
                }
                match apps::scan_uwp_apps() {
                    Ok(v) => records.extend(v),
                    Err(e) => warn!("UWP uygulamaları taranamadı: {e}"),
                }

                let state = scan_handle.state::<state::AppState>();
                if let Ok(guard) = state.db.lock() {
                    if let Some(db) = guard.as_ref() {
                        let _ = db::queries::bulk_insert_apps(&db.conn, &records);
                    }
                }
                info!("Uygulama taraması bitti: {} kayıt", records.len());
                let _ = scan_handle.emit("apps-indexed", records.len());
            });

            // Servis bağlantısını başlat (arka planda)
            let app_handle = app.handle().clone();
            let state = app.state::<state::AppState>().inner().clone();
            tauri::async_runtime::spawn(async move {
                ipc::init_service_connection(app_handle, state).await;
            });

            // Global kısayol kur
            let app_handle2 = app.handle().clone();
            std::thread::spawn(move || {
                hotkey::register_global_hotkey(app_handle2, hotkey_spec);
            });

            // Pencere `visible: false` doğuyor; arayüz ilk karesini çizince
            // `app_ready` ile kendini gösteriyor. Sinyal gelmezse emniyet
            // zamanlayıcısı devreye girer.
            let show_handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(STARTUP_SHOW_TIMEOUT_MS));
                let state = show_handle.state::<state::AppState>();
                if state.startup_shown.swap(true, Ordering::SeqCst) {
                    return; // arayüz zaten gösterdi
                }
                warn!("Arayüz hazır sinyali gelmedi, pencere yine de açılıyor.");
                if let Some(window) = show_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Spotlight davranışı: odak kaybında pencere kendini toplar.
            // Varsayılan KAPALI — açıkken küçültme düğmesi pencereyi tamamen
            // yok ediyormuş gibi görünüyor (küçültmek zaten odağı kaybettirir),
            // bu da uygulamanın çöktüğü izlenimi veriyor.
            if let WindowEvent::Focused(false) = event {
                let app = window.app_handle();
                let hide = app
                    .state::<state::AppState>()
                    .settings
                    .lock()
                    .ok()
                    .map(|s| s.get("hide_on_blur").map(|v| v == "true").unwrap_or(false))
                    .unwrap_or(false);

                // Küçültülmüş pencereyi ayrıca gizleme: kullanıcı onu görev
                // çubuğundan geri istiyor, tepsiye kaçmasını değil.
                let minimized = window.is_minimized().unwrap_or(false);

                if hide && !minimized {
                    let _ = window.hide();
                }
            }

            // Çarpıya basmak uygulamayı sonlandırmasın; tepside beklesin.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("File Finder başlatılamadı");
}
