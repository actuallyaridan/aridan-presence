// Starting at login, the freedesktop way: a .desktop file in
// ~/.config/autostart, which KDE, GNOME and the rest all read. It starts
// the app with --minimized, so it goes straight to the tray.
//
// The same file the Tauri version made, so switching between them keeps
// the setting.

use std::fs;
use std::path::PathBuf;

fn path() -> Option<PathBuf> {
    let folder = dirs::config_dir()?.join("autostart");
    Some(folder.join("aridan-presence.desktop"))
}

pub fn is_enabled() -> bool {
    match path() {
        Some(path) => path.exists(),
        None => false,
    }
}

pub fn set(enabled: bool) -> Result<(), String> {
    let Some(path) = path() else {
        return Err(String::from("Could not find the settings folder."));
    };

    if !enabled {
        if path.exists() {
            fs::remove_file(&path).map_err(|error| format!("Could not turn off start at login: {}", error))?;
        }
        return Ok(());
    }

    // Wherever this copy of the app is, so it is the one that starts.
    let program = std::env::current_exe().map_err(|error| format!("Could not find the app itself: {}", error))?;

    // Quoted, in case the path has spaces in it.
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=aridan-presence\n\
         Comment=Shares what I am listening to with aridan.net\n\
         Exec=\"{}\" --minimized\n\
         Icon=aridan-presence\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        program.display()
    );

    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(|error| format!("Could not turn on start at login: {}", error))?;
    }

    fs::write(&path, entry).map_err(|error| format!("Could not turn on start at login: {}", error))
}
