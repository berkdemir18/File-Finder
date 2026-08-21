use anyhow::{anyhow, Result};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, GENERIC_READ, GENERIC_WRITE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FlushFileBuffers,
    OPEN_EXISTING, FILE_SHARE_READ, FILE_SHARE_WRITE,
};
use windows::core::PCWSTR;

/// Named Pipe istemcisi
pub struct ServiceClient {
    handle: HANDLE,
}

impl ServiceClient {
    /// Pipe'a bağlan
    pub fn connect(pipe_name: &str) -> Result<Self> {
        let pipe_path: Vec<u16> = pipe_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            CreateFileW(
                PCWSTR(pipe_path.as_ptr()),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                Default::default(),
                None,
            )
        };

        match handle {
            Ok(h) if h != INVALID_HANDLE_VALUE => {
                log::info!("Named pipe bağlandı: {}", pipe_name);
                Ok(Self { handle: h })
            }
            Ok(_) | Err(_) => {
                Err(anyhow!("Pipe bağlantısı başarısız: {}", pipe_name))
            }
        }
    }

    /// JSON mesaj gönder ve yanıt al
    pub fn send_receive(&self, msg: &str) -> Result<String> {
        self.write_message(msg)?;
        self.read_message()
    }

    fn write_message(&self, msg: &str) -> Result<()> {
        let bytes = msg.as_bytes();
        let len = bytes.len() as u32;
        let len_buf = len.to_le_bytes();
        let mut written: u32 = 0;

        unsafe {
            WriteFile(self.handle, Some(&len_buf), Some(&mut written), None)?;
            WriteFile(self.handle, Some(bytes), Some(&mut written), None)?;
            FlushFileBuffers(self.handle)?;
        }
        Ok(())
    }

    fn read_message(&self) -> Result<String> {
        let mut len_buf = [0u8; 4];
        let mut read: u32 = 0;

        unsafe {
            ReadFile(self.handle, Some(&mut len_buf), Some(&mut read), None)?;
        }

        if read != 4 {
            return Err(anyhow!("Eksik header"));
        }

        let len = u32::from_le_bytes(len_buf) as usize;
        let mut body = vec![0u8; len];
        let mut total = 0;

        while total < len {
            unsafe {
                ReadFile(
                    self.handle,
                    Some(&mut body[total..]),
                    Some(&mut read),
                    None,
                )?;
            }
            total += read as usize;
        }

        Ok(String::from_utf8(body)?)
    }
}

impl Drop for ServiceClient {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}
