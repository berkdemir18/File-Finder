use log::{info, warn};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

const HOTKEY_ID: i32 = 1001;

/// Kullanıcının kombinasyonu tutmazsa denenecek yedekler.
/// Alt+Space bilerek en sonda: Windows'ta sistem pencere menüsünü açar.
const FALLBACKS: [&str; 3] = ["Ctrl+Space", "Ctrl+Shift+Space", "Alt+Space"];

/// "Ctrl+Shift+Space" → (MOD_CONTROL | MOD_SHIFT, VK_SPACE)
fn parse_hotkey(spec: &str) -> Option<(HOT_KEY_MODIFIERS, u32)> {
    let mut modifiers = MOD_NOREPEAT;
    let mut vk: Option<u32> = None;

    for part in spec.split('+') {
        let token = part.trim().to_ascii_lowercase();
        match token.as_str() {
            "" => continue,
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "alt" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            "win" | "super" | "meta" => modifiers |= MOD_WIN,
            "space" => vk = Some(VK_SPACE.0 as u32),
            other => {
                let mut chars = other.chars();
                match (chars.next(), chars.next()) {
                    // Tek harf/rakam: sanal tuş kodu ASCII büyük harfle aynı
                    (Some(c), None) if c.is_ascii_alphanumeric() => {
                        vk = Some(c.to_ascii_uppercase() as u32)
                    }
                    // F1..F12 → VK_F1 = 0x70
                    (Some('f'), Some(_)) => {
                        let n: u32 = other[1..].parse().ok()?;
                        if (1..=12).contains(&n) {
                            vk = Some(0x6F + n);
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
        }
    }

    // Değiştirici tuşsuz kısayol tüm sistemde o tuşu ele geçirirdi.
    if modifiers == MOD_NOREPEAT {
        return None;
    }
    vk.map(|k| (modifiers, k))
}

/// Global kısayolu kaydet ve Windows mesaj döngüsünü çalıştır.
/// `spec` ayarlardan gelir; tutmazsa yedeklere düşer ve gerçekte hangisinin
/// bağlandığını `hotkey-registered` olayıyla arayüze bildirir — sessizce
/// başka bir tuşa bağlanıp kullanıcıyı şaşırtmaz.
pub fn register_global_hotkey(app: AppHandle, spec: String) {
    unsafe {
        let mut bound: Option<String> = None;

        for candidate in std::iter::once(spec.as_str()).chain(FALLBACKS.iter().copied()) {
            let Some((modifiers, vk)) = parse_hotkey(candidate) else {
                warn!("Kısayol çözümlenemedi: {}", candidate);
                continue;
            };
            if RegisterHotKey(HWND(std::ptr::null_mut()), HOTKEY_ID, modifiers, vk).is_ok() {
                bound = Some(candidate.to_string());
                break;
            }
            warn!("Kısayol kayıt edilemedi (başka uygulamada olabilir): {}", candidate);
        }

        match &bound {
            Some(combo) => info!("Global kısayol kaydedildi: {}", combo),
            None => warn!("Hiçbir global kısayol kaydedilemedi."),
        }
        let _ = app.emit("hotkey-registered", bound.clone());

        if bound.is_none() {
            return;
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0).as_bool() {
            if msg.message == WM_HOTKEY {
                toggle_window(&app);
            }
        }

        let _ = UnregisterHotKey(HWND(std::ptr::null_mut()), HOTKEY_ID);
    }
}

fn toggle_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);

    // Görünür ama arkadaysa gizlemek yerine öne getir — kısayola basan
    // kullanıcı pencereyi kapatmak değil, ona ulaşmak istiyor.
    if visible && focused {
        let _ = window.hide();
        return;
    }

    if let Err(e) = position_on_active_monitor(&window) {
        warn!("Monitör konumlandırma hatası: {}", e);
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("window-shown", ());
}

/// İmlecin bulunduğu monitörde, üst üçte bire hizalı konumlandır.
fn position_on_active_monitor(window: &tauri::WebviewWindow) -> anyhow::Result<()> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    unsafe {
        let mut cursor_pos = POINT { x: 0, y: 0 };
        GetCursorPos(&mut cursor_pos)?;

        let monitor = MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut monitor_info).ok()?;

        let work_area = monitor_info.rcWork;
        let monitor_width = work_area.right - work_area.left;
        let monitor_height = work_area.bottom - work_area.top;

        // Sabit ölçü yerine pencerenin kendi boyutu — kullanıcı yeniden
        // boyutlandırdıysa da ortada kalır.
        let size = window.outer_size()?;
        let window_width = size.width as i32;
        let window_height = size.height as i32;

        let x = work_area.left + (monitor_width - window_width) / 2;
        let y = work_area.top + ((monitor_height - window_height) / 3).max(0);

        window.set_position(tauri::PhysicalPosition::new(x, y))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_combos() {
        assert!(parse_hotkey("Ctrl+Space").is_some());
        assert!(parse_hotkey("Ctrl+Shift+F").is_some());
        assert!(parse_hotkey("Alt+F12").is_some());
    }

    #[test]
    fn rejects_modifierless_and_garbage() {
        assert!(parse_hotkey("Space").is_none());
        assert!(parse_hotkey("Ctrl+Nope").is_none());
        assert!(parse_hotkey("Ctrl+F99").is_none());
    }
}
