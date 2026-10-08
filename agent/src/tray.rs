// The icon in the system tray - KDE's panel, the corner by the Windows
// clock, the macOS menu bar - and its menu:
//
//   Popila - Severina          what is being shared (not clickable)
//   ─────────────
//   ✓ Pause sharing
//   Open aridan-presence
//   ─────────────
//   Quit
//
// Its icon turns grey while sharing is paused or nothing is set up yet.

use presence_core::state::{Shared, Status};
use crate::window;
use std::sync::Arc;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

// Baked into the program, so there are no image files to install next to it.
const ICON_ON: &[u8] = include_bytes!("../icons/tray.png");
const ICON_OFF: &[u8] = include_bytes!("../icons/tray-paused.png");

// Kept so the menu and icon can be changed after they are made.
// `Wry` is the name of the window library Tauri uses underneath.
pub struct Tray {
    icon: TrayIcon<Wry>,
    now: MenuItem<Wry>,
    pause: CheckMenuItem<Wry>,
}

pub fn build(app: &AppHandle) -> tauri::Result<Tray> {
    let now = MenuItem::with_id(app, "now", "Nothing playing", false, None::<&str>)?;
    let pause = CheckMenuItem::with_id(app, "pause", "Pause sharing", true, false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open aridan-presence", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let first_line = PredefinedMenuItem::separator(app)?;
    let last_line = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(app, &[&now, &first_line, &pause, &open, &last_line, &quit])?;

    let icon = TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(ICON_OFF)?)
        .tooltip("aridan-presence")
        .menu(&menu)
        // Left click opens the window instead of the menu, on Windows and
        // macOS. Linux trays always open the menu.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            on_menu(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::show(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(Tray {
        icon: icon,
        now: now,
        pause: pause,
    })
}

fn on_menu(app: &AppHandle, id: &str) {
    let shared = app.state::<Arc<Shared>>();

    if id == "pause" {
        // The tick has already flipped by the time this runs, so the menu's
        // own idea of it is the one to follow.
        let paused = !shared.is_paused();
        shared.set_paused(paused);
    } else if id == "open" {
        window::show(app);
    } else if id == "quit" {
        window::quit(app);
    }
}

// Called by the engine whenever the status changes.
pub fn update(app: &AppHandle, status: &Status) {
    let Some(tray) = app.try_state::<Tray>() else {
        return;
    };

    let line = now_line(status);
    let _ = tray.now.set_text(&line);
    let _ = tray.icon.set_tooltip(Some(format!("aridan-presence\n{}", line)));
    let _ = tray.pause.set_checked(status.paused);

    let sharing = status.set_up && !status.paused;

    let bytes = if sharing { ICON_ON } else { ICON_OFF };
    if let Ok(image) = Image::from_bytes(bytes) {
        let _ = tray.icon.set_icon(Some(image));
    }
}

fn now_line(status: &Status) -> String {
    if !status.set_up {
        return String::from("Not set up yet");
    }

    let Some(first) = status.activities.first() else {
        if status.paused {
            return String::from("Paused");
        }
        return String::from("Nothing playing");
    };

    let mut line = first.details.clone().unwrap_or_else(|| first.name.clone());

    if let Some(state) = &first.state {
        line = format!("{} - {}", line, state);
    }

    if status.paused {
        line = format!("Paused: {}", line);
    }

    line
}
