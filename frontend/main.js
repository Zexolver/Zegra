// main.js — Zegra frontend logic.
//
// Runs two ways: for real, inside the Tauri desktop app (window.__TAURI__ is
// injected because tauri.conf.json sets app.withGlobalTauri = true), or in a
// plain browser for local preview/testing, where there is no native backend
// to call. `window.__ZEGRA_TEST_INVOKE__` lets automated browser tests
// (Playwright) inject a fake backend without needing the real desktop app.

const TAURI = window.__TAURI__;
const invoke = TAURI ? TAURI.core.invoke : window.__ZEGRA_TEST_INVOKE__ || null;
const hasBackend = typeof invoke === "function";

const state = {
  games: [],
  platformResults: [],
};

function $(selector) {
  return document.querySelector(selector);
}
function $all(selector) {
  return document.querySelectorAll(selector);
}

// ---------- View / tab switching ----------

let currentView = null;

function initTabs() {
  $all("nav li[data-view]").forEach((item) => {
    item.addEventListener("click", () => switchView(item.dataset.view));
  });
  const saved = localStorage.getItem("activeView") || "library";
  switchView(saved);
}

function switchView(view) {
  const previousView = currentView;
  currentView = view;

  $all("nav li[data-view]").forEach((item) => {
    item.classList.toggle("active", item.dataset.view === view);
  });
  $all(".view").forEach((panel) => {
    panel.classList.toggle("hidden", panel.dataset.viewPanel !== view);
  });
  localStorage.setItem("activeView", view);

  if (view === "mods") {
    // Auto-show the last-used (or default) mod site the moment this tab is
    // opened — no "Open Nexus Mods" click required.
    const site = activeModSite || localStorage.getItem("activeModSite") || "nexusmods";
    showModSite(site);
  } else if (previousView === "mods") {
    // The mod-site window is a separate window, not embedded content, so it
    // doesn't need to be hidden when leaving this tab the way a truly
    // embedded webview would — but we still hide it so it doesn't linger on
    // top of Zegra's main window while the user works in another tab.
    hideAllModSites();
  }
}

// ---------- Runtime badge ----------

function initRuntimeBadge() {
  const badge = $("#runtime-badge");
  if (TAURI) {
    badge.textContent = "Zegra desktop";
    badge.className = "badge badge-ok";
  } else if (hasBackend) {
    badge.textContent = "test harness";
    badge.className = "badge badge-ok";
  } else {
    badge.textContent = "browser preview (no backend)";
    badge.className = "badge badge-warn";
  }
}

// ---------- Library ----------

const PLATFORM_LABELS = {
  steam: "Steam",
  gog: "GOG",
  epic: "Epic",
  itchio: "itch.io",
  gamejolt: "GameJolt",
};

function platformLabel(platform) {
  return PLATFORM_LABELS[platform] || platform;
}

function renderPlatformStatus(platformResults) {
  const bar = $("#platform-status-bar");
  bar.innerHTML = "";
  const noteworthy = platformResults.filter((r) => r.status.status !== "ok");
  if (noteworthy.length === 0) {
    bar.hidden = true;
    return;
  }
  bar.hidden = false;
  noteworthy.forEach((r) => {
    const pill = document.createElement("div");
    pill.className = `status-pill status-${r.status.status}`;
    const reason = r.status.reason || r.status.message || "";
    pill.textContent = `${platformLabel(r.platform)}: ${r.status.status}${reason ? " — " + reason : ""}`;
    bar.appendChild(pill);
  });
}

function renderGames(games) {
  const grid = $("#game-grid");
  const emptyMsg = $("#library-empty-msg");
  grid.innerHTML = "";
  if (games.length === 0) {
    emptyMsg.hidden = false;
    emptyMsg.textContent = "No games were found on this machine for any configured platform.";
    return;
  }
  emptyMsg.hidden = true;

  games.forEach((game) => {
    const card = document.createElement("div");
    card.className = "game-card";
    card.dataset.gameId = game.id;
    card.dataset.gameName = game.name.toLowerCase();

    const img = document.createElement("img");
    img.src = game.cover_url || "placeholder.svg";
    img.alt = game.name;
    img.addEventListener("error", () => {
      img.src = "placeholder.svg";
    });

    const title = document.createElement("h3");
    title.textContent = game.name;

    const badge = document.createElement("span");
    badge.className = `platform-badge platform-${game.platform}`;
    badge.textContent = platformLabel(game.platform);

    card.append(img, title, badge);
    grid.appendChild(card);
  });
}

function applySearchFilter() {
  const query = $("#search-input").value.trim().toLowerCase();
  $all("#game-grid .game-card").forEach((card) => {
    const matches = !query || card.dataset.gameName.includes(query);
    card.classList.toggle("hidden", !matches);
  });
}

async function scanLibrary() {
  const btn = $("#scan-btn");
  const emptyMsg = $("#library-empty-msg");

  if (!hasBackend) {
    emptyMsg.hidden = false;
    emptyMsg.textContent =
      "Library scanning requires running inside the Zegra desktop app — this is a plain browser preview with no backend.";
    return;
  }

  btn.disabled = true;
  btn.textContent = "Scanning…";
  try {
    const result = await invoke("scan_library");
    state.games = result.games;
    state.platformResults = result.platform_results;
    renderGames(state.games);
    renderPlatformStatus(state.platformResults);
    applySearchFilter();
  } catch (err) {
    emptyMsg.hidden = false;
    emptyMsg.textContent = `Scan failed: ${err}`;
  } finally {
    btn.disabled = false;
    btn.textContent = "Scan Library";
  }
}

// ---------- Settings ----------

function linesToArray(text) {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function arrayToLines(arr) {
  return (arr || []).join("\n");
}

// The last-loaded/saved settings, kept around so the Mods tab can render
// tabs for the user's custom sites without an extra round-trip every time.
let cachedSettings = null;

async function loadSettingsIntoForm() {
  if (!hasBackend) return;
  try {
    const settings = await invoke("get_settings");
    cachedSettings = settings;
    $("#itchio-api-key").value = settings.itchio_api_key || "";
    $("#nexus-api-key").value = settings.nexus_api_key || "";
    $("#custom-theme-path").value = settings.custom_theme_path || "";
    $("#extra-steam-roots").value = arrayToLines(settings.extra_steam_roots);
    $("#extra-gog-roots").value = arrayToLines(settings.extra_gog_roots);
    $("#extra-legendary-paths").value = arrayToLines(settings.extra_legendary_paths);
    renderCustomModSitesList(settings.custom_mod_sites || []);
    renderModSiteTabs();
  } catch (err) {
    setSettingsStatus(`Failed to load settings: ${err}`, true);
  }
}

function readSettingsFromForm() {
  return {
    itchio_api_key: $("#itchio-api-key").value.trim() || null,
    nexus_api_key: $("#nexus-api-key").value.trim() || null,
    custom_theme_path: $("#custom-theme-path").value.trim() || null,
    extra_steam_roots: linesToArray($("#extra-steam-roots").value),
    extra_gog_roots: linesToArray($("#extra-gog-roots").value),
    extra_legendary_paths: linesToArray($("#extra-legendary-paths").value),
    custom_mod_sites: (cachedSettings && cachedSettings.custom_mod_sites) || [],
  };
}

function setSettingsStatus(msg, isError) {
  const el = $("#settings-status");
  el.textContent = msg;
  el.classList.toggle("status-error", !!isError);
  el.classList.toggle("status-ok", !isError);
}

async function saveSettings(event) {
  event.preventDefault();
  if (!hasBackend) {
    setSettingsStatus("Cannot save — no backend available in this preview.", true);
    return;
  }
  const settings = readSettingsFromForm();
  try {
    await invoke("save_settings", { settings });
    cachedSettings = settings;
    setSettingsStatus("Settings saved.", false);
  } catch (err) {
    setSettingsStatus(`Failed to save settings: ${err}`, true);
  }
}

// ---------- Custom mod sites (Settings) ----------

const BUILT_IN_MOD_SITES = [
  { id: "nexusmods", name: "Nexus Mods" },
  { id: "curseforge", name: "CurseForge" },
];

function slugify(name) {
  return name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** Returns a unique slug for `name`, avoiding collisions with built-in ids and any already-taken id in `takenIds`. */
function uniqueModSiteId(name, takenIds) {
  const base = slugify(name) || "site";
  if (!takenIds.has(base)) return base;
  let n = 2;
  while (takenIds.has(`${base}-${n}`)) n += 1;
  return `${base}-${n}`;
}

function setModSitesStatus(msg, isError) {
  const el = $("#mod-sites-status");
  el.textContent = msg;
  el.classList.toggle("status-error", !!isError);
  el.classList.toggle("status-ok", !isError);
}

function renderCustomModSitesList(customSites) {
  const list = $("#custom-mod-sites-list");
  list.innerHTML = "";
  if (customSites.length === 0) {
    const empty = document.createElement("p");
    empty.className = "hint";
    empty.textContent = "No custom sites added yet.";
    list.appendChild(empty);
    return;
  }
  customSites.forEach((site) => {
    const row = document.createElement("div");
    row.className = "custom-mod-site-row";

    const name = document.createElement("span");
    name.className = "custom-mod-site-name";
    name.textContent = site.name;

    const url = document.createElement("span");
    url.className = "custom-mod-site-url";
    url.textContent = site.url;

    const removeBtn = document.createElement("button");
    removeBtn.type = "button";
    removeBtn.className = "btn btn-small";
    removeBtn.textContent = "Remove";
    removeBtn.addEventListener("click", () => removeCustomModSite(site.id));

    row.append(name, url, removeBtn);
    list.appendChild(row);
  });
}

async function persistCustomModSites(customSites) {
  const settings = { ...readSettingsFromForm(), custom_mod_sites: customSites };
  await invoke("save_settings", { settings });
  cachedSettings = settings;
  renderCustomModSitesList(customSites);
  renderModSiteTabs();
}

async function addCustomModSite() {
  const nameInput = $("#new-mod-site-name");
  const urlInput = $("#new-mod-site-url");
  const name = nameInput.value.trim();
  let url = urlInput.value.trim();

  if (!hasBackend) {
    setModSitesStatus("Cannot add a site — no backend available in this preview.", true);
    return;
  }
  if (!name) {
    setModSitesStatus("Enter a name for the site.", true);
    return;
  }
  // Reject anything that already specifies a non-http(s) scheme (javascript:,
  // file:, nxm:, etc) up front, before the bare-domain convenience below
  // would otherwise turn e.g. "javascript:alert(1)" into the
  // superficially-valid-looking "https://javascript:alert(1)".
  if (/^[a-z][a-z0-9+.-]*:/i.test(url) && !/^https?:\/\//i.test(url)) {
    setModSitesStatus("Enter a valid http:// or https:// URL.", true);
    return;
  }
  if (url && !/^https?:\/\//i.test(url)) {
    url = `https://${url}`;
  }
  if (!/^https?:\/\/.+/i.test(url)) {
    setModSitesStatus("Enter a valid http:// or https:// URL.", true);
    return;
  }

  const existing = (cachedSettings && cachedSettings.custom_mod_sites) || [];
  const takenIds = new Set([
    ...BUILT_IN_MOD_SITES.map((s) => s.id),
    ...existing.map((s) => s.id),
  ]);
  const id = uniqueModSiteId(name, takenIds);
  const updated = [...existing, { id, name, url }];

  try {
    await persistCustomModSites(updated);
    nameInput.value = "";
    urlInput.value = "";
    setModSitesStatus(`Added ${name}.`, false);
  } catch (err) {
    setModSitesStatus(`Failed to add site: ${err}`, true);
  }
}

async function removeCustomModSite(id) {
  const existing = (cachedSettings && cachedSettings.custom_mod_sites) || [];
  const updated = existing.filter((s) => s.id !== id);
  try {
    await persistCustomModSites(updated);
    setModSitesStatus("Site removed.", false);
  } catch (err) {
    setModSitesStatus(`Failed to remove site: ${err}`, true);
  }
}

/** All mod sites (built-in + the user's custom ones) available for the Mods tab's tab bar. */
function allModSites() {
  const custom = (cachedSettings && cachedSettings.custom_mod_sites) || [];
  return [...BUILT_IN_MOD_SITES, ...custom.map((s) => ({ id: s.id, name: s.name }))];
}

// ---------- Theme ----------

async function applyCustomTheme() {
  const styleTag = $("#custom-theme");
  const path = $("#custom-theme-path").value.trim();
  if (!hasBackend) {
    setSettingsStatus("Cannot load theme — no backend available in this preview.", true);
    return;
  }
  if (!path) {
    setSettingsStatus("Enter a theme file path first.", true);
    return;
  }
  try {
    // Previews whatever path is currently typed, independent of saved settings.
    const css = await invoke("preview_theme", { path });
    styleTag.textContent = css;
    setSettingsStatus("Custom theme applied.", false);
  } catch (err) {
    setSettingsStatus(`Failed to load theme: ${err}`, true);
  }
}

/** Re-applies whichever theme was already saved, so it persists across restarts. */
async function applySavedThemeOnStartup() {
  if (!hasBackend) return;
  try {
    const css = await invoke("load_custom_theme");
    if (css) {
      $("#custom-theme").textContent = css;
    }
  } catch {
    // A saved-but-now-invalid theme path shouldn't block the rest of the app.
  }
}

function resetTheme() {
  $("#custom-theme").textContent = "";
  setSettingsStatus("Theme reset to default.", false);
}

// ---------- Mods ----------

// Which mod site (if any) is currently shown, and which ones have already
// finished loading once this session — so switching back to a site already
// visited shows instantly, with no "Opening…" flash, since the backend just
// re-shows/repositions the same live window instead of navigating it again.
let activeModSite = null;
const modSitesLoadedOnce = new Set();

function modSiteLabel(id) {
  const site = allModSites().find((s) => s.id === id);
  return site ? site.name : id;
}

function logNxmEvent(message, isError) {
  const log = $("#nxm-log");
  const entry = document.createElement("div");
  entry.className = isError ? "nxm-log-entry nxm-log-error" : "nxm-log-entry nxm-log-ok";
  entry.textContent = message;
  log.prepend(entry);
}

function setNxmStatus(msg, isError) {
  const el = $("#nxm-status");
  el.textContent = msg;
  el.classList.toggle("status-error", !!isError);
  el.classList.toggle("status-ok", !isError);
}

/** (Re)builds the Nexus Mods / CurseForge / custom-site tab buttons from `allModSites()`. */
function renderModSiteTabs() {
  const container = $("#mod-site-tabs");
  container.innerHTML = "";
  allModSites().forEach((site) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "mod-site-tab";
    btn.dataset.site = site.id;
    btn.setAttribute("role", "tab");
    btn.textContent = site.name;
    btn.classList.toggle("active", site.id === activeModSite);
    btn.addEventListener("click", () => showModSite(site.id));
    container.appendChild(btn);
  });
}

/** The on-screen size of the Mods tab's placeholder area, in logical (CSS) pixels — used to size the companion window to roughly match it. */
function modsContainerSize() {
  const rect = $("#mods-webview-container").getBoundingClientRect();
  return { width: rect.width, height: rect.height };
}

/**
 * Shows `site` in its own window (creating it the first time, otherwise just
 * re-showing/focusing the same one), so switching sites is instant and each
 * site's logged-in session is preserved across visits.
 *
 * This is a real, separate OS window rather than content embedded pixel-for
 * -pixel inside the Mods tab: on Linux, Tauri's child-webview positioning
 * API turned out not to be reliably controllable (see
 * `API-Implementation.md` for what testing that revealed). To still feel
 * like part of the same app, the window is undecorated and sized to roughly
 * match the Mods tab's placeholder area; `initModSiteTrackingListener` keeps
 * it sized to that as Zegra's own window resizes. Pinning its on-screen
 * *position* to the placeholder turned out not to be achievable — see the
 * backend command's doc comment and `API-Implementation.md` for why — so the
 * window manager places it, same as any other window.
 */
async function showModSite(site) {
  activeModSite = site;
  localStorage.setItem("activeModSite", site);
  $all(".mod-site-tab").forEach((tab) => {
    tab.classList.toggle("active", tab.dataset.site === site);
  });

  const unavailableMsg = $("#mods-embed-unavailable-msg");
  const status = $("#mods-status");
  const label = modSiteLabel(site);

  if (!hasBackend) {
    unavailableMsg.classList.remove("hidden");
    status.classList.add("hidden");
    return;
  }
  unavailableMsg.classList.add("hidden");
  status.classList.remove("hidden");
  status.textContent = modSitesLoadedOnce.has(site)
    ? `${label} is open.`
    : `Opening ${label}…`;

  const { width, height } = modsContainerSize();
  try {
    await invoke("set_active_mod_site", { site, width, height });
    modSitesLoadedOnce.add(site);
    // Outside the real desktop runtime there's no "mod-site-loaded" event to
    // wait for (see initModSiteLoadListener), so just settle the status now.
    if (!TAURI) {
      status.textContent = `${label} is open.`;
    }
  } catch (err) {
    setNxmStatus(`Failed to open ${label}: ${err}`, true);
    status.textContent = `Couldn't open ${label}.`;
  }
}

/** Hides every mod-site window — called whenever the user navigates away from the Mods tab. */
async function hideAllModSites() {
  if (!hasBackend) return;
  try {
    await invoke("set_active_mod_site", { site: null, width: 0, height: 0 });
  } catch {
    // Best-effort: nothing the user can act on if this fails.
  }
}

/** Re-sends the current site's placeholder size so the companion window stays roughly the same size — called on window resize. */
function resizeActiveModSiteIfVisible() {
  if (currentView !== "mods" || !activeModSite || !hasBackend) return;
  const { width, height } = modsContainerSize();
  invoke("set_active_mod_site", { site: activeModSite, width, height }).catch(() => {});
}

/** Updates the status line once the real window reports its page finished loading. */
function initModSiteLoadListener() {
  if (!TAURI) return;
  TAURI.event.listen("mod-site-loaded", (event) => {
    if (event.payload === activeModSite) {
      $("#mods-status").textContent = `${modSiteLabel(event.payload)} is open.`;
    }
  });
}

/**
 * Keeps the companion mod-site window sized to roughly match its placeholder
 * area as Zegra's own window resizes. Safe to listen on the generic scale
 * here (unlike the earlier true-inline-embedding attempt): the companion
 * window is a fully independent top-level window now, not a child widget
 * inside Zegra's own window, so resizing it can't itself trigger a resize of
 * Zegra's window and cause a feedback loop.
 */
function initModSiteTrackingListener() {
  if (!TAURI) return;
  let debounceHandle = null;
  const scheduleResize = () => {
    clearTimeout(debounceHandle);
    debounceHandle = setTimeout(resizeActiveModSiteIfVisible, 100);
  };
  const win = TAURI.window.getCurrentWindow();
  win.onResized(scheduleResize);
}

async function openDownloadsFolder() {
  if (!hasBackend) {
    setNxmStatus("Cannot open the downloads folder — no backend available in this preview.", true);
    return;
  }
  try {
    await invoke("open_downloads_folder");
  } catch (err) {
    setNxmStatus(`Failed to open downloads folder: ${err}`, true);
  }
}

async function resolveNxmLink() {
  const nxmUrl = $("#nxm-link-input").value.trim();
  if (!hasBackend) {
    setNxmStatus("Cannot resolve links — no backend available in this preview.", true);
    return;
  }
  if (!nxmUrl) {
    setNxmStatus("Paste an nxm:// link first.", true);
    return;
  }
  const btn = $("#resolve-nxm-btn");
  btn.disabled = true;
  try {
    const resolvedUrl = await invoke("resolve_and_open_nxm_link", { nxmUrl });
    setNxmStatus("Resolved and opened.", false);
    logNxmEvent(`Resolved: ${resolvedUrl}`, false);
    $("#nxm-link-input").value = "";
  } catch (err) {
    setNxmStatus(`Failed to resolve link: ${err}`, true);
    logNxmEvent(`Error: ${err}`, true);
  } finally {
    btn.disabled = false;
  }
}

/** Listens for automatic nxm:// deep-link handling (real OS-level clicks), when running under the real Tauri runtime. */
function initNxmEventListener() {
  if (!TAURI) return;
  TAURI.event.listen("nxm-download-result", (event) => {
    const message = event.payload;
    logNxmEvent(message, message.startsWith("error:"));
  });
}

// ---------- Wiring ----------

function init() {
  renderModSiteTabs(); // built-ins immediately; re-rendered with custom sites once settings load
  initTabs();
  initRuntimeBadge();
  $("#scan-btn").addEventListener("click", scanLibrary);
  $("#search-input").addEventListener("input", applySearchFilter);
  $("#settings-form").addEventListener("submit", saveSettings);
  $("#apply-theme-btn").addEventListener("click", applyCustomTheme);
  $("#reset-theme-btn").addEventListener("click", resetTheme);
  $("#add-mod-site-btn").addEventListener("click", addCustomModSite);
  $("#open-downloads-btn").addEventListener("click", openDownloadsFolder);
  $("#resolve-nxm-btn").addEventListener("click", resolveNxmLink);
  loadSettingsIntoForm();
  applySavedThemeOnStartup();
  initNxmEventListener();
  initModSiteLoadListener();
  initModSiteTrackingListener();
}

window.addEventListener("DOMContentLoaded", init);

// Exposed so automated browser tests can drive/inspect app state directly.
window.__zegraTestApi = {
  state,
  scanLibrary,
  applySearchFilter,
  switchView,
  renderGames,
  renderPlatformStatus,
};
