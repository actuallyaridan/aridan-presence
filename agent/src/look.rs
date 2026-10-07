// How the window should look on this computer.
//
// The page is the same everywhere, but each system has its own way of
// drawing a window, so the page is told which system it is on and dresses
// itself to match - see ui/platform.js and ui/windows.css. It is told before
// anything is drawn, so it never shows one look and then jumps to another.
//
// The window itself also gets the system's material behind it: Mica on
// Windows 11, the softly tinted background Windows' own apps have.

use serde::Serialize;
use tauri::{Manager, Runtime, WebviewWindowBuilder};

#[derive(Serialize)]
struct Look {
    // "windows", "macos" or "linux".
    platform: &'static str,

    // Whether the window has the system's material behind it, in which case
    // the page leaves its background empty to let it through.
    material: bool,

    // The accent colour picked in Windows' settings, in the two shades
    // Windows' own switches use: a darker one on a light background and a
    // lighter one on a dark background. None anywhere else.
    accent: Option<Accent>,
}

#[derive(Serialize)]
struct Accent {
    light: String,
    dark: String,
}

// Gives a window being built its look: the material behind it, and a line of
// JavaScript that runs before the page does and leaves the look where the
// page can find it, as window.__PRESENCE_LOOK__.
pub fn dress<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    let look = Look {
        platform: std::env::consts::OS,
        material: has_material(),
        accent: accent(),
    };

    let json = serde_json::to_string(&look).unwrap_or_else(|_| String::from("null"));
    let script = format!("window.__PRESENCE_LOOK__ = {};", json);

    let mut builder = builder.initialization_script(script);

    if look.material {
        builder = with_material(builder);
    }

    builder
}

// Mica came with Windows 11, which is Windows build 22000 and up - Windows
// 11 still calls itself Windows 10 everywhere else, so the build number is
// the way to tell them apart.
#[cfg(windows)]
fn has_material() -> bool {
    let build = sysinfo::System::kernel_version().unwrap_or_default();

    match build.trim().parse::<u32>() {
        Ok(number) => number >= 22000,
        Err(_) => false,
    }
}

#[cfg(not(windows))]
fn has_material() -> bool {
    false
}

// Mica only shows through where nothing is drawn on top of it, so the
// window is see-through. It is only ever asked for where there is Mica:
// anywhere else, a see-through window would show the desktop behind it.
#[cfg(windows)]
fn with_material<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    use tauri::utils::config::WindowEffectsConfig;
    use tauri::window::Effect;

    let effects = WindowEffectsConfig {
        effects: vec![Effect::Mica],
        ..Default::default()
    };

    builder.transparent(true).effects(effects)
}

#[cfg(not(windows))]
fn with_material<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    builder
}

#[cfg(windows)]
fn accent() -> Option<Accent> {
    use ::windows::UI::ViewManagement::{UIColorType, UISettings};

    let settings = UISettings::new().ok()?;

    // "AccentDark1" and "AccentLight2" are the shades Windows calls
    // AccentFillColorDefault in its light and dark themes.
    let dark1 = settings.GetColorValue(UIColorType::AccentDark1).ok()?;
    let light2 = settings.GetColorValue(UIColorType::AccentLight2).ok()?;

    let accent = Accent {
        light: format!("#{:02x}{:02x}{:02x}", dark1.R, dark1.G, dark1.B),
        dark: format!("#{:02x}{:02x}{:02x}", light2.R, light2.G, light2.B),
    };

    Some(accent)
}

#[cfg(not(windows))]
fn accent() -> Option<Accent> {
    None
}
