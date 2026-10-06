// What the engine knows, kept where the window and the tray can read it.
//
// The engine (engine.rs) is the only thing that changes Status. The window
// asks for it once when it opens, and is sent a fresh copy - the "status"
// event - every time it changes after that.

use crate::activity::{now_ms, Activity};
use crate::config::Config;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tokio::sync::Notify;

// How many lines the Log tab keeps.
const LOG_LINES: usize = 60;

#[derive(Serialize, Clone, Default, PartialEq)]
pub struct Status {
    // False until there is a token, and the window opens on Settings.
    pub set_up: bool,
    pub paused: bool,
    pub device: String,

    // Whether the last report reached the Worker, and if not, why.
    pub server_ok: bool,
    pub server_message: String,

    // One word each, which the window turns into a sentence:
    //   cider:   "no-token", "not-running", "token-refused", "paused", "playing"
    //   discord: "starting", "listening", "discord-open", "unavailable"
    pub cider: String,
    pub discord: String,

    // Every player Now Playing can see, allowed or not, so the window can
    // show why something is or is not being shared.
    pub players: Vec<PlayerSeen>,

    // What is being shared - or, while paused, what would be.
    pub activities: Vec<Activity>,

    // A problem with the settings file, shown on the Settings tab.
    pub config_error: String,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct PlayerSeen {
    pub key: String,
    pub name: String,
    pub allowed: bool,
    pub playing: bool,
}

#[derive(Serialize, Clone)]
pub struct LogLine {
    pub at: u64,
    pub text: String,
}

// Everything the engine, the window and the tray share. One of these is made
// at startup and handed to all of them inside an Arc, a pointer several
// owners can hold at once.
pub struct Shared {
    pub status: Mutex<Status>,
    pub config: Mutex<Config>,
    pub log: Mutex<Vec<LogLine>>,

    // Set from the tray or the window. The engine reads it every tick.
    pub paused: AtomicBool,

    // Goes up by one every time Settings is saved, so the engine can tell
    // that the settings it started with are out of date.
    pub config_version: AtomicU64,

    // Wakes the engine early, so a change shows straight away instead of at
    // the next tick.
    pub wake: Notify,
}

impl Shared {
    pub fn new(config: Config, config_error: String) -> Shared {
        let mut status = Status::default();
        status.config_error = config_error;
        status.set_up = config.is_set_up();
        status.device = config.device_name();
        status.cider = String::from("no-token");
        status.discord = String::from("starting");
        status.server_ok = true;

        Shared {
            status: Mutex::new(status),
            config: Mutex::new(config),
            log: Mutex::new(Vec::new()),
            paused: AtomicBool::new(false),
            config_version: AtomicU64::new(0),
            wake: Notify::new(),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
        self.wake.notify_one();
    }

    pub fn config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    pub fn replace_config(&self, config: Config) {
        *self.config.lock().unwrap() = config;
        self.config_version.fetch_add(1, Ordering::SeqCst);
        self.wake.notify_one();
    }

    pub fn config_version(&self) -> u64 {
        self.config_version.load(Ordering::SeqCst)
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    // Printed for the terminal as before, and kept for the Log tab.
    pub fn log(&self, text: impl Into<String>) {
        let text = text.into();
        println!("{}", text);

        let mut log = self.log.lock().unwrap();
        log.push(LogLine { at: now_ms(), text: text });

        if log.len() > LOG_LINES {
            let extra = log.len() - LOG_LINES;
            log.drain(0..extra);
        }
    }

    pub fn log_lines(&self) -> Vec<LogLine> {
        self.log.lock().unwrap().clone()
    }
}
