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

function initTabs() {
  $all("nav li[data-view]").forEach((item) => {
    item.addEventListener("click", () => switchView(item.dataset.view));
  });
  const saved = localStorage.getItem("activeView") || "library";
  switchView(saved);
}

function switchView(view) {
  $all("nav li[data-view]").forEach((item) => {
    item.classList.toggle("active", item.dataset.view === view);
  });
  $all(".view").forEach((panel) => {
    panel.classList.toggle("hidden", panel.dataset.viewPanel !== view);
  });
  localStorage.setItem("activeView", view);
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

async function loadSettingsIntoForm() {
  if (!hasBackend) return;
  try {
    const settings = await invoke("get_settings");
    $("#itchio-api-key").value = settings.itchio_api_key || "";
    $("#nexus-api-key").value = settings.nexus_api_key || "";
    $("#custom-theme-path").value = settings.custom_theme_path || "";
    $("#extra-steam-roots").value = arrayToLines(settings.extra_steam_roots);
    $("#extra-gog-roots").value = arrayToLines(settings.extra_gog_roots);
    $("#extra-legendary-paths").value = arrayToLines(settings.extra_legendary_paths);
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
    setSettingsStatus("Settings saved.", false);
  } catch (err) {
    setSettingsStatus(`Failed to save settings: ${err}`, true);
  }
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

async function openModSite(site) {
  if (!hasBackend) {
    setNxmStatus("Cannot open an embedded browser — no backend available in this preview.", true);
    return;
  }
  try {
    await invoke("open_mod_site", { site });
  } catch (err) {
    setNxmStatus(`Failed to open ${site}: ${err}`, true);
  }
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
  initTabs();
  initRuntimeBadge();
  $("#scan-btn").addEventListener("click", scanLibrary);
  $("#search-input").addEventListener("input", applySearchFilter);
  $("#settings-form").addEventListener("submit", saveSettings);
  $("#apply-theme-btn").addEventListener("click", applyCustomTheme);
  $("#reset-theme-btn").addEventListener("click", resetTheme);
  $("#open-nexusmods-btn").addEventListener("click", () => openModSite("nexusmods"));
  $("#open-curseforge-btn").addEventListener("click", () => openModSite("curseforge"));
  $("#open-downloads-btn").addEventListener("click", openDownloadsFolder);
  $("#resolve-nxm-btn").addEventListener("click", resolveNxmLink);
  loadSettingsIntoForm();
  applySavedThemeOnStartup();
  initNxmEventListener();
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
