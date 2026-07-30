// Frontend integration tests, run with: node --test tests/frontend.test.mjs
//
// Drives the real static frontend (frontend/index.html + main.js) in a real
// Chromium instance via Playwright. Since there's no live Tauri backend in
// this environment, `window.__ZEGRA_TEST_INVOKE__` is injected before page
// scripts run to stand in for `window.__TAURI__.core.invoke`, exercising the
// exact same frontend code path a real desktop run would use.

import test from "node:test";
import assert from "node:assert/strict";
import { execSync } from "node:child_process";
import { createRequire } from "node:module";
import { startServer } from "./static-server.mjs";

const globalModules = execSync("npm root -g").toString().trim();
const require = createRequire(import.meta.url);
const { chromium } = require(`${globalModules}/playwright`);

const FIXTURE_SCAN_RESULT = {
  games: [
    {
      id: "steam:440",
      name: "Team Fortress 2",
      platform: "steam",
      install_path: "/home/user/.steam/steam/steamapps/common/Team Fortress 2",
      cover_url: null,
      installed: true,
    },
    {
      id: "gog:1495134320",
      name: "The Witcher 3: Wild Hunt",
      platform: "gog",
      install_path: "/home/user/GOG Games/The Witcher 3",
      cover_url: null,
      installed: true,
    },
    {
      id: "itchio:12345",
      name: "Celeste Classic",
      platform: "itchio",
      install_path: "https://x.itch.io/celeste-classic",
      cover_url: null,
      installed: false,
    },
  ],
  platform_results: [
    { platform: "steam", status: { status: "ok" }, games: [] },
    { platform: "gog", status: { status: "ok" }, games: [] },
    { platform: "epic", status: { status: "ok" }, games: [] },
    { platform: "itchio", status: { status: "ok" }, games: [] },
    {
      platform: "gamejolt",
      status: { status: "unsupported", reason: "GameJolt has no public library API." },
      games: [],
    },
  ],
};

const FIXTURE_SETTINGS = {
  itchio_api_key: "existing-key",
  custom_theme_path: "/home/user/my-theme.css",
  extra_steam_roots: ["/mnt/games/SteamLibrary"],
  extra_gog_roots: [],
  extra_legendary_paths: [],
};

/** Builds the init script string that defines window.__ZEGRA_TEST_INVOKE__. */
function mockInvokeInitScript({
  scanResult,
  settings,
  themeCss,
  resolveNxmResult,
  resolveNxmError,
} = {}) {
  return `
    window.__zegraMockCalls = [];
    window.__ZEGRA_TEST_INVOKE__ = async (cmd, args) => {
      window.__zegraMockCalls.push({ cmd, args });
      if (cmd === "scan_library") return ${JSON.stringify(scanResult ?? null)};
      if (cmd === "get_settings") return ${JSON.stringify(settings ?? {})};
      if (cmd === "save_settings") return null;
      if (cmd === "load_custom_theme") return ${JSON.stringify(themeCss ?? null)};
      if (cmd === "preview_theme") return ${JSON.stringify(themeCss ?? null)};
      if (cmd === "set_active_mod_site") return null;
      if (cmd === "open_downloads_folder") return null;
      if (cmd === "resolve_and_open_nxm_link") {
        ${resolveNxmError ? `throw new Error(${JSON.stringify(resolveNxmError)});` : `return ${JSON.stringify(resolveNxmResult ?? null)};`}
      }
      throw new Error("unmocked command: " + cmd);
    };
  `;
}

async function withPage(t, initScript, fn) {
  const { server, url } = await startServer();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    if (initScript) {
      await page.addInitScript(initScript);
    }
    await page.goto(url + "/index.html");
    await page.waitForSelector("#app");
    await fn(page, url);
  } finally {
    await browser.close();
    server.close();
  }
}

test("tab switching shows the right panel and persists across reload", async (t) => {
  await withPage(t, null, async (page) => {
    const isVisible = (sel) => page.isVisible(sel);

    assert.equal(await isVisible("#view-library"), true);
    assert.equal(await isVisible("#view-store"), false);

    await page.click('nav li[data-view="store"]');
    assert.equal(await isVisible("#view-store"), true);
    assert.equal(await isVisible("#view-library"), false);

    await page.click('nav li[data-view="settings"]');
    assert.equal(await isVisible("#view-settings"), true);

    await page.reload();
    await page.waitForSelector("#app");
    assert.equal(await isVisible("#view-settings"), true, "last active tab should persist via localStorage");
  });
});

test("without a backend, the runtime badge warns and scanning shows a graceful message", async (t) => {
  await withPage(t, null, async (page) => {
    const badgeText = await page.textContent("#runtime-badge");
    assert.match(badgeText, /browser preview/);

    await page.click("#scan-btn");
    const msg = await page.textContent("#library-empty-msg");
    assert.match(msg, /requires running inside the Zegra desktop app/);
  });
});

test("scanning renders games, platform status pills, and search filters results", async (t) => {
  const initScript = mockInvokeInitScript({ scanResult: FIXTURE_SCAN_RESULT });
  await withPage(t, initScript, async (page) => {
    await page.click("#scan-btn");
    await page.waitForSelector("#game-grid .game-card");

    const cardCount = await page.locator("#game-grid .game-card").count();
    assert.equal(cardCount, 3);

    const titles = await page.locator("#game-grid .game-card h3").allTextContents();
    assert.deepEqual(titles.sort(), [
      "Celeste Classic",
      "Team Fortress 2",
      "The Witcher 3: Wild Hunt",
    ]);

    // Platform status bar should surface the GameJolt "unsupported" pill.
    const statusText = await page.textContent("#platform-status-bar");
    assert.match(statusText, /GameJolt/);
    assert.match(statusText, /unsupported/);

    // Search filter: only Steam's game should remain visible.
    await page.fill("#search-input", "team fortress");
    const visibleTitles = await page
      .locator("#game-grid .game-card:not(.hidden) h3")
      .allTextContents();
    assert.deepEqual(visibleTitles, ["Team Fortress 2"]);

    const hiddenCount = await page.locator("#game-grid .game-card.hidden").count();
    assert.equal(hiddenCount, 2);

    // Clearing the search shows everything again.
    await page.fill("#search-input", "");
    const allVisibleAgain = await page.locator("#game-grid .game-card:not(.hidden)").count();
    assert.equal(allVisibleAgain, 3);
  });
});

test("settings form loads existing settings and round-trips edits on save", async (t) => {
  const initScript = mockInvokeInitScript({ settings: FIXTURE_SETTINGS });
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="settings"]');

    await page.waitForFunction(() => document.querySelector("#itchio-api-key").value.length > 0);
    assert.equal(await page.inputValue("#itchio-api-key"), "existing-key");
    assert.equal(await page.inputValue("#custom-theme-path"), "/home/user/my-theme.css");
    assert.equal(await page.inputValue("#extra-steam-roots"), "/mnt/games/SteamLibrary");

    await page.fill("#itchio-api-key", "new-key-123");
    await page.fill("#extra-gog-roots", "/mnt/games/GOG\n/mnt/games/GOG2");
    await page.click('#settings-form button[type="submit"]');

    await page.waitForFunction(() => document.getElementById("settings-status").textContent.includes("saved"));

    const savedPayload = await page.evaluate(() => {
      const call = window.__zegraMockCalls.find((c) => c.cmd === "save_settings");
      return call.args.settings;
    });
    assert.equal(savedPayload.itchio_api_key, "new-key-123");
    assert.deepEqual(savedPayload.extra_gog_roots, ["/mnt/games/GOG", "/mnt/games/GOG2"]);
    assert.deepEqual(savedPayload.extra_steam_roots, ["/mnt/games/SteamLibrary"]);
  });
});

test("applying a custom theme previews the path currently typed in the form", async (t) => {
  const themeCss = "body { background: hotpink !important; }";
  const initScript = mockInvokeInitScript({ themeCss });
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="settings"]');
    await page.fill("#custom-theme-path", "/home/user/my-theme.css");
    await page.click("#apply-theme-btn");

    await page.waitForFunction(
      (expected) => document.getElementById("custom-theme").textContent === expected,
      themeCss
    );
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    assert.equal(bg, "rgb(255, 105, 180)"); // hotpink

    const previewCall = await page.evaluate(() =>
      window.__zegraMockCalls.find((c) => c.cmd === "preview_theme")
    );
    assert.equal(previewCall.args.path, "/home/user/my-theme.css");

    await page.click("#reset-theme-btn");
    const styleContent = await page.textContent("#custom-theme");
    assert.equal(styleContent, "");
  });
});

test("a saved theme is auto-applied on startup", async (t) => {
  const themeCss = "body { background: rgb(10, 20, 30) !important; }";
  const initScript = mockInvokeInitScript({
    settings: { ...FIXTURE_SETTINGS, custom_theme_path: "/home/user/my-theme.css" },
    themeCss,
  });
  await withPage(t, initScript, async (page) => {
    await page.waitForFunction(
      (expected) => document.getElementById("custom-theme").textContent === expected,
      themeCss
    );
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    assert.equal(bg, "rgb(10, 20, 30)");
  });
});

test("applying a theme with an empty path shows an error instead of calling the backend", async (t) => {
  const initScript = mockInvokeInitScript({});
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="settings"]');
    await page.fill("#custom-theme-path", "");
    await page.click("#apply-theme-btn");
    const status = await page.textContent("#settings-status");
    assert.match(status, /Enter a theme file path/);
    const previewCalls = await page.evaluate(() =>
      window.__zegraMockCalls.filter((c) => c.cmd === "preview_theme")
    );
    assert.equal(previewCalls.length, 0);
  });
});

test("Mods tab: entering it auto-opens Nexus Mods, with no extra click needed", async (t) => {
  const initScript = mockInvokeInitScript({});
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="mods"]');
    assert.equal(await page.isVisible("#view-mods"), true);

    // The site should open automatically — this is the whole point of the
    // redesign: no "Open Nexus Mods" button click required.
    await page.waitForFunction(() =>
      window.__zegraMockCalls.some((c) => c.cmd === "set_active_mod_site" && c.args.site === "nexusmods")
    );
    assert.equal(await page.locator('.mod-site-tab[data-site="nexusmods"]').getAttribute("class"), "mod-site-tab active");

    const firstCall = await page.evaluate(() =>
      window.__zegraMockCalls.find((c) => c.cmd === "set_active_mod_site")
    );
    assert.equal(firstCall.args.site, "nexusmods");

    const status = await page.textContent("#mods-status");
    assert.match(status, /Nexus Mods is open in its own window/);
  });
});

test("Mods tab: switching site tabs is a single click, and switches which tab is marked active", async (t) => {
  const initScript = mockInvokeInitScript({});
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="mods"]');
    await page.waitForFunction(() =>
      window.__zegraMockCalls.some((c) => c.cmd === "set_active_mod_site")
    );

    await page.click('.mod-site-tab[data-site="curseforge"]');
    await page.waitForFunction(() =>
      window.__zegraMockCalls.some((c) => c.cmd === "set_active_mod_site" && c.args.site === "curseforge")
    );

    assert.match(
      await page.locator('.mod-site-tab[data-site="curseforge"]').getAttribute("class"),
      /active/
    );
    assert.doesNotMatch(
      await page.locator('.mod-site-tab[data-site="nexusmods"]').getAttribute("class"),
      /active/
    );
  });
});

test("Mods tab: leaving the tab hides the embedded webview, and returning restores the last site used", async (t) => {
  const initScript = mockInvokeInitScript({});
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="mods"]');
    await page.waitForFunction(() =>
      window.__zegraMockCalls.some((c) => c.cmd === "set_active_mod_site" && c.args.site === "nexusmods")
    );
    await page.click('.mod-site-tab[data-site="curseforge"]');
    await page.waitForFunction(() =>
      window.__zegraMockCalls.some((c) => c.cmd === "set_active_mod_site" && c.args.site === "curseforge")
    );

    // Navigating away must hide the (native, always-on-top) embedded webview.
    await page.click('nav li[data-view="library"]');
    await page.waitForFunction(() =>
      window.__zegraMockCalls.some((c) => c.cmd === "set_active_mod_site" && c.args.site === null)
    );

    // Coming back should remember CurseForge, not reset to the default.
    await page.click('nav li[data-view="mods"]');
    await page.waitForFunction(() => {
      const calls = window.__zegraMockCalls.filter(
        (c) => c.cmd === "set_active_mod_site" && c.args.site !== null
      );
      return calls.length >= 3;
    });
    const calls = await page.evaluate(() =>
      window.__zegraMockCalls
        .filter((c) => c.cmd === "set_active_mod_site")
        .map((c) => c.args.site)
    );
    assert.equal(calls[calls.length - 1], "curseforge");
  });
});

test("Mods tab: without a backend, shows a message instead of an embedded browser", async (t) => {
  await withPage(t, null, async (page) => {
    await page.click('nav li[data-view="mods"]');
    const msg = await page.textContent("#mods-embed-unavailable-msg");
    assert.match(msg, /requires running inside the Zegra desktop app/);
  });
});

test("Mods tab: downloads folder button calls open_downloads_folder", async (t) => {
  const initScript = mockInvokeInitScript({});
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="mods"]');
    await page.click("#open-downloads-btn");
    const calls = await page.evaluate(() =>
      window.__zegraMockCalls.filter((c) => c.cmd === "open_downloads_folder")
    );
    assert.equal(calls.length, 1);
  });
});

test("Mods tab: resolving a pasted nxm:// link shows success and logs the resolved URL", async (t) => {
  const initScript = mockInvokeInitScript({
    resolveNxmResult: "https://cdn.nexusmods.com/some-mod-file.zip",
  });
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="mods"]');
    await page.click(".mods-advanced summary");
    await page.fill(
      "#nxm-link-input",
      "nxm://skyrimspecialedition/mods/12345/files/67890?key=abc&expires=123"
    );
    await page.click("#resolve-nxm-btn");

    await page.waitForFunction(() =>
      document.getElementById("nxm-status").textContent.includes("Resolved and opened")
    );
    const logText = await page.textContent("#nxm-log");
    assert.match(logText, /cdn\.nexusmods\.com\/some-mod-file\.zip/);

    // The input clears after a successful resolve.
    assert.equal(await page.inputValue("#nxm-link-input"), "");

    const call = await page.evaluate(() =>
      window.__zegraMockCalls.find((c) => c.cmd === "resolve_and_open_nxm_link")
    );
    assert.equal(
      call.args.nxmUrl,
      "nxm://skyrimspecialedition/mods/12345/files/67890?key=abc&expires=123"
    );
  });
});

test("Mods tab: a failed resolve shows the error and doesn't clear the input", async (t) => {
  const initScript = mockInvokeInitScript({
    resolveNxmError: "no Nexus Mods API key configured in Settings",
  });
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="mods"]');
    await page.click(".mods-advanced summary");
    await page.fill("#nxm-link-input", "nxm://skyrim/mods/1/files/2");
    await page.click("#resolve-nxm-btn");

    await page.waitForFunction(() =>
      document.getElementById("nxm-status").textContent.includes("Failed to resolve link")
    );
    const status = await page.textContent("#nxm-status");
    assert.match(status, /no Nexus Mods API key configured/);
    assert.equal(await page.inputValue("#nxm-link-input"), "nxm://skyrim/mods/1/files/2");
  });
});

test("Mods tab: resolving with an empty link shows an error without calling the backend", async (t) => {
  const initScript = mockInvokeInitScript({});
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="mods"]');
    await page.click(".mods-advanced summary");
    await page.fill("#nxm-link-input", "");
    await page.click("#resolve-nxm-btn");
    const status = await page.textContent("#nxm-status");
    assert.match(status, /Paste an nxm:\/\/ link first/);
    const calls = await page.evaluate(() =>
      window.__zegraMockCalls.filter((c) => c.cmd === "resolve_and_open_nxm_link")
    );
    assert.equal(calls.length, 0);
  });
});

test("Settings: Nexus API key field loads and round-trips on save", async (t) => {
  const initScript = mockInvokeInitScript({
    settings: { nexus_api_key: "existing-nexus-key" },
  });
  await withPage(t, initScript, async (page) => {
    await page.click('nav li[data-view="settings"]');
    await page.waitForFunction(() => document.querySelector("#nexus-api-key").value.length > 0);
    assert.equal(await page.inputValue("#nexus-api-key"), "existing-nexus-key");

    await page.fill("#nexus-api-key", "new-nexus-key");
    await page.click('#settings-form button[type="submit"]');
    await page.waitForFunction(() =>
      document.getElementById("settings-status").textContent.includes("saved")
    );

    const savedPayload = await page.evaluate(() => {
      const call = window.__zegraMockCalls.find((c) => c.cmd === "save_settings");
      return call.args.settings;
    });
    assert.equal(savedPayload.nexus_api_key, "new-nexus-key");
  });
});
