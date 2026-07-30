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

- [ ] Curseforge integration (not started — needs an API key/partnership)
- [ ] Nexus Mods integration (not started — needs an API key/partnership)
- [ ] Plugin API for the launcher (not started)

## V2.5 — Minecraft

- [ ] Minecraft instance/profile management (not started)
- [ ] Modrinth mod integration (not started)

## V3+

Too far ahead to plan in detail. The README also floats wallet/crypto payment support as a
longer-term "if research proves it possible" idea — that would need real payment-processor
integration and compliance work, so it isn't scoped here yet.
