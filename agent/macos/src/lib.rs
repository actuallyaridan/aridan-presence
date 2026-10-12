// The bridge between the Swift app (Sources/) and the Rust side that does
// the work (../core). The same handful of things the Qt app's ffi.rs offers,
// as plain C functions, since that is what Swift can call directly. Their
// declarations for Swift are in Sources/CPresence/presence.h.
//
// Status, settings and the log cross as JSON text, which Swift reads with
// JSONDecoder. Every string handed to Swift is Rust's to free, so Swift gives
// each one back with presence_free_string once it has copied it.
//
// The engine runs on tokio's threads; Swift's window runs on the main thread.
// The callbacks below are called from tokio's threads, and Swift moves them
// over to the main thread itself.

use presence_core::config::{self, Config};
use presence_core::engine;
use presence_core::state::{OnStatus, Shared, Status};
use std::ffi::{c_char, CStr, CString};
use std::sync::{Arc, OnceLock};

// What Swift's callbacks look like: a JSON string, which Swift must free.
type StatusCallback = extern "C" fn(*mut c_char);
type DoneCallback = extern "C" fn();

struct Context {
    shared: Arc<Shared>,
    runtime: tokio::runtime::Runtime,
}

static CONTEXT: OnceLock<Context> = OnceLock::new();

fn context() -> &'static Context {
    CONTEXT.get().expect("presence_start is called before anything else")
}

// Starts the engine in the background. `on_status` is called with every new
// status, from then on until the app quits.
#[unsafe(no_mangle)]
pub extern "C" fn presence_start(on_status: StatusCallback) {
    if CONTEXT.get().is_some() {
        return;
    }

    // A broken settings file does not stop the app: it starts with the
    // defaults, and Settings says what was wrong.
    let (config, config_error) = match config::load() {
        Ok(config) => (config, String::new()),
        Err(message) => {
            eprintln!("{}", message);
            (Config::default(), message)
        }
    };

    let shared = Arc::new(Shared::new(config, config_error));

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio should always start");

    let callback: OnStatus = Arc::new(move |status: &Status| {
        on_status(to_c(to_json(status)));
    });

    runtime.spawn(engine::run(shared.clone(), callback));

    let _ = CONTEXT.set(Context {
        shared: shared,
        runtime: runtime,
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn presence_status_json() -> *mut c_char {
    to_c(to_json(&context().shared.status()))
}

#[unsafe(no_mangle)]
pub extern "C" fn presence_config_json() -> *mut c_char {
    to_c(to_json(&context().shared.config()))
}

#[unsafe(no_mangle)]
pub extern "C" fn presence_log_json() -> *mut c_char {
    to_c(to_json(&context().shared.log_lines()))
}

#[unsafe(no_mangle)]
pub extern "C" fn presence_config_path() -> *mut c_char {
    to_c(config::path().display().to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn presence_version() -> *mut c_char {
    to_c(String::from(env!("CARGO_PKG_VERSION")))
}

// Saves the settings. Gives back an empty string if that worked, or why not.
// The new status follows through on_status as usual.
#[unsafe(no_mangle)]
pub extern "C" fn presence_save_config(json: *const c_char) -> *mut c_char {
    let text = from_c(json);

    let wanted: Config = match serde_json::from_str(&text) {
        Ok(config) => config,
        Err(error) => return to_c(format!("Could not read the settings: {}", error)),
    };

    match context().shared.save_config(wanted) {
        Ok(_) => to_c(String::new()),
        Err(message) => to_c(message),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn presence_set_paused(paused: bool) {
    context().shared.set_paused(paused);
}

// Quitting: takes this computer off the site, then calls `done`, after which
// Swift lets the app end.
#[unsafe(no_mangle)]
pub extern "C" fn presence_quit(done: DoneCallback) {
    let Some(context) = CONTEXT.get() else {
        done();
        return;
    };

    let shared = context.shared.clone();

    context.runtime.spawn(async move {
        shared.clear_from_site().await;
        done();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn presence_free_string(text: *mut c_char) {
    if text.is_null() {
        return;
    }

    // Safety: every string Swift is given was made by to_c, and Swift hands
    // each back exactly once.
    unsafe {
        drop(CString::from_raw(text));
    }
}

fn to_json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| String::from("null"))
}

// A NUL in the middle would cut the string short in C, so any are dropped.
fn to_c(text: String) -> *mut c_char {
    let text = text.replace('\0', "");
    CString::new(text).unwrap_or_default().into_raw()
}

fn from_c(text: *const c_char) -> String {
    if text.is_null() {
        return String::new();
    }

    // Safety: Swift passes a NUL-terminated string it keeps alive for the
    // length of the call.
    unsafe { CStr::from_ptr(text).to_string_lossy().into_owned() }
}
