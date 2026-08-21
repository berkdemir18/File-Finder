use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
use shared::file_types::FileRecord;
use shared::ipc_types::{IndexingPhase, IndexingProgress};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetLogicalDriveStringsW, GetVolumeInformationW, QueryDosDeviceW,
    FILE_FLAG_NO_BUFFERING, FILE_FLAG_OVERLAPPED, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{
    FSCTL_ENUM_USN_DATA, MFT_ENUM_DATA_V0, USN_RECORD_V2,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::core::PCWSTR;

const BATCH_SIZE: usize = 10_000;

/// Sistemdeki NTFS ciltlerini listele (örn. ["C:", "D:"])
pub fn get_ntfs_volumes() -> Result<Vec<String>> {
    let mut volumes = Vec::new();

    unsafe {
        // Tüm mantıksal sürücüleri al
        let mut buf = vec![0u16; 512];
        let len = GetLogicalDriveStringsW(Some(&mut buf));
        if len == 0 {
            return Err(anyhow!("GetLogicalDriveStringsW başarısız"));
        }

        let mut offset = 0;
        while offset < len as usize {
            // Null-terminated string bul
            let end = buf[offset..]
                .iter()
                .position(|&c| c == 0)
                .map(|p| offset + p)
                .unwrap_or(len as usize);

            if end == offset {
                break;
            }

            let drive_str = OsString::from_wide(&buf[offset..end]);
            let drive = drive_str.to_string_lossy().to_string();
            offset = end + 1;

            // Sadece harfi al: "C:\" → "C:"
            let drive_letter = if drive.ends_with('\\') {
                drive[..drive.len() - 1].to_string()
            } else {
                drive.clone()
            };

            // NTFS olup olmadığını kontrol et
            let root_path: Vec<u16> = format!("{}\\", drive_letter)
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let mut fs_name_buf = vec![0u16; 32];
            let ok = GetVolumeInformationW(
                PCWSTR(root_path.as_ptr()),
                None,
                None,
                None,
                None,
                Some(&mut fs_name_buf),
            );

            if ok.is_ok() {
                let fs_name = OsString::from_wide(
                    &fs_name_buf[..fs_name_buf
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(fs_name_buf.len())],
                );
                let fs_str = fs_name.to_string_lossy();
                if fs_str == "NTFS" {
                    info!("NTFS cilt bulundu: {}", drive_letter);
                    volumes.push(drive_letter);
                } else {
                    info!("NTFS olmayan cilt atlandı: {} ({})", drive_letter, fs_str);
                }
            }
        }
    }

    Ok(volumes)
}

/// Bir NTFS cildinin MFT'sini tara ve tüm dosyaları döndür
/// `progress_cb`: İlerleme bildirim callback'i
pub fn scan_volume_mft<F>(volume: &str, mut progress_cb: F) -> Result<Vec<FileRecord>>
where
    F: FnMut(IndexingProgress),
{
    info!("MFT taraması başlatılıyor: {}", volume);

    // Cilt handle'ı aç
    let volume_path: Vec<u16> = format!("\\\\.\\{}", volume)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let handle = unsafe {
        CreateFileW(
            PCWSTR(volume_path.as_ptr()),
            0x80000000u32, // GENERIC_READ
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_NO_BUFFERING,
            None,
        )?
    };

    if handle == INVALID_HANDLE_VALUE {
        return Err(anyhow!("Cilt handle'ı açılamadı: {}", volume));
    }

    let result = scan_mft_with_handle(handle, volume, &mut progress_cb);

    unsafe {
        let _ = CloseHandle(handle);
    }

    result
}

/// MFT handle ile dosyaları tara
fn scan_mft_with_handle<F>(
    handle: HANDLE,
    volume: &str,
    progress_cb: &mut F,
) -> Result<Vec<FileRecord>>
where
    F: FnMut(IndexingProgress),
{
    let mut records: Vec<FileRecord> = Vec::new();
    let mut enum_data = MFT_ENUM_DATA_V0 {
        StartFileReferenceNumber: 0,
        LowUsn: 0,
        HighUsn: i64::MAX,
    };

    let buf_size = 1024 * 1024; // 1MB buffer
    let mut output_buf: Vec<u8> = vec![0u8; buf_size];
    let mut bytes_returned: u32 = 0;

    loop {
        let ok = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_ENUM_USN_DATA,
                Some(&enum_data as *const _ as *const _),
                std::mem::size_of::<MFT_ENUM_DATA_V0>() as u32,
                Some(output_buf.as_mut_ptr() as *mut _),
                buf_size as u32,
                Some(&mut bytes_returned),
                None,
            )
        };

        if ok.is_err() || bytes_returned <= 8 {
            break;
        }

        // İlk 8 byte: bir sonraki USN (next file reference number)
        let next_usn = unsafe {
            *(output_buf.as_ptr() as *const i64)
        };
        enum_data.StartFileReferenceNumber = next_usn as u64;

        let mut offset = 8usize;
        while offset < bytes_returned as usize {
            if offset + std::mem::size_of::<USN_RECORD_V2>() > bytes_returned as usize {
                break;
            }

            let usn_record = unsafe {
                &*(output_buf.as_ptr().add(offset) as *const USN_RECORD_V2)
            };

            if usn_record.RecordLength == 0 {
                break;
            }

            // Dosya adını al
            let name_offset = usn_record.FileNameOffset as usize;
            let name_length = usn_record.FileNameLength as usize;

            if name_offset + name_length <= usn_record.RecordLength as usize {
                let name_ptr = unsafe {
                    output_buf
                        .as_ptr()
                        .add(offset + name_offset) as *const u16
                };
                let name_slice = unsafe {
                    std::slice::from_raw_parts(name_ptr, name_length / 2)
                };
                let name = OsString::from_wide(name_slice)
                    .to_string_lossy()
                    .to_string();

                let is_directory =
                    (usn_record.FileAttributes & 0x10) != 0; // FILE_ATTRIBUTE_DIRECTORY

                let extension = if !is_directory {
                    std::path::Path::new(&name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_lowercase())
                } else {
                    None
                };

                // MFT'den sadece isim ve temel bilgiler alınır;
                // tam yolu almak için parent chain traverse gerekir.
                // Büyük çaplı taramada bu path resolution ayrı bir adımda yapılır.
                // Burada parent_path placeholder olarak volume kökünü kullanıyoruz,
                // gerçek yol build_path_map ile sonradan çözümlenir.
                let frn = usn_record.FileReferenceNumber as i64;
                let parent_frn = usn_record.ParentFileReferenceNumber;

                // Geçici kayıt — yollar sonra çözümlenecek
                records.push(FileRecord {
                    id: 0,
                    name: name.clone(),
                    extension,
                    parent_path: format!("{}\\[FRN:{}]", volume, parent_frn),
                    full_path: format!("{}\\[FRN:{}]\\{}", volume, parent_frn, name),
                    size: None,
                    created_time: None,
                    modified_time: None,
                    is_directory,
                    volume_id: Some(volume.to_string()),
                    frn: Some(frn),
                    parent_frn: Some(parent_frn as i64),
                    score: 0.0,
                }); if records.len() % 50_000 == 0 {
                    progress_cb(IndexingProgress {
                        current: records.len() as u64,
                        total: 0,
                        volume: volume.to_string(),
                        phase: IndexingPhase::Scanning,
                    });
                    debug!("{}: {} kayıt tarandı", volume, records.len());
                }
            }

            offset += usn_record.RecordLength as usize;
        }

        if next_usn == 0 {
            break;
        }
    }

    info!("{}: Tarama tamamlandı, {} kayıt", volume, records.len());

    // Yol çözümleme
    progress_cb(IndexingProgress {
        current: 0,
        total: records.len() as u64,
        volume: volume.to_string(),
        phase: IndexingPhase::Inserting,
    });

    let records = resolve_paths(records, volume);

    progress_cb(IndexingProgress {
        current: records.len() as u64,
        total: records.len() as u64,
        volume: volume.to_string(),
        phase: IndexingPhase::Complete,
    });

    Ok(records)
}

/// FRN tabanlı parent referansları gerçek yollara çevir
fn resolve_paths(mut records: Vec<FileRecord>, volume: &str) -> Vec<FileRecord> {
    use std::collections::HashMap;

    // FRN → (name, parent_frn) haritası oluştur
    let mut frn_map: HashMap<i64, (String, i64, bool)> = HashMap::new();
    for r in &records {
        if let Some(frn) = r.frn {
            // parent_frn'i placeholder'dan çıkar
            let parent_frn = if r.parent_path.contains("[FRN:") {
                r.parent_path
                    .trim_start_matches(&format!("{}\\[FRN:", volume))
                    .trim_end_matches(']')
                    .parse::<i64>()
                    .unwrap_or(0)
            } else {
                0
            };
            frn_map.insert(frn, (r.name.clone(), parent_frn, r.is_directory));
        }
    }

    // Özyinelemeli yol oluşturma (derinlik sınırlı)
    fn build_path(
        frn: i64,
        frn_map: &HashMap<i64, (String, i64, bool)>,
        volume: &str,
        depth: u32,
    ) -> String {
        if depth > 64 {
            return format!("{}\\", volume);
        }
        match frn_map.get(&frn) {
            None => format!("{}\\", volume),
            Some((name, parent_frn, _)) => {
                if *parent_frn == 0 || *parent_frn == frn {
                    format!("{}\\{}", volume, name)
                } else {
                    let parent_path = build_path(*parent_frn, frn_map, volume, depth + 1);
                    format!("{}\\{}", parent_path, name)
                }
            }
        }
    }

    for r in &mut records {
        if let Some(frn) = r.frn {
            if let Some((name, parent_frn, _)) = frn_map.get(&frn) {
                let parent_path = build_path(*parent_frn, &frn_map, volume, 0);
                r.parent_path = parent_path.clone();
                r.full_path = format!("{}\\{}", parent_path, name);
            }
        }
    }

    records
}
