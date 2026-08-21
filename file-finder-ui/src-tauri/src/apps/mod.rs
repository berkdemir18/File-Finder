use anyhow::Result;
use log::{error, info, warn};
use std::path::Path;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON, SHGFI_TYPENAME,
};
use windows::core::{PCWSTR, Interface};

fn to_wstr(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Başlat Menüsü uygulamalarını tara
pub fn scan_start_menu() -> Result<Vec<crate::db::queries::AppRecord>> {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let mut apps = Vec::new();

    let start_menu_paths = [
        // Kullanıcı başlat menüsü
        std::env::var("APPDATA")
            .map(|p| format!(r"{}\Microsoft\Windows\Start Menu\Programs", p))
            .unwrap_or_default(),
        // Sistem genelinde başlat menüsü
        std::env::var("PROGRAMDATA")
            .map(|p| format!(r"{}\Microsoft\Windows\Start Menu\Programs", p))
            .unwrap_or_default(),
    ];

    for base in &start_menu_paths {
        let base_path = Path::new(base);
        if !base_path.exists() {
            continue;
        }
        scan_lnk_directory(base_path, &mut apps)?;
    }

    unsafe {
        CoUninitialize();
    }

    info!("Başlat Menüsü: {} uygulama tarandı.", apps.len());
    Ok(apps)
}

/// .lnk dosyalarını özyinelemeli tara
fn scan_lnk_directory(
    dir: &Path,
    apps: &mut Vec<crate::db::queries::AppRecord>,
) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_lnk_directory(&path, apps)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("lnk") {
            if let Some(app) = resolve_lnk(&path) {
                apps.push(app);
            }
        }
    }
    Ok(())
}

/// .lnk kısayolunu çöz (IShellLink COM arayüzü)
fn resolve_lnk(lnk_path: &Path) -> Option<crate::db::queries::AppRecord> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CLSCTX_INPROC_SERVER,
        IPersistFile, STGM,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    let name = lnk_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    if name.is_empty() {
        return None;
    }

    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(
            &ShellLink,
            None,
            CLSCTX_INPROC_SERVER,
        ).ok()?;

        let persist: IPersistFile = shell_link.cast().ok()?;

        let lnk_w: Vec<u16> = lnk_path
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        persist.Load(PCWSTR(lnk_w.as_ptr()), STGM(0)).ok()?;

        let mut target_buf = [0u16; 260];
        shell_link
            .GetPath(&mut target_buf, std::ptr::null_mut(), 0)
            .ok()?;

        let target = String::from_utf16_lossy(
            &target_buf[..target_buf.iter().position(|&c| c == 0).unwrap_or(260)],
        );

        if target.is_empty() || !Path::new(&target).exists() {
            return None;
        }

        Some(crate::db::queries::AppRecord {
            id: 0,
            name,
            exe_path: target,
            icon_path: Some(lnk_path.to_string_lossy().to_string()),
            source: "start_menu".to_string(),
            score: 0.0,
        })
    }
}

/// UWP / Windows Store uygulamalarını tara
pub fn scan_uwp_apps() -> Result<Vec<crate::db::queries::AppRecord>> {
    // PowerShell üzerinden Get-AppxPackage çıktısını al
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("powershell")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .args([
            "-NoProfile",
            "-Command",
            "Get-AppxPackage | Select-Object Name,InstallLocation,PackageFamilyName | ConvertTo-Json -Compress",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let json_str = String::from_utf8_lossy(&out.stdout);
            parse_uwp_json(&json_str)
        }
        Ok(out) => {
            warn!("Get-AppxPackage başarısız: {}", String::from_utf8_lossy(&out.stderr));
            Ok(vec![])
        }
        Err(e) => {
            warn!("PowerShell çalıştırılamadı: {}", e);
            Ok(vec![])
        }
    }
}

fn parse_uwp_json(json_str: &str) -> Result<Vec<crate::db::queries::AppRecord>> {
    let mut apps = Vec::new();

    let value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Ok(apps),
    };

    let array = match value {
        serde_json::Value::Array(a) => a,
        serde_json::Value::Object(_) => vec![value],
        _ => return Ok(apps),
    };

    for pkg in array {
        let name = pkg["Name"].as_str().unwrap_or("").to_string();
        let location = pkg["InstallLocation"].as_str().unwrap_or("").to_string();

        if name.is_empty() || location.is_empty() {
            continue;
        }

        // Sistem paketlerini filtrele
        if name.starts_with("Microsoft.Windows") || name.starts_with("Windows.") {
            continue;
        }

        apps.push(crate::db::queries::AppRecord {
            id: 0,
            name: prettify_uwp_name(&name),
            exe_path: location.clone(),
            icon_path: None,
            source: "uwp".to_string(),
            score: 0.0,
        });
    }

    info!("UWP: {} uygulama tarandı.", apps.len());
    Ok(apps)
}

/// UWP paket adını okunabilir hale getir
/// "Microsoft.VisualStudioCode" → "Visual Studio Code"
fn prettify_uwp_name(name: &str) -> String {
    let stripped = name
        .strip_prefix("Microsoft.")
        .unwrap_or(name);

    // CamelCase'i boşluklara böl
    let mut result = String::new();
    for (i, c) in stripped.char_indices() {
        if i > 0 && c.is_uppercase() {
            result.push(' ');
        }
        result.push(c);
    }
    result
}
