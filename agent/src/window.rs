// Opening the window, and quitting.
//
// The window is not kept around while closed: closing it throws it away, and
// opening it makes a new one. A hidden window still holds a whole web page in
// memory, which is a lot for something that sits in the tray all day.

use crate::look;
use presence_core::state::Shared;
use std::sync::Arc;
use tauri::{AppHandle, Manager, WebviewWindowBuilder};

pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    // Made from the "main" entry in tauri.conf.json, which says how big it is
    // and so on. That entry has "create": false, so Tauri does not make one by
    // itself at startup.
    let Some(settings) = app.config().app.windows.first() else {
        return;
    };

    match WebviewWindowBuilder::from_config(app, settings) {
        Ok(builder) => {
            let builder = look::dress(builder);

            if let Err(error) = builder.build() {
                eprintln!("Could not open the window: {}", error);
            }
        }
        Err(error) => eprintln!("Could not open the window: {}", error),
    }
}

// Takes this computer off the site, then exits.
pub fn quit(app: &AppHandle) {
    let app = app.clone();
    let shared = app.state::<Arc<Shared>>().inner().clone();

    tauri::async_runtime::spawn(async move {
        shared.clear_from_site().await;
        app.exit(0);
    });
}
