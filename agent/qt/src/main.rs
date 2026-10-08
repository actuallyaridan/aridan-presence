// aridan-presence on Linux: the same app as on Windows and macOS, with its
// window and tray made with Qt, so the desktop's theme draws them.
//
// What happens where:
//   ../core      the work - reading the players, reporting to the site
//   src/         starting up, and the bridge to the window (ffi.rs)
//   ui/          the window, the settings and the tray icon, in C++ with
//                Qt Widgets, like linux-taskmgr and linux-devmgmt
//
// The engine runs on tokio's threads in the background; Qt runs on this
// one, the main thread, until Quit.

mod autostart;
mod ffi;
mod instance;

use instance::Instance;
use presence_core::config;
use presence_core::engine;
use presence_core::state::{OnStatus, Shared, Status};
use std::sync::{Arc, OnceLock};

// Starting at login passes this, so the app goes straight to the tray.
const MINIMIZED: &str = "--minimized";

// What the bridge needs to reach from Qt's side. Set once, at startup.
pub struct Context {
    pub shared: Arc<Shared>,
    pub runtime: tokio::runtime::Handle,
    pub start_minimized: bool,
}

static CONTEXT: OnceLock<Context> = OnceLock::new();

pub fn context() -> &'static Context {
    CONTEXT.get().expect("the context is set before Qt starts")
}

fn main() {
    // A broken settings file does not stop the app: it starts with the
    // defaults, and Settings says what was wrong.
    let (config, config_error) = match config::load() {
        Ok(config) => (config, String::new()),
        Err(message) => {
            eprintln!("{}", message);
            (config::Config::default(), message)
        }
    };

    let shared = Arc::new(Shared::new(config, config_error));

    // tokio's threads, for the engine and everything else that waits on the
    // network or D-Bus.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio should always start");

    // Kept until the end of main, so the D-Bus name stays this copy's.
    let instance = runtime.block_on(instance::claim());
    if let Instance::AlreadyRunning = instance {
        println!("Already running. Showed its window instead.");
        return;
    }

    let start_minimized = std::env::args().any(|argument| argument == MINIMIZED);

    let context = Context {
        shared: shared.clone(),
        runtime: runtime.handle().clone(),
        start_minimized: start_minimized,
    };
    let _ = CONTEXT.set(context);

    let on_status: OnStatus = Arc::new(|status: &Status| {
        ffi::post_status(ffi::to_json(status));
    });

    runtime.spawn(engine::run(shared, on_status));
    runtime.spawn(quit_on_signal());

    let code = ffi::run_application();

    drop(instance);
    std::process::exit(code);
}

// Quit from the tray, or the system asking: takes this computer off the
// site first, then lets Qt stop.
pub fn quit() {
    let context = context();
    let shared = context.shared.clone();

    context.runtime.spawn(async move {
        shared.clear_from_site().await;
        ffi::post_quit();
    });
}

// Logging out or shutting down sends a polite "please stop" (SIGTERM), and
// Ctrl+C in a terminal sends SIGINT. Both are answered like Quit.
async fn quit_on_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let Ok(mut terminate) = signal(SignalKind::terminate()) else {
        return;
    };
    let Ok(mut interrupt) = signal(SignalKind::interrupt()) else {
        return;
    };

    tokio::select! {
        _ = terminate.recv() => {}
        _ = interrupt.recv() => {}
    }

    quit();
}
