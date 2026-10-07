// Opening the window, and quitting.
//
// The window is not kept around while closed: closing it throws it away, and
// opening it makes a new one. A hidden window still holds a whole web page in
// memory, which is a lot for something that sits in the tray all day.

use crate::config::Config;
use crate::look;
use crate::server::Server;
use crate::state::Shared;
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

// Takes this computer off the site, then exits. Quitting is the one time the
// app goes away on purpose, so it is worth the moment it takes to say so.
pub fn quit(app: &AppHandle) {
    let app = app.clone();
    let shared = app.state::<Arc<Shared>>().inner().clone();
    let config: Config = shared.config();

    tauri::async_runtime::spawn(async move {
        if config.is_set_up() {
            shared.log("Quitting. Clearing this computer from the site.");
            let _ = Server::new(&config).clear().await;
        }

        app.exit(0);
    });
}
