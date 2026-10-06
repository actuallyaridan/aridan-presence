// What the window can ask the Rust side to do. In ui/app.js these are called
// with invoke("get_status"), invoke("save_config", { config }) and so on.
//
// `#[tauri::command]` is what makes a function callable from the window.
// `State<...>` is Tauri handing over the shared state that main.rs gave it.

use crate::config::{self, Config};
use crate::state::{LogLine, Shared, Status};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_status(shared: State<'_, Arc<Shared>>) -> Status {
    shared.status()
}

#[tauri::command]
pub fn get_log(shared: State<'_, Arc<Shared>>) -> Vec<LogLine> {
    shared.log_lines()
}

#[tauri::command]
pub fn get_config(shared: State<'_, Arc<Shared>>) -> Config {
    shared.config()
}

#[tauri::command]
pub fn config_path() -> String {
    config::path().display().to_string()
}

// Writes the file first, and only hands the new settings to the engine once
// that worked - so the app never runs with settings it could not save.
#[tauri::command]
pub fn save_config(app: AppHandle, shared: State<'_, Arc<Shared>>, config: Config) -> Result<Config, String> {
    let saved = config::save(config)?;

    shared.replace_config(saved.clone());
    shared.log("Settings saved.");

    // Whatever was wrong with the old file is gone now that it has been
    // written fresh.
    let status = {
        let mut status = shared.status.lock().unwrap();
        status.config_error.clear();
        status.set_up = saved.is_set_up();
        status.device = saved.device_name();
        status.clone()
    };
    let _ = app.emit("status", &status);

    Ok(saved)
}

#[tauri::command]
pub fn set_paused(shared: State<'_, Arc<Shared>>, paused: bool) {
    shared.set_paused(paused);
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();

    let result = if enabled { launcher.enable() } else { launcher.disable() };

    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(format!("Could not change start at login: {}", error)),
    }
}
