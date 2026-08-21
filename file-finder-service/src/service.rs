use anyhow::Result;
use log::{error, info};
use std::ffi::OsString;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;
use windows::Win32::System::Services::{
    ChangeServiceConfigW, CloseServiceHandle, ControlService, CreateServiceW,
    DeleteService, OpenSCManagerW, OpenServiceW, QueryServiceStatus, StartServiceW,
    SERVICE_ALL_ACCESS, SERVICE_AUTO_START, SERVICE_CONTROL_STOP, SERVICE_DEMAND_START,
    SERVICE_ERROR_NORMAL, SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_STATUS,
    SERVICE_STOP, SERVICE_STOPPED, SERVICE_WIN32_OWN_PROCESS,
    SC_MANAGER_ALL_ACCESS,
};

const SERVICE_NAME: &str = "FileFinderService";
const SERVICE_DISPLAY_NAME: &str = "File Finder Indexing Service";
const SERVICE_DESCRIPTION: &str =
    "File Finder için NTFS MFT tabanlı dosya indeksleme ve gerçek zamanlı izleme servisi.";

/// Windows Service olarak başlat (SCM dispatcher)
pub fn run() -> Result<()> {
    // Windows Service olarak kayıt
    // Not: Gerçek Windows Service dağıtımı için windows-service crate kullanılacak
    // Şimdilik doğrudan mantığı çalıştırıyoruz
    info!("ServiceMain başlatıldı.");
    crate::run_service_logic()
}

/// Servisi Windows SCM'e kaydet
pub fn install_service() -> Result<()> {
    let exe_path = std::env::current_exe()?;
    let exe_str = exe_path.to_str().ok_or_else(|| anyhow::anyhow!("Geçersiz exe yolu"))?;

    // Binary path: "C:\...\file-finder-service.exe"
    let binary_path = format!("\"{}\"", exe_str);

    unsafe {
        // SCM'i aç
        let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)?;

        let service_name: Vec<u16> = SERVICE_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let display_name: Vec<u16> = SERVICE_DISPLAY_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let binary_path_w: Vec<u16> = binary_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        // Servis zaten var mı kontrol et
        let existing = OpenServiceW(scm, PCWSTR(service_name.as_ptr()), SERVICE_QUERY_STATUS);
        if let Ok(svc) = existing {
            CloseServiceHandle(svc)?;
            CloseServiceHandle(scm)?;
            info!("Servis zaten kurulu.");
            return Ok(());
        }

        // Servisi oluştur
        let svc = CreateServiceW(
            scm,
            PCWSTR(service_name.as_ptr()),
            PCWSTR(display_name.as_ptr()),
            SERVICE_ALL_ACCESS,
            SERVICE_WIN32_OWN_PROCESS,
            SERVICE_AUTO_START,
            SERVICE_ERROR_NORMAL,
            PCWSTR(binary_path_w.as_ptr()),
            PCWSTR::null(), // load order group
            None,           // tag id
            PCWSTR::null(), // dependencies
            PCWSTR::null(), // service account (LocalSystem)
            PCWSTR::null(), // password
        )?;

        CloseServiceHandle(svc)?;
        CloseServiceHandle(scm)?;

        info!("Servis başarıyla kuruldu: {}", SERVICE_NAME);
    }

    Ok(())
}

/// Servisi SCM'den kaldır
pub fn uninstall_service() -> Result<()> {
    unsafe {
        let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)?;

        let service_name: Vec<u16> = SERVICE_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let svc = match OpenServiceW(scm, PCWSTR(service_name.as_ptr()), SERVICE_ALL_ACCESS) {
            Ok(s) => s,
            Err(e) => {
                CloseServiceHandle(scm)?;
                if e.code() == windows::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST.to_hresult() {
                    info!("Servis zaten kurulu değil.");
                    return Ok(());
                }
                return Err(e.into());
            }
        };

        // Çalışıyorsa durdur
        let mut status = SERVICE_STATUS::default();
        let _ = ControlService(svc, SERVICE_CONTROL_STOP, &mut status);

        // Sil
        DeleteService(svc)?;
        CloseServiceHandle(svc)?;
        CloseServiceHandle(scm)?;

        info!("Servis kaldırıldı: {}", SERVICE_NAME);
    }

    Ok(())
}
