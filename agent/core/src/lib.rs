// The part of aridan-presence that does the work, shared by both windows.
//
// engine.rs is where to start: it runs in the background for as long as the
// app is open, asks the sources what is playing, and reports it. Everything
// else here is something it uses. state.rs is what it shares with the
// window and the tray.

// `pub mod` makes each file usable from the apps, as presence_core::config
// and so on.
pub mod activity;
pub mod artwork;
pub mod config;
pub mod engine;
pub mod server;
pub mod sources;
pub mod state;
pub mod track;
