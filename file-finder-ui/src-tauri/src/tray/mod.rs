use anyhow::Result;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

/// Pencereyi öne getir. `show()` tek başına yetmez: küçültülmüş pencere
/// görünür sayıldığı için tepsiden/kısayoldan çağrıldığında görev çubuğunda
/// kapalı kalıyordu — `unminimize` olmadan uygulama "açılmıyor" görünüyor.
pub fn reveal_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("window-shown", ());
}

/// System Tray iconunu kur. Kısayol etiketi ayardan gelir ki menü ile
/// gerçekte bağlı olan tuş birbirini tutsun.
pub fn setup_tray(app: &tauri::App, hotkey: &str) -> Result<()> {
    let show_item = MenuItem::with_id(app, "show", format!("Aç ({})", hotkey), true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "Gizle", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Ayarlar", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&show_item, &hide_item, &settings_item, &separator, &quit_item],
    )?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            anyhow::anyhow!("Uygulama ikonu bulunamadı")
        })?)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(format!("File Finder — {} ile aç", hotkey))
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                app.exit(0);
            }
            "show" => reveal_window(app),
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "settings" => {
                reveal_window(app);
                let _ = app.emit("open-settings", ());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Yalnız tuş bırakılınca tepki ver: basma + bırakma iki ayrı olay
            // geldiği için tek tıklama pencereyi açıp anında kapatıyordu.
            let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            else {
                return;
            };
            if button != MouseButton::Left || button_state != MouseButtonState::Up {
                return;
            }

            let app = tray.app_handle();
            let Some(window) = app.get_webview_window("main") else {
                return;
            };

            let visible = window.is_visible().unwrap_or(false);
            let minimized = window.is_minimized().unwrap_or(false);
            let focused = window.is_focused().unwrap_or(false);

            if visible && !minimized && focused {
                let _ = window.hide();
            } else {
                reveal_window(app);
            }
        })
        .build(app)?;

    Ok(())
}
