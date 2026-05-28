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

- [x] Faz 0 — Tauri 2.0 + React/TS + Vite + Fluent UI iskeleti, `ping` komutu çalışıyor.
- [ ] Faz 1 — Tek dosya transferi (manuel IP, blake3).
- [ ] Faz 2 — Çok dosya pipeline.
- [ ] Faz 3 — mDNS keşfi.
- [ ] Faz 4 — UI cilası (Mica, sürükle-bırak, canlı hız).

## Faz 0 nasıl test edilir

```bash
npm install
npm run tauri dev
```

Pencere açıldığında "Ping gönder" butonuna bas. `pong from Rust, hello LanBlaze` yanıtı görünmeli.
