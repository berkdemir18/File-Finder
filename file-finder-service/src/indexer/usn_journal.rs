use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use shared::file_types::FileRecord;
use shared::ipc_types::{ChangeEvent, ServerMessage};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_NO_BUFFERING, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{
    FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0,
    USN_JOURNAL_DATA_V0, USN_RECORD_V2,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::core::PCWSTR;

use crate::db::Database;

/// USN Journal değişiklik bayrakları
const USN_REASON_FILE_CREATE: u32 = 0x00000100;
const USN_REASON_FILE_DELETE: u32 = 0x00000200;
const USN_REASON_RENAME_NEW_NAME: u32 = 0x00002000;
const USN_REASON_DATA_OVERWRITE: u32 = 0x00000001;
const USN_REASON_DATA_EXTEND: u32 = 0x00000002;

/// Tüm NTFS ciltleri için USN Journal dinlemeyi başlat
pub async fn start_watching(
    volumes: Vec<String>,
    db: Arc<Mutex<Database>>,
    event_tx: broadcast::Sender<ServerMessage>,
) {
    for volume in volumes {
        let db = db.clone();
        let tx = event_tx.clone();
        let vol = volume.clone();

        tokio::spawn(async move {
            loop {
                let result = tokio::task::spawn_blocking({
                    let vol = vol.clone();
                    let db = db.clone();
                    let tx = tx.clone();
                    move || watch_volume_usn(&vol, db, tx)
                })
                .await;

                match result {
                    Ok(Ok(())) => info!("{}: USN Journal izleme tamamlandı.", vol),
                    Ok(Err(e)) => {
                        error!("{}: USN Journal hatası: {}. 5s sonra yeniden başlatılıyor.", vol, e);
                    }
                    Err(e) => {
                        error!("{}: USN Journal task panic: {}.", vol, e);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    }
}

/// Tek bir cilt için USN Journal izleme döngüsü
fn watch_volume_usn(
    volume: &str,
    db: Arc<Mutex<Database>>,
    event_tx: broadcast::Sender<ServerMessage>,
) -> Result<()> {
    let volume_path: Vec<u16> = format!("\\\\.\\{}", volume)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let handle = unsafe {
        CreateFileW(
            PCWSTR(volume_path.as_ptr()),
            0x80000000u32,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_NO_BUFFERING,
            None,
        )?
    };

    if handle == INVALID_HANDLE_VALUE {
        return Err(anyhow!("{}: USN Journal handle açılamadı", volume));
    }

    info!("{}: USN Journal dinleme başlatıldı.", volume);

    // Journal ID ve güncel USN'i al
    let mut journal_data = USN_JOURNAL_DATA_V0::default();
    let mut bytes_returned: u32 = 0;

    unsafe {
        DeviceIoControl(
            handle,
            FSCTL_QUERY_USN_JOURNAL,
            None,
            0,
            Some(&mut journal_data as *mut _ as *mut _),
            std::mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
            Some(&mut bytes_returned),
            None,
        )?;
    }

    let mut next_usn = journal_data.NextUsn;
    info!("{}: USN Journal ID: {}, başlangıç USN: {}", volume, journal_data.UsnJournalID, next_usn);

    let buf_size = 512 * 1024; // 512KB buffer
    let mut output_buf: Vec<u8> = vec![0u8; buf_size];

    loop {
        let read_data = READ_USN_JOURNAL_DATA_V0 {
            StartUsn: next_usn,
            ReasonMask: 0xFFFFFFFF, // Tüm değişiklik tipleri
            ReturnOnlyOnClose: 0,
            Timeout: 1,             // 1 saniye bekle
            BytesToWaitFor: 1,
            UsnJournalID: journal_data.UsnJournalID,
        };

        bytes_returned = 0;
        let result = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_READ_USN_JOURNAL,
                Some(&read_data as *const _ as *const _),
                std::mem::size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                Some(output_buf.as_mut_ptr() as *mut _),
                buf_size as u32,
                Some(&mut bytes_returned),
                None,
            )
        };

        if result.is_err() {
            // ERROR_JOURNAL_DELETE_IN_PROGRESS veya benzeri
            warn!("{}: USN Journal okuma hatası, yeniden başlatılıyor.", volume);
            break;
        }

        if bytes_returned <= 8 {
            // Yeni değişiklik yok, kısa bekle
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }

        // İlk 8 byte: sonraki USN
        next_usn = unsafe { *(output_buf.as_ptr() as *const i64) };

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

            let name_offset = usn_record.FileNameOffset as usize;
            let name_length = usn_record.FileNameLength as usize;
            let reason = usn_record.Reason;

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
                    (usn_record.FileAttributes & 0x10) != 0;

                debug!("{}: USN değişiklik: {} (reason: {:#010x})", volume, name, reason);

                // İşle: oluşturma, silme veya yeniden adlandırma
                let event = if reason & USN_REASON_FILE_DELETE != 0 {
                    Some(ChangeEvent::Deleted)
                } else if reason & USN_REASON_FILE_CREATE != 0 {
                    Some(ChangeEvent::Created)
                } else if reason & USN_REASON_RENAME_NEW_NAME != 0 {
                    Some(ChangeEvent::Renamed {
                        old_name: String::new(), // USN'de eski isim yok
                    })
                } else if reason & (USN_REASON_DATA_OVERWRITE | USN_REASON_DATA_EXTEND) != 0 {
                    Some(ChangeEvent::Modified)
                } else {
                    None
                };

                if let Some(event) = event {
                    let frn = usn_record.FileReferenceNumber as i64;
                    let parent_frn = usn_record.ParentFileReferenceNumber as i64;

                    // Veritabanını güncelle
                    let path = format!("{}\\[FRN:{}]\\{}", volume, parent_frn, name);

                    let db = db.clone();
                    let tx = event_tx.clone();
                    let name_clone = name.clone();
                    let volume_clone = volume.to_string();
                    let event_clone = event.clone();

                    // Senkron DB güncellemesi (spawn_blocking içindeyiz zaten)
                    let db_guard = futures::executor::block_on(db.lock());
                    match &event {
                        ChangeEvent::Deleted => {
                            let _ = crate::db::queries::delete_file_by_path(
                                &db_guard.conn, &path,
                            );
                        }
                        ChangeEvent::Created | ChangeEvent::Modified | ChangeEvent::Renamed { .. } => {
                            let extension = if !is_directory {
                                std::path::Path::new(&name_clone)
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .map(|e| e.to_lowercase())
                            } else {
                                None
                            };
                            let record = FileRecord {
                                id: 0,
                                name: name_clone.clone(),
                                extension,
                                parent_path: format!("{}\\[FRN:{}]", volume_clone, parent_frn),
                                full_path: path.clone(),
                                size: None,
                                created_time: None,
                                modified_time: Some(chrono::Utc::now().timestamp()),
                                is_directory,
                                volume_id: Some(volume_clone.clone()),
                                frn: Some(frn),
                                parent_frn: Some(parent_frn),
                                score: 0.0,
                            };
                            let _ = crate::db::queries::upsert_file(&db_guard.conn, &record);
                        }
                    }
                    drop(db_guard);

                    // UI'ya bildir
                    let _ = tx.send(ServerMessage::IndexUpdated {
                        path,
                        event: event_clone,
                    });
                }
            }

            offset += usn_record.RecordLength as usize;
        }
    }

    unsafe {
        let _ = CloseHandle(handle);
    }

    Ok(())
}
