# LanBlaze

**Fast, cloudless file transfer between devices on the same LAN.**
No accounts, no internet round-trip, no relay servers. Built with Tauri 2
(Rust core + React/Fluent UI), aiming to saturate the wire on a single TCP
stream.

[![Latest release](https://img.shields.io/github/v/release/ethemdemirkaya/lan-file-transfer?label=latest&logo=github&color=0078d4)](https://github.com/ethemdemirkaya/lan-file-transfer/releases/latest)
[![Total downloads](https://img.shields.io/github/downloads/ethemdemirkaya/lan-file-transfer/total?logo=github&color=0078d4)](https://github.com/ethemdemirkaya/lan-file-transfer/releases)
[![Latest downloads](https://img.shields.io/github/downloads/ethemdemirkaya/lan-file-transfer/latest/total?label=latest%20downloads&color=0078d4)](https://github.com/ethemdemirkaya/lan-file-transfer/releases/latest)
[![Repo stars](https://img.shields.io/github/stars/ethemdemirkaya/lan-file-transfer?style=flat&logo=github&color=0078d4)](https://github.com/ethemdemirkaya/lan-file-transfer/stargazers)
[![Last commit](https://img.shields.io/github/last-commit/ethemdemirkaya/lan-file-transfer?color=0078d4)](https://github.com/ethemdemirkaya/lan-file-transfer/commits/main)
&nbsp;
![Platform](https://img.shields.io/badge/platform-Windows-0078d4)
![Tauri](https://img.shields.io/badge/Tauri-2.x-FFC131?logo=tauri&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust)
![React](https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=black)
![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)
![Fluent UI](https://img.shields.io/badge/Fluent%20UI-v9-0078d4)

![LanBlaze main screen](docs/screenshots/main.png)

---

## Download

➡️ **[Latest installer (Windows x64)](https://github.com/ethemdemirkaya/lan-file-transfer/releases/latest)**

Per-user install (no UAC). Pick installer language at first run. On first
launch Windows Defender Firewall asks for network access — tick **Private
networks** and click **Allow access**.

## Why

Existing tools either round-trip through a cloud service or fall apart on
the "50 000 tiny files" case because they ack every file. LanBlaze is built
around two ideas:

- **One big file:** push it through a single TCP connection with large
  buffers (256 KB chunks, `TCP_NODELAY`); the kernel handles the rest.
- **Many small files:** pipeline them back-to-back over the same stream,
  tar-style — no per-file ack, no RTT tax.

## Features

- **mDNS discovery** — peers appear in a list automatically. A `↻ Refresh`
  button re-broadcasts your own advertisement and re-issues the browse
  query when a late-joiner doesn't show up.
- **6-digit pairing code** — every receiver shows a code; senders must
  enter it. Wrong code → instant reject, no user prompt.
- **Trust list** — tick "Always trust this device" when accepting an
  incoming transfer; future transfers from the same device skip the
  dialog. Manage from Settings.
- **blake3 integrity check** end-to-end, with atomic `.part` → final rename.
- **Resume support** for files ≥ 8 MB — kill the transfer mid-flight,
  restart it, it picks up from the byte that hit disk.
- **Path-traversal safe** — incoming relative paths are sanitized
  (absolute paths, drive letters, `..` rejected).
- **Drag and drop** files or folders onto the window.
- **Per-transfer destination override** — the accept dialog lets you
  redirect a single incoming transfer to a different folder.
- **Live per-direction rates** — instant MB/s and average MB/s shown
  separately, plus on the receiver side `network: X · disk: Y` so the
  bottleneck is visible.
- **Disk-space pre-check** — receiver rejects before the user is even
  prompted if there isn't ~1.05× of the announced size free.
- **Persistent history** — last 200 transfers kept across restarts.
- **System notifications + sounds** for incoming requests and completions.
- **Tray icon, autostart, close-to-tray** — runs quietly in the
  background so the receiver is always available.
- **Windows 11 Fluent look** with Mica window background; follows the
  system light/dark theme or override from Settings.
- **10 UI languages**: English · Türkçe · Español · Deutsch · Français ·
  日本語 · 中文 · Русский · Português · العربية (RTL).

## Tech stack

| Layer | Choice | Why |
|---|---|---|
| Desktop framework | **Tauri 2** | Native Rust core, web view for UI. Tiny installer (~3 MB) compared to Electron. |
| Backend language | **Rust** (stable) | Zero-cost async I/O via tokio, no GC pauses on the data path. |
| Async runtime | **tokio** | LAN I/O + per-file pipelined tasks (reader/writer split via `mpsc`). |
| Hashing | **blake3** | Faster than SHA-2 on modern CPUs; doesn't bottleneck a gigabit link. |
| Discovery | **mdns-sd** | Standard DNS-SD over multicast, no central server. |
| Disk-space probe | **fs2** | Cross-platform `available_space`. |
| Tray + plugins | `tauri-plugin-{notification, autostart, dialog}` | OS-level toasts, autostart on login, native file picker. |
| Window effect | **window-vibrancy** | Windows 11 Mica fallback when `tauri.conf.json windowEffects` isn't enough. |
| Frontend | **React 18 + TypeScript + Vite** | Familiar, fast HMR. |
| UI library | **Fluent UI React v9** | Microsoft's official Fluent Design components — closest you get to WinUI 3 in a web view. |
| i18n | **i18next + react-i18next** | 10 bundled locales with pluralization + RTL handling. |
| Installer | **NSIS** (via `tauri-bundler`) | Per-user or all-users install, multi-language wizard, LZMA-compressed. |

## How to use it

1. **First run** — pick a device name and a default destination folder.
2. The main screen shows your **6-digit pairing code** in the top right.
   Share it with the sender.
3. As the sender:
   - drop files/folders onto the window (or use the buttons),
   - pick the destination device from the discovered list (or paste an IP),
   - paste the receiver's 6-digit code,
   - hit **Send**.
4. The receiver sees a dialog with the sender, file count, total size,
   and a "save somewhere else" button. Tick **"Always trust this
   device"** to skip the dialog on future transfers from the same peer.

### Single-machine loopback test

Manual IP `127.0.0.1` + your own pairing code lets you send to yourself
for a quick sanity check.

### Testing in a VirtualBox VM

- VM settings → **Network → Adapter 1**: **Bridged Adapter** with your
  Wi-Fi / Ethernet selected (NAT will not work — mDNS multicast doesn't
  cross the NAT boundary).
- Adapter Type: Intel PRO/1000 MT Desktop, Promiscuous: Allow All on
  Wi-Fi.
- Install the same `setup.exe` inside the VM.
- If the host doesn't see the VM right away, press **↻ Refresh** in
  Step 2 — it re-broadcasts your advertisement and re-issues the
  browse query.

## Protocol (v2)

All multi-byte integers are little-endian. One TCP connection carries the
whole transfer:

```
[u32 hello_len][HELLO JSON]               ← sender → receiver
[u32 ack_len][HELLO_ACK JSON]             ← receiver → sender
repeated per file:
  [u16 path_len][path bytes][u64 size][u8 resumable]
  if resumable == 1:
    [u64 offset]                          ← receiver → sender
  [size - offset bytes of file body]
  [32 bytes blake3 of the full file]
[u16 0]                                   ← end-of-stream
```

`HELLO` carries the sender's device name, OS, file count, total bytes
and the receiver's pairing code. `HELLO_ACK` is `{ accept, reason? }`.
Files below 8 MiB skip the resume handshake to keep the small-files
pipeline at zero extra RTTs.

## Build from source

Requirements: **Node.js ≥ 20**, **Rust stable**, Windows with the
**WebView2** runtime.

```bash
git clone https://github.com/ethemdemirkaya/lan-file-transfer
cd lan-file-transfer
npm install
npm run tauri dev          # development (HMR)
npm run tauri build        # production installer
```

The installer ends up in
`src-tauri/target/release/bundle/nsis/LanBlaze_<version>_x64-setup.exe`.

## Roadmap

- [x] Phase 0 — Tauri 2 + React/TS + Fluent UI scaffold
- [x] Phase 1 — Single-file transfer, blake3, atomic write
- [x] Phase 2 — Many-file / folder pipeline
- [x] Phase 3 — mDNS discovery
- [x] Phase 4 — Setup wizard, pairing code, drag-and-drop, incoming dialog
- [ ] Phase 5 — TLS encryption (`tokio-rustls` + `rcgen` self-signed)
- [x] Phase 6 — Resume for interrupted large-file transfers
- [x] Phase 7 — NSIS installer
- [ ] **Next:** sender-side read/write pipeline + bigger SO_SNDBUF +
  `FILE_FLAG_SEQUENTIAL_SCAN` to push the many-tiny-files throughput
  up further
- [ ] Cross-platform builds (Linux, macOS) via GitHub Actions
- [ ] Auto-updater (`tauri-plugin-updater` with signed releases)

## Contributing

Issues and pull requests welcome. The project follows Conventional
Commits and ships everything English (commit messages, code comments,
docs); the UI is fully localized.

## License

No license attached yet — ask before redistributing.
