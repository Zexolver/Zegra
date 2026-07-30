# Platform integration notes

None of these platforms offer an official "list a user's owned/installed games" SDK for
third-party launchers, so each one needed a different, community-standard workaround. All of them
are implemented in `src-tauri/src/platforms/`.

## Steam — done

No API needed. Steam's own client relies entirely on plain-text files: `steamapps/libraryfolders.vdf`
lists every library folder, and each library has one `appmanifest_<id>.acf` per installed game.
Zegra parses these directly (`src-tauri/src/vdf.rs` + `platforms/steam.rs`).

## GOG — done

GOG's installers drop a `goggame-<id>.info` JSON file next to every installed game (the same file
GOG Galaxy itself reads). Zegra walks common install directories for these files
(`platforms/gog.rs`).

## itch.io — done

itch.io has a real, documented server-side API (<https://itch.io/docs/api/serverside>).
`GET https://api.itch.io/profile/owned-keys` (Bearer auth with a personal API key from
`itch.io/user/settings/api-keys`) returns every game the user owns. No store partnership needed —
just the same kind of personal access token GitHub/GitLab use (`platforms/itchio.rs`).

## Epic Games — done, via a community format

Epic has no Linux client and no public library API at all. The de-facto standard on Linux is
[Legendary](https://github.com/derrod/legendary), an open-source, community-reverse-engineered
Epic client also used under the hood by Heroic Games Launcher. Both write an `installed.json` file
mapping app name → title/install path, which Zegra reads directly (`platforms/epic.rs`).

## GameJolt — investigated, not viable

GameJolt does not expose anything usable for "list this user's games":

- Its official [Game API](https://gamejolt.com/game-api/doc) is scoped *per game* behind that
  game's own private key, for trophies/scores/data-store — not a user-wide library.
- GameJolt discontinued its desktop client, so there's no local install-manifest format either
  (unlike Steam/GOG/Epic).
- There is no documented, generally-available catalog/library endpoint. Probing
  `gamejolt.com`'s own web endpoints during development (e.g. `site-api/web/discover/games/best`)
  either required parameters we don't have or returned nothing usable — and scraping undocumented
  internal endpoints isn't something Zegra relies on, since it could break at any time and was
  never meant for third-party use.

Zegra reports this honestly in the UI (`platforms/gamejolt.rs`) instead of faking a library.

## Difficulty, in hindsight

|            | Expected  | Actual                                              |
|------------|-----------|------------------------------------------------------|
| Itch.io    | Easiest   | Easiest — real official API                          |
| GameJolt   | Easiest   | **Not currently possible** — no usable API at all    |
| Steam      | Intermediate | Easy — well-documented local file formats         |
| GOG        | Hardest   | Easy — same idea as Steam, simpler file format       |
| Epic Games | Hardest   | Doable — via the community Legendary/Heroic format   |
