# Zegra
Zegra is the ultimate game launcher, for desktop, mobile and web. It utilizes the power of Tauri to ensure cross-platform compatibility, which also allows for rust to be used in its backend, therefore making it extremely lightweight and effecient to use, so you can squeeze out as much performance as you can for your games.
(Zegra is made with the help of AI, so it's not perfect. By using Zegra you are fine to use this software that was made with the help of AI).

Zegra helps to combine all launchers and services into one place, so you never have to install multiple launchers to play your games. Zegra will also be available for the web browser (WIP right now) to allow you to play games that were made playable in the web from different platforms, like itch.io or GameJolt. Zegra also will come in app form for both desktop and mobile phones (NOTE: Making Zegra for phones is utilizing less mature software. Tauri for mobile is very young, so many problems could arise.)
Zegra plans to take the following platforms for games and integrate them into itself (in no particular order) (more may be planed to be included as time continues);
 - Steam
 - Epic Games
 - GOG
 - Itch.io
 - GameJolt

Zegra also plans to add support for using wine/proton on linux in a similar manner to how Steam or The Heroic Launcher does.

Zegra will also eventually gain the ability for users to use their own CSS theme or layout file to customize their launcher. Not only that, but their will eventually be an API for plugins for the launcher as well.

#### Current Roadmap Vision:
- V1: Game library integration (Steam games, GOG games, etc.)
- V2: Add Modding support (Curseforge, Nexus Mods)
- V'2.5': Add integration for Minecraft and modding Minecraft (add Modrinth mods as well)
- ###### V3+: Too far ahead to plan any further.

#### V1 Roadmap ideals:
Whichever platforms are easiest to integrate into the launcher will get added first. With the more difficult ones being worked on after the easiest of them.

V1 is a means to get its raw and more basic components completed, integrated, and to make sure that they are in a stable state.

There will also be integration of platforms data, such as SteamDB or ModDB, and others for linux/Mac OS only, such as ProtonDB and WineDB.

Zegra also hopes to be the only launcher you need to install; It does not want to be like Cartridges where you still have to install the other launchers in order to play your games.

There will also be plans (if research proves it to be possible) to include wallet integration, to allow for different wallets to pay for games, as well as supporting crypto, and possibly converting crypto into other means of money if needed to help pay for games.

## Current status (V1)

Zegra is now a real, working Tauri v2 desktop app (`src-tauri/`) with a plain HTML/CSS/JS
frontend (`frontend/`) — no game data is faked or mocked. What actually works today:

- **Steam** — reads `libraryfolders.vdf` and per-game `appmanifest_*.acf` files (the same
  plain-text files Steam's own client uses), so Zegra can see installed games without any API key.
- **GOG** — reads the `goggame-*.info` files GOG's installer places next to every game.
- **Epic Games** — Epic has no Linux client or public "my library" API, so Zegra reads the
  `installed.json` format used by [Legendary](https://github.com/derrod/legendary) and Heroic
  Games Launcher instead — the standard community approach on Linux.
- **itch.io** — uses itch.io's real, documented [server-side API](https://itch.io/docs/api/serverside)
  (`/profile/owned-keys`) with a personal API key you generate yourself, no store partnership needed.
- **GameJolt** — honestly reported as *unsupported*. GameJolt has no public API for listing a
  user's owned/installed games (its Game API is scoped per-game via a private key, for
  trophies/scores only), so Zegra says so in the UI instead of faking data.
- **Custom themes** — Settings lets you point at any local `.css` file, which is validated and
  injected live into the app, plus a "preview before saving" flow.
- **Extra scan locations** — non-default install drives can be added per-platform in Settings.
- **Mods tab (V2, started)** — Nexus Mods and CurseForge are just websites, so Zegra opens the real
  site in its own window (tied to Zegra's main window); no scraping or unofficial API needed for
  browsing/downloading. Opening the Mods tab auto-opens the last site you used (no button click
  needed), and switching between Nexus Mods/CurseForge just shows/focuses the same reused window
  per site, so it's instant and keeps you logged in. Nexus Mods' "Mod Manager Download" buttons
  additionally use `nxm://` links, which Zegra can register itself as the OS handler for and
  resolve into real download URLs via a personal Nexus API key (with a manual "paste the link"
  fallback in the UI either way). See `API-Implementation.md` for exactly what's been verified live
  vs. what couldn't be exercised in this sandboxed environment (OS-level protocol registration) —
  and for why this is a companion window rather than content embedded inside the tab itself (a real
  Linux/Tauri limitation found while building this, not a design preference).

What's intentionally *not* built yet, and why:

- **Storefront browsing** (discovering new games to buy) — needs official store-catalog access
  from Steam/Epic/GOG that isn't available to third-party apps.
- **Wine/Proton integration, SteamDB/ProtonDB/WineDB data** — not started.
- **V2.5 Minecraft/Modrinth, plugin API, wallet/crypto payments** — Minecraft/Modrinth and a plugin
  API are plausible to build the same way modding was (no partnership required), just not done yet;
  wallet/crypto payment support needs real payment-processor integration and compliance work this
  codebase doesn't attempt.

See `Roadmap-Plan.md` for the itemized V1 checklist and `API-Implementation.md` for the
per-platform integration notes.

### Running it

```sh
cd src-tauri
cargo run            # launches the desktop app
cargo test            # runs the Rust unit test suite (scanners, settings, theme loader, itch.io client)
```

```sh
node --test tests/frontend.test.mjs   # frontend integration tests (Playwright + a real Chromium)
```