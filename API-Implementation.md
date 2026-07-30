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

## Nexus Mods / CurseForge (modding, V2) — done, via a companion window

An earlier version of this doc assumed modding support needed an official API partnership. That
was wrong: **Nexus Mods and CurseForge are just websites**, and Tauri can open a real webview
window pointed at either one. The user searches, logs in, and downloads exactly as they would in a
normal browser tab — no scraping, no unofficial API, no partnership needed for that part. This was
verified live: the window genuinely loads `www.nexusmods.com` (Cloudflare bot-check and all, since
it's real browser traffic hitting the real site).

That window is a separate, real OS window — parented to Zegra's main window via
`WebviewWindowBuilder::parent()` — rather than content embedded pixel-for-pixel inside the Mods
tab. That distinction matters and was a deliberate course-correction made *during* this work: the
first implementation tried true inline embedding via Tauri's child-webview API
(`Window::add_child`, gated behind the `unstable` cargo feature), and hands-on testing under Xvfb
showed it doesn't work as hoped on Linux. Tracing through `tauri-runtime-wry`'s GTK backend
(`default_vbox()` in `tauri-runtime-wry`, and `add_to_container` in `wry`) shows that on Linux,
*every* child webview — regardless of the position/size passed to `add_child` — gets packed into
the same vertical `GtkBox` that already holds the window's main content, via
`gtk::Box::pack_start`. Only a `GtkFixed` container respects arbitrary x/y/width/height, and Tauri
never uses one for this. In practice this meant the "embedded" browser ended up sharing space with
the rest of the UI via box-packing (full window width, an arbitrary height, position always
`(0, 0)`) instead of sitting inside the small placeholder area it was supposed to cover — confirmed
by logging the webview's actual settled bounds and comparing them to what was requested. Given that,
a parented companion window is the honest, actually-working way to deliver the UX goal (one click,
no popup-juggling, remembers where you left off) on this platform today; see `commands.rs` for the
`set_active_mod_site` command and `main.js`'s Mods section for the tab-switching/auto-open logic
built around it.

The one real wrinkle is specific to Nexus Mods: its "Mod Manager Download" buttons emit
`nxm://{game_domain}/mods/{mod_id}/files/{file_id}?key={key}&expires={expires}&user_id={user_id}`
links, which only do something if an app has registered itself as the OS's `nxm://` handler — that's
what Vortex and Mod Organizer 2 do, and it's real, official Nexus Mods behavior (not a workaround).
Zegra does the same via `tauri-plugin-deep-link` (+ `tauri-plugin-single-instance` with its
`deep-link` feature, since Linux/Windows relaunch a second process for a claimed scheme rather than
emitting an in-process event — see `src-tauri/src/lib.rs`). Resolving the link into a real
downloadable URL uses the documented Nexus Mods API
(`GET /v1/games/{domain}/mods/{id}/files/{file}/download_link.json`) with a personal API key from
`nexusmods.com/users/myaccount?tab=api` (`platforms/nexus.rs`) — confirmed live against the real
`api.nexusmods.com`, which correctly rejected a fake test key with "Please provide a valid API Key".
CurseForge has no equivalent wrinkle; its downloads are just links on the page.

**What isn't verified**: OS-level `nxm://` scheme registration itself (writing the `.desktop` file
and running `xdg-mime`/`update-desktop-database`) couldn't be exercised end-to-end in the sandboxed
container this was built in — it has no persistent desktop session for a real OS-level protocol
handoff to be observed, and the container lacks `xdg-mime`. The manual "paste an nxm:// link" field
on the Mods tab (`resolve_and_open_nxm_link`) exercises the exact same resolution code path and was
verified live, so the gap is specifically in the OS registration step, not the resolution logic.

## Difficulty, in hindsight

|            | Expected  | Actual                                              |
|------------|-----------|------------------------------------------------------|
| Itch.io    | Easiest   | Easiest — real official API                          |
| GameJolt   | Easiest   | **Not currently possible** — no usable API at all    |
| Steam      | Intermediate | Easy — well-documented local file formats         |
| GOG        | Hardest   | Easy — same idea as Steam, simpler file format       |
| Epic Games | Hardest   | Doable — via the community Legendary/Heroic format   |
| Nexus Mods / CurseForge (modding) | Assumed to need an API partnership | Easy for browsing/downloading (companion window, no partnership needed); true inline embedding turned out not to work on Linux; `nxm://` handling adds real but manageable complexity |
