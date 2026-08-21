fn main() {
    // Windows kaynaklarını (ikon + manifest) exe'ye gömer.
    //
    // Bu çağrı bir dönem "windres gerekmesin diye" devre dışı bırakılmıştı;
    // sonucu exe'nin İKONSUZ kalmasıydı — Windows görev çubuğunda boş belge
    // simgesi görünüyordu. Manifest dışarıdan bir araçla geri konmuş ama ikon
    // unutulmuştu. Kaynak derleyicisi Windows SDK ile birlikte geliyor
    // (rc.exe), o yüzden bypass'a gerek yok.
    tauri_build::build()
}
