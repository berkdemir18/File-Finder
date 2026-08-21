// MFT ham kayıt parser yardımcıları
// Düşük seviyeli MFT attribute parse etme işlemleri burada tanımlanır.

use anyhow::Result;

/// FILETIME (Windows 100-nanosaniye aralıkları, 1601-01-01 başlangıçlı)
/// Unix timestamp'e çevir (saniye, 1970-01-01 başlangıçlı)
pub fn filetime_to_unix(filetime: i64) -> i64 {
    // 1601-01-01 ile 1970-01-01 arasındaki fark: 116444736000000000 * 100ns
    const FILETIME_EPOCH_DIFF: i64 = 116_444_736_000_000_000;
    (filetime - FILETIME_EPOCH_DIFF) / 10_000_000
}

/// Windows boyut (HIGH/LOW DWORD) çifti → u64
pub fn large_integer_to_u64(high: u32, low: u32) -> u64 {
    ((high as u64) << 32) | (low as u64)
}

/// MFT FRN (File Reference Number) → gerçek dosya numarası
/// FRN'in üst 16 bit'i sequence number, alt 48 bit gerçek kayıt numarası
pub fn frn_to_record_number(frn: u64) -> u64 {
    frn & 0x0000_FFFF_FFFF_FFFF
}
