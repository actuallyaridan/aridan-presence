/* Which system the window is on, from window.__PRESENCE_LOOK__, which the
 * Rust side (look.rs) leaves before the page starts. This runs in <head>,
 * before anything is drawn, so the page's first look is already the right
 * one.
 *
 * On <html> it puts:
 *   platform-windows (or -macos, -linux)   which system's look to use
 *   material                               the window has the system's
 *                                          material behind it, so the page
 *                                          leaves its background empty
 * and the system's accent colour as --system-accent-light/-dark.
 */
(() => {
  "use strict";

  const look = window.__PRESENCE_LOOK__;

  // Opened some other way than through the app, as in a browser.
  if (!look) return;

  const root = document.documentElement;

  root.classList.add("platform-" + look.platform);

  if (look.material) {
    root.classList.add("material");
  }

  if (look.accent) {
    root.style.setProperty("--system-accent-light", look.accent.light);
    root.style.setProperty("--system-accent-dark", look.accent.dark);
  }

  // Windows draws its own controls in its own way, not with the site's
  // glass, so the glass is taken off rather than drawn over.
  if (look.platform === "windows") {
    root.classList.remove("style-liquid-glass");
  }
})();
