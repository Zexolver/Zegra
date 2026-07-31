# Roadmap

## Alpha - before V1

- [x] Add game library functionality (local scanning, not a fake/mock list)

## V1

- [x] Steam library detection (`libraryfolders.vdf` + `appmanifest_*.acf`)
- [x] GOG library detection (`goggame-*.info`)
- [x] Epic Games library detection via the Legendary/Heroic `installed.json` format
- [x] itch.io owned-games integration (personal API key, real `api.itch.io` endpoint)
- [x] GameJolt: investigated and honestly marked unsupported (no viable public API — see
      `API-Implementation.md`)
- [x] Working Library/Store/Settings UI with real search filtering, not just a static mockup
- [x] Custom user CSS theme loading (with a live preview flow)
- [x] Per-platform extra scan-location configuration
- [x] Automated tests: Rust unit tests for every scanner/client, Playwright integration tests for
      the frontend, plus a manual full end-to-end pass of the compiled desktop app
- [ ] Wine/Proton integration on Linux (not started)
- [ ] SteamDB / ProtonDB / WineDB data integration (not started)
- [ ] Storefront/catalog browsing — blocked on official store APIs Zegra doesn't have access to

## V2 — Modding support

- [x] Nexus Mods + CurseForge browsing/downloading, via a companion Tauri window pointed at the real
      sites — no API partnership needed, verified live (the window genuinely loads nexusmods.com).
      Opens automatically on entering the Mods tab and remembers the last site used; true inline
      (pixel-embedded) browsing was attempted first but found not to work on Linux, and pixel-locking
      the companion window's *position* to the Mods tab placeholder was attempted next and also found
      not to work reliably (WebKitGTK rendered a black surface, or the window manager silently
      ignored the reposition) — see `API-Implementation.md` for both investigations. The window is
      sized to match the placeholder and reused across visits, just not locked to its exact position.
- [x] `nxm://` link handling (Nexus Mods' "Mod Manager Download" buttons): OS protocol registration
      via `tauri-plugin-deep-link` + `tauri-plugin-single-instance`, resolved into real download
      URLs via the documented Nexus Mods API with a personal API key — resolution verified live
      against the real API; OS-level scheme registration itself couldn't be exercised in this
      sandboxed container (no persistent desktop session, no `xdg-mime`) — see
      `API-Implementation.md`
- [x] Custom mod sites: Settings lets you add any other modding website (name + `http(s)://` URL),
      validated against scheme injection (`javascript:`, `file:`, `nxm:`, etc. rejected client- and
      server-side), each getting its own Mods tab alongside Nexus Mods/CurseForge with the same
      companion-window behavior
- [ ] Plugin API for the launcher (not started)

## V2.5 — Minecraft

- [ ] Minecraft instance/profile management (not started)
- [ ] Modrinth mod integration (not started)

## V3+

Too far ahead to plan in detail. The README also floats wallet/crypto payment support as a
longer-term "if research proves it possible" idea — that would need real payment-processor
integration and compliance work, so it isn't scoped here yet.
