# File Finder — arayüz

Windows için klavye odaklı, anlık indeksli dosya arama. Tauri 2 (Rust) + React 19.

## Çalıştırma

```bash
npm run dev          # yalnız arayüz (Vite, :1420)
npx tauri dev        # gerçek uygulama
npx tauri build      # msi + nsis kurulum paketi
```

## Tasarım sistemi

Görsel dil `Brand & Interface System v1`den gelir; tamamı
`src/styles/global.css` içindeki token'larla tanımlıdır:

| Rol     | Karanlık  | Kullanım                                    |
| ------- | --------- | ------------------------------------------- |
| Zemin   | `#0C0D0F` | pencere arkası                              |
| Yüzey   | `#13151A` | başlık çubuğu, önizleme, seçili satır       |
| Kenar   | `#22252A` | ayraçlar                                    |
| Vurgu   | `#F0C26A` | **yalnız kullanıcının kendi işaretlerinde** |
| İkincil | `#7FD8C9` | ölçüm ve durum (süre, sayaç, canlı indeks)  |
| Metin   | `#ECEEF1` | ana metin                                   |

Vurgu rengi bilerek kıttır: odaklı arama kutusu, seçili satırın kenar şeridi,
eşleşen harfler ve favori yıldızı. Sayılar, yollar ve kısayollar hep JetBrains
Mono; arayüz metni Space Grotesk. Yarıçap ailesi 10/12/14/20px.

Açık tema `[data-theme='light']` altında aynı token'ları yeniden tanımlar —
yeni renk yazarken **her zaman token kullan**, yoksa açık temada kırılır.

### Fontlar

`src/assets/fonts/` altındaki dört woff2 uygulamayla birlikte paketlenir.
Google Fonts'tan `@import` **yapılmaz**: masaüstü uygulaması çevrimdışıyken de
aynı görünmeli ve açılışta ağ isteği atmamalı.

### İkon

`src-tauri/icons/` tamamı tek bir 1024px ana görselden üretilir
(`npx tauri icon <png>`). İşaret: kıvrılmış dosya köşesi + mercek, tek çizgi
kalınlığı. `public/favicon.svg` ve `components/Brand/Logo.tsx` aynı geometriyi
paylaşır — birini değiştirirken üçünü birden güncelle.

## Tasarım önizlemesi

Uygulamayı derlemeden ekranları tarayıcıda görmek için:

```bash
npm run dev
```

sonra <http://localhost:1420/preview.html>

`src/dev/tauriMock.ts` Tauri IPC'sini sınırından taklit eder (sahte dosya
listesi, ayarlar, indeks durumu); `App` bileşeni hiç değiştirilmeden çalışır.
Bu giriş noktası `dist`e **çıkmaz** — vite yalnız `index.html`i derler.

## Mimari notlar

- **Arama sıralaması iki aşamalı.** SQLite yalnız *aday* toplar
  (`CANDIDATE_POOL = 2000`), asıl sıralamayı Rust tarafındaki fuzzy skor yapar.
  Adaylar alfabetik değil, alaka ihtimaline göre seçilir (tam eşleşme → baştan
  eşleşme → kısa isim). Havuz küçültülürse alaka sıralaması bozulur.
- **Sorgu kelimelere bölünür.** "football manager 24" üç ayrı `LIKE` olur,
  böylece `FootballManager24` bulunur. Fuzzy eşleştirici de desendeki boşlukları
  yok sayar.
- **Sayfalama sıralamadan sonra.** Her sayfa aynı aday havuzunu sıralar ve dilim
  alır; yoksa 2. sayfa bambaşka bir kümeyi yeniden sıralardı.
- **Aday toplama kapsayıcı indeksten.** Ek filtre yoksa yalnız `(id, name)`
  çekilir; `idx_files_name` isim üzerinde olduğu ve sıralama da yalnız `name`
  kullandığı için SQLite 3 GB'lık tabloya hiç inmez, sadece indeksi tarar.
  Satırların tamamı sonradan, yalnız aday havuzu için birincil anahtarla
  doldurulur. Ölçüm (4,98M kayıt): **14,9 sn → 0,76 sn**. Filtre varsa
  (`ext:`/`size:`/`modified:`/`path:`) o sütunlar indekste olmadığından
  eski tam tarama yolu kullanılır.
- **Gerçek `COUNT(*)` YOK.** İkinci bir tam tarama demekti ve arama süresini
  iki katına çıkarıyordu. Havuz dolduysa `pool_full` bayrağı dönüyor, arayüz
  sayıyı "2000+" diye yazıyor — kırpılmış sayıyı kesin sayı gibi göstermiyor.
- **WAL sınırlı.** `journal_size_limit` verilmezse SQLite WAL'ı checkpoint
  sonrası küçültmez; ilk indeksleme turunda dosya 3,9 GB'a çıkıp diski
  doldurmuştu. Artık 64 MB'ta tutuluyor ve açılışta bir kez TRUNCATE
  checkpoint yapılıyor.
- **Arama geçmişine yalnız açılan sorgu yazılır** (`lib/actions.ts`), her tuş
  vuruşunda değil — yoksa geçmiş "f", "fo", "foo" ile dolar.
- **Odak kaybında gizleme varsayılan KAPALI.** Açıkken küçültme düğmesi
  pencereyi tümden yok ediyormuş gibi görünüyor (küçültmek zaten odağı
  kaybettirir). Ayrıca küçültülmüş pencere hiçbir zaman gizlenmez.
- **Windows kaynakları `tauri_build::build()` ile gömülür.** Bu çağrı bir
  dönem devre dışı bırakılmıştı; sonucu exe'nin ikonsuz kalmasıydı. Kaynak
  derleyicisi (`rc.exe`) Windows SDK ile geliyor, bypass'a gerek yok.
- **Kısayol ayardan gelir.** Tutmazsa yedeklere düşer ve gerçekte hangisine
  bağlandığını `hotkey-registered` olayıyla bildirir; ayarlar ekranı uyuşmazlığı
  gösterir. Sessizce başka tuşa bağlanmaz.
- **Ayarların hepsinin bir karşılığı var.** "Windows ile başlat" HKCU\Run'a
  yazar, hassasiyet arama eşiğine, animasyon hızı `--speed` değişkenine,
  önizleme paneli ve odak-kaybında-gizle doğrudan davranışa bağlıdır.

## Klavye

| Tuş                | İş                                          |
| ------------------ | ------------------------------------------- |
| `Ctrl+Space`       | uygulamayı çağır (ayarlanabilir)            |
| `↑ ↓` `PgUp/PgDn`  | sonuçlar arasında gezin                     |
| `Enter`            | aç                                          |
| `Ctrl+Shift+Enter` | yönetici olarak çalıştır                    |
| `Ctrl+O`           | dosya konumunu aç                           |
| `Ctrl+C`           | yolu kopyala                                |
| `Alt+1..9`         | n. sonucu aç                                |
| `Del`              | geri dönüşüme gönder                        |
| `Esc`              | önce aramayı temizle, sonra pencereyi gizle |

Sorgu kutusu boşken `↑ ↓` arama geçmişinde gezinir.

## Sorgu filtreleri

`ext:pdf` · `size:>1gb` · `size:100kb-5mb` · `modified:today` · `modified:<7d` ·
`created:2026-01-01..2026-06-30` · `path:C:\Projeler` · `/regex/`

Filtreler birleşir ve yalnız filtre yazmak da geçerli bir aramadır (o durumda
sonuçlar en son değişene göre sıralanır).
