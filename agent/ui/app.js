/* The window. It only shows what the Rust side knows and passes on what is
 * changed in Settings - all the actual work happens in engine.rs, which keeps
 * running with this window closed.
 *
 * Talking to Rust goes two ways:
 *   invoke("name", { ... })   calls a function in commands.rs and waits for
 *                             its answer
 *   listen("status", fn)      runs fn every time the engine sends a new
 *                             status, which it does whenever anything changes
 *
 * The markup is the site's, so where something here does what a part of the
 * site does, it says which.
 */
(() => {
  "use strict";

  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;
  const { getVersion } = window.__TAURI__.app;

  const LISTENING = 2;

  // How long the page takes to slide back - the .45s in app.css.
  const REVEAL_MS = 450;

  // The apps that can be switched on in Settings, by the short name the
  // settings file uses for them. Anything else seen playing is added too.
  const KNOWN_PLAYERS = [
    ["cider", "Cider"],
    ["apple-music", "Apple Music app"],
    ["spotify", "Spotify"],
    ["firefox", "Firefox"],
    ["safari", "Safari"],
    ["chrome", "Chrome"],
  ];

  // Icons for the apps that have one in Font Awesome, found by the name an
  // activity is shared under - "Microsoft Edge" has "edge" in it. Cider and
  // Apple Music share the site's music note. Anything not listed gets its
  // card's plain icon instead: a note for music, a window for everything
  // else.
  const APP_ICONS = [
    ["apple music", "fa-brands fa-itunes-note"],
    ["cider", "fa-brands fa-itunes-note"],
    ["spotify", "fa-brands fa-spotify"],
    ["firefox", "fa-brands fa-firefox-browser"],
    ["chrome", "fa-brands fa-chrome"],
    ["edge", "fa-brands fa-edge"],
    ["safari", "fa-brands fa-safari"],
    ["youtube", "fa-brands fa-youtube"],
    ["twitch", "fa-brands fa-twitch"],
    ["discord", "fa-brands fa-discord"],
    ["steam", "fa-brands fa-steam"],
    ["visual studio code", "fa-solid fa-code"],
  ];

  const MUSIC_ICON = "fa-solid fa-music";
  const OTHER_ICON = "fa-regular fa-window-maximize";

  // How each source is doing, as the site's version list shows a library:
  // a badge, its colour ("current" is green, "outdated" orange, "unknown"
  // red, "pending" grey) and a line of explanation.
  const CIDER_STATES = {
    "no-token": ["pending", "No API token", "Without one, Cider is read through Now Playing instead."],
    "not-running": ["pending", "Not running", "Cider is closed."],
    "token-refused": ["unknown", "Token refused", "Cider did not accept the API token in Settings."],
    "paused": ["pending", "Paused", "Connected. Nothing is playing."],
    "playing": ["current", "In use", "Connected, and sharing what Cider plays."],
  };

  const DISCORD_STATES = {
    "starting": ["pending", "Starting", "Checking whether Discord is open."],
    "listening": ["current", "Standing in", "Discord is closed, so games and apps report here instead."],
    "discord-open": ["pending", "Discord is open", "Games report to Discord, and the site reads them from there."],
    "unavailable": ["unknown", "Can't listen", "Something else is listening where Discord would. See the Log."],
  };

  const els = {};
  for (const el of document.querySelectorAll("[id]")) {
    els[el.id] = el;
  }

  const root = document.documentElement;

  // On Windows, settings is a page of its own that takes the whole window,
  // as in Windows' Settings app, rather than a layer the page slides aside
  // from. It is left with a back arrow, so it has no Done button, and its
  // title is a trail - "Settings > Server" - as Settings' titles are.
  const SETTINGS_AS_PAGE = root.classList.contains("platform-windows");

  // On the site the settings are always dark. On Windows they are a page
  // like any other, so they follow the system's light or dark.
  if (SETTINGS_AS_PAGE) {
    els.settingsLayer.classList.remove("theme-dark");
  }

  let status = null;
  let config = null;
  let closingTimer = 0;

  /* ---------- Opening and closing settings ----------
   * openReveal() and closeReveal() in the site's general.js, cut down to
   * the one layer the app has. */

  function settingsOpen() {
    return root.classList.contains("revealOpen");
  }

  function openSettings() {
    if (settingsOpen()) return;

    clearTimeout(closingTimer);

    els.settingsLayer.classList.add("revealShown");
    root.classList.remove("revealClosing");
    root.classList.add("revealOpen");

    // Once the layer is showing: while it is hidden, it ignores being
    // scrolled back to the top, and would open where it was left.
    showSettingsPage("main");
    els.settingsLayer.focus();
  }

  // The layer stays until the page has slid back over it, and only then is
  // taken out - see .revealLayer in site.css.
  function closeSettings() {
    if (!settingsOpen()) return;

    // Whatever is typed in the field being left still counts.
    if (document.activeElement?.matches("[data-setting]")) {
      document.activeElement.blur();
    }

    els.settingsLayer.classList.remove("revealShown");
    root.classList.remove("revealOpen");
    root.classList.add("revealClosing");

    closingTimer = setTimeout(() => {
      root.classList.remove("revealClosing");
    }, REVEAL_MS);

    els.openSettings.focus();
  }

  // The settings' own pages, as settings.js on the site switches them: the
  // title bar takes the page's name and a back arrow, and Done is only on
  // the main page.
  function showSettingsPage(name) {
    els.settingsPanel.dataset.page = name;

    let title = "Settings";

    for (const page of els.settingsPanel.querySelectorAll(".settingsPage")) {
      const shown = page.dataset.page === name;
      page.hidden = !shown;

      if (shown && page.dataset.title) title = page.dataset.title;
    }

    showSettingsTitle(name, title);
    els.settingsBack.hidden = name === "main" && !SETTINGS_AS_PAGE;

    // The site's layer scrolls as a whole; the Windows page only below its
    // title. Both start at the top.
    els.settingsLayer.scrollTop = 0;
    els.settingsPanel.scrollTop = 0;

    if (name === "log") refreshLog();
    if (name === "sources") renderSources();
  }

  // Just the page's name, or on Windows the trail to it, where "Settings"
  // can be clicked to go back, as in Windows' Settings app.
  function showSettingsTitle(name, title) {
    if (!SETTINGS_AS_PAGE || name === "main") {
      els.settingsTitle.textContent = title;
      return;
    }

    const parent = document.createElement("button");
    parent.type = "button";
    parent.className = "settingsCrumb";
    parent.textContent = "Settings";
    parent.addEventListener("click", () => showSettingsPage("main"));

    const separator = document.createElement("i");
    separator.className = "fa-solid fa-chevron-right settingsCrumbSeparator";
    separator.setAttribute("aria-hidden", "true");

    const current = document.createElement("span");
    current.textContent = title;

    els.settingsTitle.replaceChildren(parent, separator, current);
  }

  // Back goes to Settings' main page, and from there - on Windows, where
  // it is also shown on the main page - out of settings.
  function goBack() {
    if (els.settingsPanel.dataset.page !== "main") {
      showSettingsPage("main");
    } else {
      closeSettings();
    }
  }

  // The button is on the page, and a click on the page closes settings - so
  // this click stops here, or it would close them again straight away.
  els.openSettings.addEventListener("click", (event) => {
    event.stopPropagation();
    openSettings();
  });

  els.done.addEventListener("click", closeSettings);
  els.settingsBack.addEventListener("click", goBack);

  for (const row of document.querySelectorAll("[data-subpage]")) {
    row.addEventListener("click", () => showSettingsPage(row.dataset.subpage));
  }

  // A click anywhere on the pushed-aside page brings it back, as on the site.
  els.page.addEventListener("click", () => {
    if (settingsOpen()) closeSettings();
  });

  document.addEventListener("keydown", (event) => {
    // Alt+Left is back in Windows' own apps, as in a browser.
    if (event.key === "Escape" || (event.key === "ArrowLeft" && event.altKey)) {
      goBack();
      return;
    }

    // Ctrl+, opens settings in most programs that have them.
    if (event.key === "," && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      openSettings();
    }
  });

  // The back button on the side of a mouse, which is button 3.
  document.addEventListener("mouseup", (event) => {
    if (event.button === 3 && settingsOpen()) {
      event.preventDefault();
      goBack();
    }
  });

  /* ---------- The page ---------- */

  function render() {
    if (!status) return;

    renderHeader();
    renderMusic();
    renderOther();
    renderNothing();

    if (els.settingsPanel.dataset.page === "sources") renderSources();

    els.configError.textContent = status.config_error;
    els.configError.classList.toggle("hide", !status.config_error);
  }

  function renderHeader() {
    let label = "Pause sharing";
    let icon = "fa-pause";
    if (status.paused) {
      label = "Resume sharing";
      icon = "fa-play";
    }

    els.pause.setAttribute("aria-label", label);
    els.pause.title = label;
    els.pause.firstElementChild.className = "fa-solid " + icon;

    let state = "good";
    let text = "Sharing to aridan.net as " + status.device;

    if (!status.set_up) {
      state = "warn";
      text = "Not set up yet. Add the server token in Settings.";
    } else if (status.paused) {
      state = "";
      text = "Paused. Nothing from this computer is on the site.";
    } else if (!status.server_ok) {
      state = "bad";
      text = "Can't reach the server: " + status.server_message;
    }

    els.statusDot.className = "statusDot " + state;
    els.statusDot.title = text;
    els.statusDot.setAttribute("aria-label", text);
    els.statusText.textContent = text;
  }

  // Filled in the way the site's lanyard.js fills in the same card.
  function renderMusic() {
    const music = musicOf(status);

    els.amLanyardDiscord.classList.toggle("hide", !music);
    if (!music) return;

    els.amActivityIcon.className = iconFor(music.name, MUSIC_ICON);
    els.amActivityName.textContent = music.name;
    els.amActivityDetails.textContent = music.details || "";
    els.amActivityState.textContent = music.state || "";

    const cover = coverUrl(music.assets?.large_image || "", 192);
    showImage(els.amActivityLogoLarge, els.amDiscordActivityImages, cover);

    els.amActivityLogoLarge.alt = music.assets?.large_text || "";
    els.amActivityLogoLarge.title = music.assets?.large_text || "";

    tickTimes();
  }

  // With nothing shared at all, the cards make way for a line saying so.
  function renderNothing() {
    const nothing = !musicOf(status) && !otherOf(status);

    els.discord.classList.toggle("hide", nothing);
    els.nothingPlaying.classList.toggle("hide", !nothing);
  }

  function musicOf(current) {
    const activities = current?.activities || [];
    return activities.find((a) => a.type === LISTENING) || null;
  }

  // Like the site, the first thing besides music gets the second card.
  function otherOf(current) {
    const activities = current?.activities || [];
    return activities.find((a) => a.type !== LISTENING) || null;
  }

  function renderOther() {
    const other = otherOf(status);

    els.discordActivity.classList.toggle("hide", !other);
    if (!other) return;

    els.activityIcon.className = iconFor(other.name, OTHER_ICON);
    els.activityName.textContent = other.name;
    els.activityDetails.textContent = other.details || "";
    els.activityState.textContent = other.state || "";

    const image = other.assets?.large_image || "";
    showImage(els.activityLogoLarge, els.discordActivityImages, image);

    tickTimes();
  }

  // The app's own icon if it has one in APP_ICONS, or the plain one.
  function iconFor(name, plain) {
    const lower = (name || "").toLowerCase();

    for (const [key, icon] of APP_ICONS) {
      if (lower.includes(key)) return icon;
    }

    return plain;
  }

  /* ---------- Pictures ----------
   * A card without a picture folds its picture's row away (.folded in
   * app.css). The row only opens once the picture has loaded, so it never
   * opens onto an empty space, and a picture that fails to load folds it
   * away again. */

  // Pictures that would not load, so they are not tried again every time
  // the status changes.
  const brokenImages = new Set();

  function showImage(img, row, url) {
    if (!url || brokenImages.has(url)) {
      img.removeAttribute("src");
      row.classList.add("folded");
      return;
    }

    // Already showing, or on its way. A new cover replaces the old one
    // without folding the row in between.
    if (img.getAttribute("src") === url) return;

    img.setAttribute("src", url);
  }

  function watchImage(img, row) {
    img.addEventListener("load", () => {
      row.classList.remove("folded");
    });

    img.addEventListener("error", () => {
      brokenImages.add(img.getAttribute("src"));
      img.removeAttribute("src");
      row.classList.add("folded");
    });
  }

  watchImage(els.amActivityLogoLarge, els.amDiscordActivityImages);
  watchImage(els.activityLogoLarge, els.discordActivityImages);

  // Apple's covers can be asked for at any size, by changing the size in
  // the address - see artwork() in the site's lanyardClient.js.
  function coverUrl(url, size) {
    const appleSize = /\/(\d+)x\1(bb|sr)(-\d+)?\.(jpg|png)(\?.*)?$/i;
    if (!url.includes("mzstatic.com/")) return url;
    return url.replace(appleSize, "/" + size + "x" + size + "$2$3.$4$5");
  }

  // Runs every second, so the bars and clocks move between status updates,
  // which only come when something changes.
  function tickTimes() {
    tickMusic();
    tickOther();
  }

  function tickMusic() {
    const music = musicOf(status);
    const start = music?.timestamps?.start;
    const end = music?.timestamps?.end;

    // Without both, there is no telling how far in the song is, so the bar
    // and the clock fold away rather than show a guess.
    const known = !!(start && end);
    els.amProgressTrack.classList.toggle("folded", !known);
    els.amTime.classList.toggle("folded", !known);

    if (!known) return;

    const length = end - start;
    const elapsed = Math.min(length, Math.max(0, Date.now() - start));

    els.amProgressBar.style.transform = "scaleX(" + (elapsed / length) + ")";
    els.amTime.textContent = clock(elapsed / 1000) + " / " + clock(length / 1000);
  }

  // The game card's clock, as updateActivityTime() in the site's lanyard.js:
  // counting down when the game says when it ends, up when it only says
  // when it started.
  function tickOther() {
    const timestamps = otherOf(status)?.timestamps;
    const now = Date.now();

    let seconds = null;
    if (timestamps?.end) {
      seconds = (timestamps.end - now) / 1000;
    } else if (timestamps?.start) {
      seconds = (now - timestamps.start) / 1000;
    }

    // No times at all: the clock and the line above it fold away, as the
    // picture does.
    const known = seconds !== null;
    els.Timeremaning.classList.toggle("folded", !known);
    els.activitySeparator.classList.toggle("folded", !known);

    if (known) els.ActivityTime.textContent = clock(seconds);

    const countingDown = !!timestamps?.end;
    els.Remaining.classList.toggle("hide", !countingDown || seconds === null);
    els.Elapsed.classList.toggle("hide", countingDown || seconds === null);

    const hasBar = !!(timestamps?.start && timestamps?.end);
    els.ProgressTrack.classList.toggle("folded", !hasBar);

    if (hasBar) {
      const length = timestamps.end - timestamps.start;
      const fraction = Math.min(1, Math.max(0, (now - timestamps.start) / length));
      els.ProgressBar.style.transform = "scaleX(" + fraction + ")";
    }
  }

  // 02:11, or 1:02:11 past an hour - clock() in the site's lanyard.js.
  function clock(seconds) {
    const total = Math.max(0, Math.floor(seconds));

    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    const secs = total % 60;

    const parts = [];
    if (hours > 0) parts.push(hours);
    parts.push(minutes);
    parts.push(secs);

    const padded = parts.map((n) => String(n).padStart(2, "0"));
    return padded.join(":");
  }

  els.pause.addEventListener("click", () => {
    invoke("set_paused", { paused: !status.paused });
  });

  /* ---------- Sources ----------
   * One entry each, built the way the site's versionCheck.js builds its
   * Component versions list. */

  function renderSources() {
    if (!status) return;

    els.sourceList.replaceChildren();

    const cider = CIDER_STATES[status.cider] || ["pending", status.cider, ""];
    els.sourceList.appendChild(sourceItem("Cider", cider[0], cider[1], cider[2]));

    const discord = DISCORD_STATES[status.discord] || ["pending", status.discord, ""];
    els.sourceList.appendChild(sourceItem("Discord rich presence", discord[0], discord[1], discord[2]));

    const players = status.players || [];
    const playing = players.find((p) => p.allowed && p.playing);

    let state = "pending";
    let badge = "Nothing playing";
    if (playing) {
      state = "current";
      badge = playing.name;
    }

    const lines = [];
    for (const player of players) {
      let line = player.name;
      if (player.playing) line += ", playing";
      if (!player.allowed) line += " - never shared";
      lines.push(line);
    }
    if (!lines.length) lines.push("No players open.");

    els.sourceList.appendChild(sourceItem("Now Playing", state, badge, lines.join("\n")));
  }

  function sourceItem(name, state, badge, detail) {
    const item = document.createElement("li");
    item.className = "versionItem";
    item.dataset.status = state;

    const head = document.createElement("div");
    head.className = "versionHead";

    const nameEl = document.createElement("span");
    nameEl.className = "versionName";
    nameEl.textContent = name;
    head.appendChild(nameEl);

    const badgeEl = document.createElement("span");
    badgeEl.className = "versionBadge";
    badgeEl.textContent = badge;
    head.appendChild(badgeEl);

    item.appendChild(head);

    for (const line of detail.split("\n")) {
      if (!line) continue;

      const p = document.createElement("p");
      p.className = "versionDetail";
      p.textContent = line;
      item.appendChild(p);
    }

    return item;
  }

  /* ---------- Settings ---------- */

  function fillSettings() {
    els.token.value = config.token;
    els.server.value = config.server;
    els.device.value = config.device;
    els.device.placeholder = status?.device || "";
    els.ciderToken.value = config.cider_token;
    els.countries.value = config.itunes_countries.join(", ");

    renderAllowed();
  }

  // Every known app, plus anything already allowed or currently open, so a
  // new player can be switched on as soon as it shows up in Now Playing.
  function renderAllowed() {
    const names = new Map(KNOWN_PLAYERS);

    for (const key of config.allowed_players) {
      if (!names.has(key)) names.set(key, key);
    }

    for (const player of status?.players || []) {
      if (!names.has(player.key)) names.set(player.key, player.name);
    }

    els.allowed.replaceChildren();

    for (const [key, name] of names) {
      els.allowed.appendChild(toggleRow(key, name, config.allowed_players.includes(key)));
    }
  }

  // toggleMarkup() in the site's settingsPanel.js, built element by element.
  function toggleRow(key, name, checked) {
    const id = "allow-" + key;

    const row = document.createElement("div");
    row.className = "checkbox-wrapper-51";

    const title = document.createElement("label");
    title.className = "title";
    title.htmlFor = id;
    title.textContent = name;
    row.appendChild(title);

    const box = document.createElement("input");
    box.type = "checkbox";
    box.id = id;
    box.value = key;
    box.checked = checked;
    box.addEventListener("change", save);
    row.appendChild(box);

    // The switch itself is a copy of the one in the markup, so its drawing
    // lives in one place.
    const toggle = document.querySelector('label.toggle[for="autostart"]').cloneNode(true);
    toggle.htmlFor = id;
    row.appendChild(toggle);

    return row;
  }

  // Saved as soon as something changes, the way the site's settings apply
  // straight away. Text fields count as changed once they are left or Enter
  // is pressed, not on every key, so a half-typed token is never saved.
  async function save() {
    // Kept in the order they are listed, which is also the order they win in
    // when two are playing at once.
    const allowed = [];
    for (const box of els.allowed.querySelectorAll("input")) {
      if (box.checked) allowed.push(box.value);
    }

    const countries = els.countries.value.split(",").map((c) => c.trim()).filter(Boolean);

    const wanted = {
      server: els.server.value,
      token: els.token.value,
      device: els.device.value,
      cider_token: els.ciderToken.value,
      itunes_countries: countries,
      allowed_players: allowed,
    };

    try {
      config = await invoke("save_config", { config: wanted });
      fillSettings();
      showSaveState("Saved", false);
    } catch (err) {
      showSaveState(String(err), true);
    }
  }

  let saveStateTimer = 0;

  function showSaveState(text, bad) {
    clearTimeout(saveStateTimer);

    els.saveState.textContent = text;
    els.saveState.classList.toggle("bad", bad);

    // A problem stays until it is fixed; "Saved" fades after a moment.
    if (!bad) {
      saveStateTimer = setTimeout(() => {
        els.saveState.textContent = "";
      }, 2000);
    }
  }

  for (const input of document.querySelectorAll("[data-setting]")) {
    input.addEventListener("change", save);

    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") input.blur();
    });
  }

  els.autostart.addEventListener("change", async () => {
    const wanted = els.autostart.checked;

    try {
      await invoke("set_autostart", { enabled: wanted });
      showSaveState("Saved", false);
    } catch (err) {
      els.autostart.checked = !wanted;
      showSaveState(String(err), true);
    }
  });

  /* ---------- Log ---------- */

  async function refreshLog() {
    const lines = await invoke("get_log");

    els.logLines.replaceChildren();

    // Newest first.
    for (const line of lines.slice().reverse()) {
      const item = document.createElement("li");
      item.className = "versionItem";

      const time = document.createElement("span");
      time.className = "logTime";
      time.textContent = new Date(line.at).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hourCycle: "h23",
      });
      item.appendChild(time);

      const text = document.createElement("p");
      text.className = "versionDetail";
      text.textContent = line.text;
      item.appendChild(text);

      els.logLines.appendChild(item);
    }
  }

  /* ---------- Start ---------- */

  async function start() {
    status = await invoke("get_status");
    config = await invoke("get_config");
    els.configPath.textContent = await invoke("config_path");
    els.autostart.checked = await invoke("get_autostart");
    els.version.textContent = await getVersion();

    render();
    fillSettings();

    // First run: nothing works until there is a token, and that is set in
    // Settings, so start there.
    if (!status.set_up) openSettings();

    await listen("status", (event) => {
      const playersBefore = JSON.stringify(status?.players?.map((p) => p.key));

      status = event.payload;
      render();

      // A new player showed up: it gets a switch in Settings, but without
      // touching the fields, which might be half-way through being edited.
      const playersNow = JSON.stringify(status.players.map((p) => p.key));
      if (playersNow !== playersBefore) renderAllowed();

      if (els.settingsPanel.dataset.page === "log") refreshLog();
    });

    setInterval(tickTimes, 1000);
  }

  start();
})();
