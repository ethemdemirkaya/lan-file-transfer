# LanBlaze

Aynı yerel ağdaki iki cihaz arasında bulutsuz, hesapsız, yüksek hızlı dosya/klasör aktarımı. Tauri 2.0 + Rust çekirdek + React/Fluent UI.

> Plan ve mimari ayrıntıları için `CLAUDE.md` dosyasına bakın.

## Gereksinimler

- Node.js 20+
- Rust 1.78+ (stable) ve Cargo
- Windows: WebView2 runtime (Win11'de hazır gelir)

## Geliştirme

```bash
npm install
npm run tauri dev
```

İlk derleme Rust bağımlılıkları yüzünden birkaç dakika sürer.

## Üretim derlemesi

```bash
npm run tauri build
```

Çıktı: `src-tauri/target/release/bundle/nsis/LanBlaze_<versiyon>_x64-setup.exe`

## Fazlar

- [x] Faz 0 — Tauri 2.0 + React/TS + Vite + Fluent UI iskeleti.
- [x] Faz 1 — Tek dosya transferi (manuel IP, blake3, atomik yazma).
- [x] Faz 2 — Çok dosya / klasör pipeline (tek TCP akışı, ack yok).
- [x] Faz 3 — mDNS keşfi (`_lanblaze._tcp.local.`), peer butonları.
- [ ] Faz 4 — UI cilası (Mica, sürükle-bırak, canlı hız).

## Faz 1 nasıl test edilir

İki cihazda (veya aynı cihazda iki örnek olarak):

1. **Alıcı tarafta:** `npm run tauri dev`. Sol "Alıcı" kartında bir kayıt klasörü seç,
   "Dinlemeyi başlat"a bas. Üstte `IP:port` (varsayılan `47813`) görünecek.
2. **Gönderici tarafta:** Aynı pencerede sağdaki "Gönderici" kartına alıcının IP'sini
   yaz, bir dosya seç, "Gönder"e bas.
3. Aşağıdaki "Aktif transferler" bölümünde canlı ilerleme + MB/s ve dosya sayısı
   görünür. Hash eşleşirse alıcı tarafta `.part` dosyası nihai isme atomik olarak
   yeniden adlandırılır.

Beklenen: Gigabit ethernet'te tek 10 GB dosya için ≥ 110 MB/s.

## Faz 2 nasıl test edilir

Gönderici kartında **Klasör** butonuyla bir klasör seç veya **Dosya(lar)** ile birden
çok dosya seç, "Gönder"e bas. Tüm dosyalar tek bir TCP akışında, ack beklemeden
ardı ardına aktarılır (`CLAUDE.md` §4.3). Klasör yapısı alıcı tarafta korunur.

Test ipucu — 50.000 küçük dosya:

```powershell
$root = "$env:TEMP\lanblaze-stress"; New-Item -ItemType Directory -Force $root | Out-Null
1..50000 | ForEach-Object { Set-Content -NoNewline "$root\$_.txt" ([byte[]](1..2048) -join '') }
```

Bu klasörü gönder. Protokol RTT'si yüzünden tek haneli MB/s'ye DÜŞMEMELİ; darboğaz
disk IOPS olmalı.

## Faz 3 nasıl test edilir

İki cihazda da uygulamayı aç. Her birinde "Dinlemeyi başlat"a bas. Birkaç saniye
içinde "Keşfedilen cihazlar" altında karşı tarafın butonu çıkmalı; butona
tıklayınca IP ve port otomatik dolar. mDNS engelliyse elle IP girmek hâlâ
çalışır.

## Güvenlik notları (Faz 1)

- Alıcı taraf gelen göreli yolları sıkı doğrular (`..`, sürücü harfi, mutlak yol
  reddedilir) — `CLAUDE.md` §6.
- Windows Firewall ilk çalıştırmada port için izin isteyebilir; özel ağ için
  onaylayın.
