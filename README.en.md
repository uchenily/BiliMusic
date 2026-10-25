<div align="center">
  <img src="./src/assets/icon.png" width="112" height="112" alt="BiliMusic Logo" />
  <h1>BiliMusic</h1>
  <p><strong>Turn Bilibili into a polished, animated, Apple Music-inspired desktop music library.</strong></p>
  <p>
    <a href="./README.md">中文</a> ·
    <a href="https://github.com/HanversionOvO/BiliMusic">GitHub</a>
  </p>
  <p>
    <img alt="React" src="https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=111" />
    <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB?style=for-the-badge&logo=tauri&logoColor=fff" />
    <img alt="Vite" src="https://img.shields.io/badge/Vite-6-646CFF?style=for-the-badge&logo=vite&logoColor=fff" />
    <img alt="Rust" src="https://img.shields.io/badge/Rust-1.80+-000?style=for-the-badge&logo=rust&logoColor=fff" />
  </p>
</div>

## Overview

BiliMusic is a desktop music player built around Bilibili. It provides search, recommendations, rankings, creator browsing, queues, lyrics, favorites, playlists, QR login, and WebDAV synchronization.

The desktop shell now uses **Tauri 2 + Rust**. Electron and the HarmonyOS/OpenHarmony project have been removed. React owns the interface and playback experience, while Rust handles cross-origin requests, Bilibili cookies, lyric proxying, audio downloads, WebDAV, and the system tray.

## Features

- Bilibili video, user, recommendation, popular, and music-ranking views
- Correct multi-part audio playback and CID matching
- Bilibili CC/AI subtitles and QQ Music lyric matching
- Queue, shuffle, repeat, recent tracks, and favorites
- Custom playlists with JSON import/export
- QR login, persisted Bilibili session, and logout
- WebDAV two-way sync with ETag conflict handling
- Native Tauri window controls, system tray, and fullscreen
- Windows, macOS, and Linux desktop bundles

## Architecture

```text
┌──────────────────────────────────────────────┐
│ React 19 + Vite + TypeScript                 │
│ Pages, player, lyrics, playlists, UI state    │
└──────────────────┬───────────────────────────┘
                   │ @tauri-apps/api invoke/event
┌──────────────────▼───────────────────────────┐
│ Tauri 2 Rust backend                         │
│ Bilibili / Lyrics / Download / WebDAV / Tray │
│ Cookie persistence / Window lifecycle        │
└──────────────────────────────────────────────┘
```

Desktop features are accessed through `src/services/tauriApi.ts`. The frontend still owns WBI signing, data mapping, lyric matching, and player behavior.

## Getting Started

### Requirements

- Node.js 20+
- npm
- Rust stable and Cargo
- WebView2 on Windows; the system dependencies required by Tauri on macOS/Linux

### Install

```bash
npm install
```

### Browser development

```bash
npm run dev
```

### Tauri development

```bash
npm run tauri:dev
```

### Build the frontend

```bash
npm run build
```

### Build desktop bundles

```bash
npm run tauri:build
```

Artifacts are written to `src-tauri/target/release/bundle/`.

## Scripts

| Command | Description |
| --- | --- |
| `npm run dev` | Start the Vite development server |
| `npm run build` | Build the React frontend |
| `npm run preview` | Preview the frontend build |
| `npm run tauri:dev` | Start the Tauri desktop app |
| `npm run tauri:build` | Build Tauri desktop installers |

## Project Structure

```text
BiliMusic
├─ src/
│  ├─ components/       Player, queue, lyrics, layout, shared UI
│  ├─ contexts/          Playback, auth, now-playing, playlist state
│  ├─ hooks/             Theme, settings, lyrics, auto-sync hooks
│  ├─ pages/             Discover, search, recommendations, playlists, settings
│  ├─ services/          Bilibili data, Tauri bridge, lyric matching
│  ├─ styles/            Global styles and the Apple Music design system
│  └─ utils/             Local storage, playlists, and WebDAV sync
├─ src-tauri/
│  ├─ src/lib.rs         Tauri commands, network proxy, and tray
│  ├─ src/main.rs        Desktop entry point
│  ├─ capabilities/      Tauri permission configuration
│  ├─ icons/             Application icons
│  └─ tauri.conf.json    Tauri window and bundle configuration
├─ .github/workflows/    Multi-platform build workflow
├─ vite.config.ts
└─ package.json
```

## Data and Privacy

Playback state, playlists, favorites, theme settings, and lyric caches are stored in WebView `localStorage`. The Tauri backend stores Bilibili cookies, WebDAV configuration, and downloaded files in the application data directory.

Electron session data and the old Electron WebDAV encryption format are not automatically reused. Existing users may need to scan the QR code again and re-enter their WebDAV password after migrating to Tauri.

## Development Checks

Before submitting changes, run:

```bash
npm run build
npx tsc --noEmit
cargo check --manifest-path src-tauri/Cargo.toml
```

## Disclaimer

BiliMusic is intended for learning, research, and personal use. It is not affiliated with, endorsed by, or commercially connected to Bilibili, Apple Music, Apple Inc., or any related service provider. Please respect platform terms, copyright rules, and local laws.
