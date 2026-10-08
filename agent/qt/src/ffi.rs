// The bridge between the Rust side and the C++ window (ui/).
//
// Two directions:
//   extern "Rust"   what the window can ask for: the status, the settings,
//                   saving them, pausing...
//   extern "C++"    what Rust can tell the window: a new status, "show
//                   yourself", "you can quit now" - and starting it at all
//
// Status and settings cross as JSON text, which the window reads with
// QJsonDocument. That keeps the bridge to a handful of strings instead of a
// C++ type for every field of every struct.

use crate::{autostart, context};
use presence_core::config::{self, Config};

#[cxx_qt::bridge]
mod bridge {
    extern "Rust" {
        fn status_json() -> String;
        fn config_json() -> String;

        // Gives back nothing if that worked, or why not.
        fn save_config(json: &str) -> String;

        fn set_paused(paused: bool);
        fn log_json() -> String;
        fn config_path() -> String;
        fn app_version() -> String;

        fn autostart_enabled() -> bool;

        // Gives back nothing if that worked, or why not.
        fn set_autostart(enabled: bool) -> String;

        // True when started at login, which goes straight to the tray.
        fn start_minimized() -> bool;

        // Takes this computer off the site, then tells the window it can
        // quit (post_quit).
        fn request_quit();
    }

    unsafe extern "C++" {
        include!("aridan-presence-qt/ui/Bridge.h");

        // Starts Qt and the window, and runs until Quit. Gives back the exit
        // code.
        fn run_application() -> i32;

        // These three can be called from any thread. The window picks them
        // up on its own.
        fn post_status(json: String);
        fn post_show();
        fn post_quit();
    }
}

pub use bridge::{post_quit, post_show, post_status, run_application};

fn status_json() -> String {
    to_json(&context().shared.status())
}

fn config_json() -> String {
    to_json(&context().shared.config())
}

fn save_config(json: &str) -> String {
    let wanted: Config = match serde_json::from_str(json) {
        Ok(config) => config,
        Err(error) => return format!("Could not read the settings: {}", error),
    };

    match context().shared.save_config(wanted) {
        Ok((_saved, status)) => {
            post_status(to_json(&status));
            String::new()
        }
        Err(message) => message,
    }
}

fn set_paused(paused: bool) {
    context().shared.set_paused(paused);
}

fn log_json() -> String {
    to_json(&context().shared.log_lines())
}

fn config_path() -> String {
    config::path().display().to_string()
}

fn app_version() -> String {
    String::from(env!("CARGO_PKG_VERSION"))
}

fn autostart_enabled() -> bool {
    autostart::is_enabled()
}

fn set_autostart(enabled: bool) -> String {
    match autostart::set(enabled) {
        Ok(()) => String::new(),
        Err(message) => message,
    }
}

fn start_minimized() -> bool {
    context().start_minimized
}

fn request_quit() {
    crate::quit();
}

pub fn to_json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| String::from("null"))
}
