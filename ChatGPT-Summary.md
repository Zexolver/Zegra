# Zegra Launcher

**Zegra** is a cross-platform game launcher built on Tauri (Rust backend + a plain HTML/CSS/JS
frontend), designed to be lightweight and modular for expansion.

## 🎯 Purpose

Zegra combines game libraries from multiple platforms into one launcher, so players don't need to
install and juggle Steam, GOG, the Epic launcher, itch.io, and more separately.

## ✅ Current Features (V1)

- Sidebar with tabs: **Library**, **Store**, **Mods**, and **Settings**, with real tab persistence
- Local Steam, GOG, and Epic (via Legendary/Heroic) game detection — no fake/mock data
- itch.io owned-games integration via a personal API key
- GameJolt honestly reported as unsupported (no viable public API — see `API-Implementation.md`)
- Real search/filter over the detected library
- Custom user CSS theme loading, with live preview before saving
- Per-platform extra scan-location configuration
- Mods tab: auto-opening Nexus Mods/CurseForge companion windows (parented to the main window),
  plus `nxm://` download-link handling via the real Nexus Mods API (V2 work, started — see
  `API-Implementation.md`)
- Rust unit tests for every scanner/client, Playwright integration tests for the frontend

## 🛠 Planned Features

- Wine/Proton integration on Linux
- SteamDB/ProtonDB/WineDB data integration
- V2: plugin API for the launcher
- V2.5: Minecraft instance management + Modrinth integration
- Storefront/catalog browsing (blocked on official store API access)

## 💡 Tech Stack

- **Backend:** Rust + Tauri v2
- **Frontend:** plain HTML/CSS/JS (no framework, no build step) — same file served both by the
  desktop app and, for local frontend-only testing, a small static file server
- **Tests:** `cargo test` for the backend, Playwright (driving real Chromium) for the frontend

## 📌 Notes

The original prototype was a browser-only mockup with hardcoded placeholder cards; it has since
been replaced by the real Tauri app described above, which actually detects installed games on
the machine it runs on.
