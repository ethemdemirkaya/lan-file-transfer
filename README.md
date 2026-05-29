# LanBlaze

**Fast, cloudless file transfer between devices on the same LAN.**
No accounts, no internet round-trip, no relay servers. Built with Tauri 2
(Rust core + React/Fluent UI), aiming to saturate the wire on a single TCP
stream and stay out of your way the rest of the time.

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

Per-user install (no UAC). Multi-language installer wizard. On first
launch Windows Defender Firewall asks for network access — tick
**Private networks** and click **Allow access**.

The app also checks GitHub for new releases on startup (once per 12 hours)
and offers an inline download banner.

## Why

Existing tools either round-trip through a cloud service or fall apart on
the "50 000 tiny files" case because they ack every file. LanBlaze is built
around two ideas:

- **One big file:** push it through a single TCP connection with large
  buffers (256 KB chunks, `TCP_NODELAY`, 4 MB `SO_SNDBUF` / `SO_RCVBUF`);
  the kernel handles the rest.
- **Many small files:** pipeline them back-to-back over the same stream,
  tar-style — no per-file ack, no RTT tax. Sender and receiver each run a
  decoupled read/write task pair so disk-read and network-write overlap.

## Features

### Transfer

- **mDNS discovery** with a manual `↻ Refresh` button that re-broadcasts
  our advertisement and re-issues the browse query when a late-joiner
  doesn't show up on its own.
- **6-digit pairing code** — every receiver shows a code; senders must
  enter it. Wrong code → instant reject, no user prompt.
- **Trust list** — tick "Always trust this device" when accepting an
  incoming transfer; subsequent transfers from the same device skip the
  dialog. Manage from Settings.
- **blake3 integrity check** end-to-end, with atomic `.part` → final rename.
- **Resume support** for files ≥ 8 MB — kill mid-transfer, restart,
  picks up from the byte that hit disk. The handshake is protocol-level
  so the sender skips already-on-disk bytes.
- **Path-traversal safe** — incoming relative paths are sanitized
  (absolute paths, drive letters, `..` rejected).
- **Drag and drop** files or folders onto the window.
- **Right-click → "Send via LanBlaze"** on any file, folder, or empty
  folder background — installer registers the menu entry; the app
  opens with the selection pre-loaded.
- **Per-transfer destination override** — the accept dialog lets you
  redirect a single incoming transfer to a different folder.
- **Pause / Resume** for any active transfer from either side; ▶/⏸ button
  on the row, peer notices through TCP back-pressure.
- **Cancel** from either side — sender or receiver can stop a transfer;
  the other side is told *who* cancelled ("Canceled by you" vs
  "Canceled by peer").
- **Disk-space pre-check** — receiver rejects with a clear reason before
  the user is prompted if there isn't ~1.05× of the announced size free.

### UI / UX

- **Live per-direction rates** with proper labels: `NOW` / `AVERAGE` /
  `ETA` for senders; on the receiver, `NETWORK` and `DISK` split out
  separately so the bottleneck is visible. ETA in `d / h / m / s`.
- **Windows taskbar progress overlay** — green bar tracks the combined
  progress of active transfers; turns yellow when any is paused.
- **Custom Mica title bar** — extends edge-to-edge, with Win11-spec
  46×32 min/max/close buttons.
- **System accent color** — read from `HKCU\Software\Microsoft\Windows\DWM`
  and applied across the Fluent brand tokens (buttons, badges,
  progress bar, selected peer tile). Toggle in Settings.
- **System notifications + sounds** for incoming requests and
  completions (incl. "canceled by you" vs "canceled by peer" toasts).
- **Tray icon, autostart, close-to-tray** — runs quietly in the
  background so the receiver is always available. All toggles in Settings.
- **Persistent history** — last 200 transfers kept across restarts,
  visible from Settings → Recent activity.
- **Window state persistence** — size, position, maximized state come
  back where you left them.
- **In-app update check** — startup probe of the GitHub Releases API
  (12-hour throttle, dismiss-per-version); also manual "Check now" in
  Settings.
- **10 UI languages**: English · Türkçe · Español · Deutsch · Français ·
  日本語 · 中文 · Русский · Português · العربية (RTL).
- **Theme override** — system / light / dark.

## Tech stack

| Layer | Choice | Why |
|---|---|---|
| Desktop framework | **Tauri 2** | Native Rust core, web view for UI. ~3 MB installer. |
| Backend language | **Rust** (stable) | Zero-cost async I/O via tokio, no GC pauses on the data path. |
| Async runtime | **tokio** | Per-file pipelined tasks (reader/writer split via `mpsc`), `tokio::select!` for cancel + pause. |
| Hashing | **blake3** | Faster than SHA-2 on modern CPUs; doesn't bottleneck a gigabit link. |
| Discovery | **mdns-sd** | Standard DNS-SD over multicast, no central server. |
| Sockets | **socket2** | `SO_SNDBUF` / `SO_RCVBUF` tuning. |
| Disk-space probe | **fs2** | Cross-platform `available_space`. |
| Windows native | **windows-rs**, **winreg**, **window-vibrancy** | `ITaskbarList3` for the taskbar overlay, DWM registry for the accent colour, Mica fallback. |
| Tauri plugins | `notification`, `autostart`, `dialog`, `window-state`, `single-instance` | OS toasts, login autostart, native file picker, remember window size, hand argv from second launch to the running instance. |
| Frontend | **React 18 + TypeScript + Vite** | Familiar, fast HMR. |
| UI library | **Fluent UI React v9** | Microsoft's official Fluent Design components — closest to WinUI 3 in a web view. |
| i18n | **i18next + react-i18next** | 10 bundled locales with pluralization + RTL handling. |
| Installer | **NSIS** (via `tauri-bundler`) | Per-user or all-users, multi-language wizard, LZMA-compressed, custom hooks for the shell context menu. |

## How to use it

1. **First run** — pick a device name and a default destination folder.
2. The main screen shows your **6-digit pairing code** in the top right.
   Share it with the sender.
3. As the sender:
   - drop files / folders onto the window (or right-click → "Send via
     LanBlaze" in Explorer),
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

`HELLO` carries the sender's device name, OS, file count, total bytes,
the receiver's pairing code, and the manifest. `HELLO_ACK` is
`{ accept, reason? }`. Files below 8 MiB skip the resume handshake to
keep the small-files pipeline at zero extra RTTs.

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

- [x] Tauri 2 + React/TS + Fluent UI scaffold
- [x] Single-file transfer, blake3, atomic write
- [x] Many-file / folder pipeline (no per-file ack)
- [x] mDNS discovery + refresh button
- [x] Setup wizard, pairing code, drag-and-drop, incoming dialog
- [x] Resume for interrupted large-file transfers
- [x] NSIS installer (multi-language, custom artwork)
- [x] 10 UI languages
- [x] Background-friendly (tray, autostart, close-to-tray, notifications)
- [x] Persistent history + trust list + disk-space pre-check
- [x] Cancel + Pause / Resume from either side
- [x] Live per-direction rates (network vs disk) + labeled metric grid + d/h/m/s ETA
- [x] In-app update check (GitHub Releases API)
- [x] Custom Mica title bar + system accent color + window state persistence
- [x] Windows taskbar progress overlay
- [x] Windows Explorer "Send via LanBlaze" right-click menu
- [ ] **Next:** TLS encryption (`tokio-rustls` + `rcgen` self-signed)
- [ ] Cross-platform builds (Linux, macOS) via GitHub Actions
- [ ] Signed `tauri-plugin-updater` auto-install
- [ ] Concurrent file open (K-deep prefetch) for tiny-files throughput

## Contributing

Issues and pull requests welcome. The project follows Conventional
Commits and ships everything English (commit messages, code comments,
docs); the UI is fully localized.

## License

No license attached yet — ask before redistributing.
