// aridan-presence: tells presence.aridan.net what I am listening to and
// playing, so the music widget on aridan.net works without Discord open.
//
// It is a tray app. The work happens in engine.rs, in the background, for as
// long as the app is open. The window (ui/) is only for looking at what it is
// doing and changing its settings; closing it leaves the app running in the
// tray. Quit from the tray menu to stop it.

// On Windows, a program is either a console program or a windowed one. This
// makes the finished build a windowed one, so no black console window opens
// alongside it. Test builds keep the console, for their printed output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Each `mod` line pulls in the file of that name.
mod activity;
mod artwork;
mod commands;
mod config;
mod engine;
mod server;
mod sources;
mod state;
mod track;
mod tray;
mod window;

use state::Shared;
use std::sync::Arc;
use tauri::{Manager, RunEvent};
use tauri_plugin_autostart::MacosLauncher;

// Starting at login passes this, so the app goes straight to the tray instead
// of opening its window every morning.
const MINIMIZED: &str = "--minimized";

fn main() {
    // A broken settings file does not stop the app: it starts with the
    // defaults, and the Settings tab says what was wrong.
    let (config, config_error) = match config::load() {
        Ok(config) => (config, String::new()),
        Err(message) => {
            eprintln!("{}", message);
            (config::Config::default(), message)
        }
    };

    let set_up = config.is_set_up();
    let shared = Arc::new(Shared::new(config, config_error));

    // Not set up yet means the window opens anyway, since Settings is where
    // that gets done.
    let started_minimized = std::env::args().any(|argument| argument == MINIMIZED);
    let open_window = !started_minimized || !set_up;

    let engine_shared = shared.clone();

    let app = tauri::Builder::default()
        // Starting it a second time - from the app menu, say - opens the
        // window of the one already running instead. Two copies would fight
        // over discord-ipc-0 and report over each other.
        .plugin(tauri_plugin_single_instance::init(|app, _arguments, _folder| {
            window::show(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED]),
        ))
        .manage(shared)
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_log,
            commands::get_config,
            commands::config_path,
            commands::save_config,
            commands::set_paused,
            commands::get_autostart,
            commands::set_autostart,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            let tray = tray::build(&handle)?;
            app.manage(tray);

            if open_window {
                window::show(&handle);
            }

            tauri::async_runtime::spawn(engine::run(handle.clone(), engine_shared));
            tauri::async_runtime::spawn(quit_on_signal(handle));

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("the app should always start");

    app.run(|_app, event| {
        // Closing the last window would normally end the program. Here it
        // only closes the window, and the app carries on in the tray.
        // `code` is only set when something actually asked to exit, like
        // Quit in the tray menu.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}

// Logging out or shutting down on Linux and macOS sends a polite "please
// stop" first. Answering it like Quit takes this computer off the site
// straight away, instead of 90 seconds later.
async fn quit_on_signal(app: tauri::AppHandle) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let Ok(mut terminate) = signal(SignalKind::terminate()) else {
            return;
        };

        terminate.recv().await;
        window::quit(&app);
    }

    #[cfg(not(unix))]
    {
        let _ = app;
    }
}
