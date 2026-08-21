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

use tauri::{Manager, WindowEvent};
use log::info;

pub use commands::*;

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
            commands::get_service_status,
            commands::install_service,
        ])
        .setup(|app| {
            info!("File Finder başlatılıyor...");

            // Veritabanını başlat
            {
                let state = app.state::<state::AppState>();
                let mut db = state.db.lock().unwrap();
                let opened_db = db::Database::open().unwrap();
                opened_db.init_schema().unwrap();

                // Uygulamaları tara ve kaydet (Claude/Spotify vb. anında bulmak için)
                if let Ok(start_apps) = apps::scan_start_menu() {
                    let _ = db::queries::bulk_insert_apps(&opened_db.conn, &start_apps);
                }
                if let Ok(uwp_apps) = apps::scan_uwp_apps() {
                    let _ = db::queries::bulk_insert_apps(&opened_db.conn, &uwp_apps);
                }

                // Ayar cache'ini doldur — arama ve pencere davranışı bunu okur.
                if let Ok(mut cache) = state.settings.lock() {
                    for key in config::Config::SETTING_KEYS {
                        if let Ok(Some(val)) = db::queries::get_setting(&opened_db.conn, key) {
                            cache.insert(key.to_string(), val);
                        }
                    }
                }

                *db = Some(opened_db);
            }

            let hotkey_spec = {
                let state = app.state::<state::AppState>();
                let cache = state.settings.lock().unwrap();
                cache
                    .get("hotkey")
                    .cloned()
                    .unwrap_or_else(|| config::Config::DEFAULT_HOTKEY.to_string())
            };

            // Tray icon kur
            tray::setup_tray(app, &hotkey_spec)?;

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
