use anyhow::{anyhow, Result};
use log::info;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::PCWSTR;

fn to_wstr(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Dosyayı varsayılan uygulamayla aç
pub fn open_file(path: &str) -> Result<()> {
    let path_w = to_wstr(path);
    let open_w = to_wstr("open");

    unsafe {
        let result = ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR(open_w.as_ptr()),
            PCWSTR(path_w.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );

        // ShellExecuteW > 32 döndürürse başarılı
        if result.0 as isize <= 32 {
            return Err(anyhow!("ShellExecuteW başarısız: {}", result.0 as isize));
        }
    }

    info!("Dosya açıldı: {}", path);
    Ok(())
}

/// Dosyayı Explorer'da seçili olarak göster
pub fn open_file_location(path: &str) -> Result<()> {
    let args = format!("/select,\"{}\"", path);
    let explorer_w = to_wstr("explorer.exe");
    let args_w = to_wstr(&args);

    unsafe {
        let result = ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR::null(),
            PCWSTR(explorer_w.as_ptr()),
            PCWSTR(args_w.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );

        if result.0 as isize <= 32 {
            return Err(anyhow!("Explorer açılamadı: {}", result.0 as isize));
        }
    }

    Ok(())
}

/// Yönetici olarak çalıştır (kendi UAC promptunu gösterir)
pub fn run_as_admin(path: &str) -> Result<()> {
    let path_w = to_wstr(path);
    let runas_w = to_wstr("runas");

    unsafe {
        let result = ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR(runas_w.as_ptr()),
            PCWSTR(path_w.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );

        if result.0 as isize <= 32 {
            return Err(anyhow!("Yönetici olarak çalıştırılamadı: {}", result.0 as isize));
        }
    }

    Ok(())
}

/// Dosyayı Geri Dönüşüm Kutusu'na gönder (COM IFileOperation)
pub fn delete_to_recycle_bin(path: &str) -> Result<()> {
    use windows::Win32::UI::Shell::{
        IFileOperation, FileOperation, FOF_ALLOWUNDO, FOF_NOCONFIRMATION,
        FILEOPERATION_FLAGS,
    };
    use windows::Win32::System::Com::{CoInitializeEx, CoCreateInstance, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{IShellItem, SHCreateItemFromParsingName};

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;

        let file_op: IFileOperation = CoCreateInstance(
            &FileOperation,
            None,
            CLSCTX_ALL,
        )?;

        // FOFX_RECYCLEONDELETE | FOFX_SHOWELEVATIONPROMPT | FOF_ALLOWUNDO
        file_op.SetOperationFlags(FILEOPERATION_FLAGS(0x0040 | 0x0100 | 0x0200))?; // FOF_ALLOWUNDO | FOF_SILENT | FOF_NOCONFIRMATION

        let path_w = to_wstr(path);
        let item: IShellItem = SHCreateItemFromParsingName(
            PCWSTR(path_w.as_ptr()),
            None,
        )?;

        file_op.DeleteItem(&item, None)?;
        file_op.PerformOperations()?;

        CoUninitialize();
    }

    info!("Geri dönüşüme gönderildi: {}", path);
    Ok(())
}

/// Dosyayı yeniden adlandır
pub fn rename_file(old_path: &str, new_name: &str) -> Result<String> {
    let old = std::path::Path::new(old_path);
    let parent = old.parent().ok_or_else(|| anyhow!("Üst klasör bulunamadı"))?;
    let new_path = parent.join(new_name);

    std::fs::rename(old, &new_path)?;

    Ok(new_path.to_string_lossy().to_string())
}

/// Dosya yolunu panoya kopyala
pub fn copy_path_to_clipboard(path: &str) -> Result<()> {
    // Windows clipboard API ile metin kopyala
    use windows::Win32::System::DataExchange::{
        OpenClipboard, EmptyClipboard, SetClipboardData, CloseClipboard,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::CF_UNICODETEXT;

    let text_w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = text_w.len() * 2;

    unsafe {
        OpenClipboard(HWND(std::ptr::null_mut()))?;
        EmptyClipboard()?;

        let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len)?;
        let ptr = GlobalLock(hmem) as *mut u16;
        std::ptr::copy_nonoverlapping(text_w.as_ptr(), ptr, text_w.len());
        GlobalUnlock(hmem)?;

        SetClipboardData(CF_UNICODETEXT.0 as u32, windows::Win32::Foundation::HANDLE(hmem.0))?;
        CloseClipboard()?;
    }

    Ok(())
}

/// Dosyayı taşı
pub fn move_file(source: &str, destination: &str) -> Result<()> {
    let src = std::path::Path::new(source);
    let dst_dir = std::path::Path::new(destination);

    let file_name = src.file_name().ok_or_else(|| anyhow!("Dosya adı alınamadı"))?;
    let dst_file = dst_dir.join(file_name);

    std::fs::rename(src, dst_file)?;
    Ok(())
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "FileFinder";

/// "Windows ile başlat" — HKCU\...\Run altına kendi yolunu yazar/siler.
/// Yalnız kullanıcı kovanına yazdığı için yönetici izni gerekmez.
pub fn set_run_at_startup(enabled: bool) -> Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
    };

    let subkey = to_wstr(RUN_KEY);
    let value = to_wstr(RUN_VALUE);

    unsafe {
        let mut hkey = HKEY::default();
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        )
        .ok()?;

        let result = if enabled {
            let exe = std::env::current_exe()?;
            let command = format!("\"{}\"", exe.to_string_lossy());
            let data = to_wstr(&command);
            let bytes = std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * std::mem::size_of::<u16>(),
            );
            RegSetValueExW(hkey, PCWSTR(value.as_ptr()), 0, REG_SZ, Some(bytes)).ok()
        } else {
            // Kayıt zaten yoksa silme hatası bir sorun değil.
            let _ = RegDeleteValueW(hkey, PCWSTR(value.as_ptr()));
            Ok(())
        };

        let _ = RegCloseKey(hkey);
        result?;
    }

    info!("Windows ile başlat: {}", enabled);
    Ok(())
}
