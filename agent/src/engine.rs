// The part that does the work, running in the background for as long as the
// app is open - with or without its window.
//
// Every few seconds it asks each source what is going on, puts the answers
// together into one list of activities, and sends that to the Worker: when
// it has changed, and every 30 seconds regardless, so the Worker knows this
// computer is still here.
//
// Music comes from the first of these with an answer:
//   1. Cider's own API                 sources/cider.rs
//   2. Discord rich presence           sources/discord_ipc.rs
//   3. The system's Now Playing        sources/now_playing/
//
// Games and other apps that report to Discord are sent alongside the music,
// whichever source the music came from.

use crate::activity::{Activity, Timestamps, LISTENING};
use crate::artwork::Artwork;
use crate::server::Server;
use crate::sources::cider::Cider;
use crate::sources::discord_ipc::DiscordIpc;
use crate::sources::now_playing::NowPlaying;
use crate::state::{Shared, Status};
use crate::track::{Track, TrackClock};
use crate::tray;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const POLL_INTERVAL: Duration = Duration::from_secs(3);

// Must stay well under the Worker's 90 seconds (DEVICE_TTL_MS in merge.js),
// or this computer would blink out between reports.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

// Everything asked each tick. Made again from scratch whenever Settings is
// saved, since most of it depends on the settings.
struct Sources {
    cider: Cider,
    now_playing: NowPlaying,
    artwork: Artwork,
}

// What has been sent to the Worker, kept across Settings changes so that
// changing, say, the Cider token does not make the song blink off the site.
struct Reported {
    server: Option<Server>,
    activities: Option<Vec<Activity>>,
    at: Instant,
    server_was_ok: bool,
    server_message: String,
}

pub async fn run(app: AppHandle, shared: Arc<Shared>) {
    shared.log("Starting.");

    // These two outlive Settings changes: the Discord socket should not be
    // dropped and taken again, and a song's start time should not jump.
    let discord = DiscordIpc::start(shared.clone());
    let mut clock = TrackClock::new();

    let mut reported = Reported {
        server: None,
        activities: None,
        at: Instant::now(),
        server_was_ok: true,
        server_message: String::new(),
    };

    let mut ticker = tokio::time::interval(POLL_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // The outer loop runs once per version of the settings; the inner one
    // once per tick, until the settings change.
    loop {
        let version = shared.config_version();
        let config = shared.config();

        let server = Server::new(&config);

        let mut sources = Sources {
            cider: Cider::new(&config),
            now_playing: NowPlaying::new(&config).await,
            artwork: Artwork::new(config.itunes_countries.clone()),
        };

        // Reporting somewhere else now - a new computer name or server - so
        // the old place is told this computer has gone.
        if let Some(old) = reported.server.take() {
            if old.target() != server.target() {
                let _ = old.clear().await;
                reported.activities = None;
            }
        }

        if config.is_set_up() {
            shared.log(format!("Reporting to {} as \"{}\".", config.server, config.device_name()));
        } else {
            shared.log("Waiting for a token in Settings before reporting anything.");
        }

        loop {
            // Whichever comes first: the next tick, or something in the window
            // or tray that wants an answer now.
            tokio::select! {
                _ = ticker.tick() => {}
                _ = shared.wake.notified() => {}
            }

            if shared.config_version() != version {
                break;
            }

            let activities = gather(&mut sources, &discord, &mut clock).await;
            let paused = shared.is_paused();

            if !config.is_set_up() {
                // Nothing to do but show what would be shared.
            } else if paused {
                withdraw(&shared, &server, &mut reported).await;
            } else {
                report(&shared, &server, &activities, &mut reported).await;
            }

            let mut status = shared.status();
            status.set_up = config.is_set_up();
            status.paused = paused;
            status.device = config.device_name();
            status.server_ok = reported.server_was_ok;
            status.server_message = reported.server_message.clone();
            status.cider = sources.cider.status.to_string();
            status.discord = discord.mode().to_string();
            status.players = sources.now_playing.seen.clone();
            status.activities = activities;

            publish(&app, &shared, status);
        }
    }
}

async fn gather(sources: &mut Sources, discord: &DiscordIpc, clock: &mut TrackClock) -> Vec<Activity> {
    let from_discord = discord.activities();

    // 1. Cider
    let mut music: Option<Activity> = None;

    if let Some(track) = sources.cider.current().await {
        let start = clock.start_of(&track);
        music = Some(track.into_activity(start));
    }

    // 2. Music an app reported to Discord
    if music.is_none() {
        for activity in &from_discord {
            if activity.kind == LISTENING {
                music = Some(activity.clone());
                break;
            }
        }
    }

    // 3. Now Playing. Asked every tick even when it is not needed, so the
    // Sources list in the window stays current.
    let from_now_playing = sources.now_playing.current().await;

    if let Some(mut track) = from_now_playing {
        if music.is_none() {
            sources.artwork.fill_in(&mut track).await;

            let start = clock.start_of(&track);
            music = Some(track.into_activity(start));
        } else if let Some(found) = music.as_mut() {
            // Cider's Discord presence only says when the song ends, not when
            // it started, so there is no telling how far in it is. Now
            // Playing knows, and when it is the same song its times are used.
            borrow_times(found, &track, clock);
        }
    }

    // Music first, then everything else from Discord. Other music from
    // Discord is left out: only one song is ever really playing.
    let mut activities = Vec::new();

    if let Some(music) = music {
        activities.push(music);
    }

    for activity in from_discord {
        if activity.kind != LISTENING {
            activities.push(activity);
        }
    }

    activities
}

fn borrow_times(music: &mut Activity, track: &Track, clock: &mut TrackClock) {
    let has_start = music.timestamps.as_ref().and_then(|times| times.start).is_some();
    if has_start {
        return;
    }

    let title = music.details.clone().unwrap_or_default();
    if simplified(&title) != simplified(&track.title) {
        return;
    }

    let Some(length) = track.length_ms else {
        return;
    };

    let start = clock.start_of(track);

    let mut timestamps = Timestamps::default();
    timestamps.start = Some(start);
    timestamps.end = Some(start + length);
    music.timestamps = Some(timestamps);
}

// Lowercase, with runs of spaces made single: Cider's Discord presence and
// its Now Playing do not always space a title the same way.
fn simplified(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    words.join(" ").to_lowercase()
}

async fn report(shared: &Shared, server: &Server, activities: &[Activity], reported: &mut Reported) {
    let changed = reported.activities.as_deref() != Some(activities);
    let heartbeat_due = reported.at.elapsed() >= HEARTBEAT_INTERVAL;

    if !changed && !heartbeat_due {
        return;
    }

    if changed {
        describe(shared, activities);
    }

    match server.report(activities).await {
        Ok(()) => {
            if !reported.server_was_ok {
                shared.log("Reaching the server again.");
            }

            reported.server_was_ok = true;
            reported.server = Some(server.clone());
            reported.activities = Some(activities.to_vec());
            reported.at = Instant::now();
            reported.server_message.clear();
        }
        Err(message) => {
            // Said once, not every three seconds while the internet is out.
            if reported.server_was_ok {
                shared.log(format!("Could not report: {}. Will keep trying.", message));
            }

            reported.server_was_ok = false;
            reported.server_message = message;
        }
    }
}

// Paused: whatever is on the site comes off it, once.
async fn withdraw(shared: &Shared, server: &Server, reported: &mut Reported) {
    if reported.activities.is_none() {
        return;
    }

    match server.clear().await {
        Ok(()) => {
            shared.log("Paused: cleared this computer from the site.");
            reported.activities = None;
            reported.server = None;
        }
        Err(message) => {
            shared.log(format!("Could not clear: {}. Will keep trying.", message));
        }
    }
}

// One line per activity in the Log tab, so it is easy to see what is being
// sent without reading JSON.
fn describe(shared: &Shared, activities: &[Activity]) {
    if activities.is_empty() {
        shared.log("Now: nothing");
        return;
    }

    for activity in activities {
        let details = activity.details.clone().unwrap_or_default();
        let state = activity.state.clone().unwrap_or_default();

        shared.log(format!("Now: [{}] {} - {} / {}", activity.source, activity.name, details, state));
    }
}

// Stores the new status, and if anything in it changed, tells the window and
// updates the tray.
fn publish(app: &AppHandle, shared: &Shared, status: Status) {
    {
        let mut current = shared.status.lock().unwrap();

        if *current == status {
            return;
        }

        *current = status.clone();
    }

    let _ = app.emit("status", &status);
    tray::update(app, &status);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::now_ms;

    fn track(title: &str) -> Track {
        Track {
            player: String::from("Apple Music"),
            source: "nowplaying",
            title: title.to_string(),
            artist: String::from("Journey"),
            album: String::new(),
            artwork: None,
            link: None,
            position_ms: 60_000,
            length_ms: Some(250_000),
        }
    }

    fn from_cider_via_discord(title: &str) -> Activity {
        let mut timestamps = Timestamps::default();
        timestamps.end = Some(now_ms() + 190_000);

        Activity {
            kind: LISTENING,
            name: String::from("Apple Music"),
            source: "discord",
            details: Some(title.to_string()),
            state: None,
            application_id: None,
            url: None,
            timestamps: Some(timestamps),
            assets: None,
        }
    }

    #[test]
    fn fills_in_the_start_from_now_playing() {
        let mut clock = TrackClock::new();
        let mut music = from_cider_via_discord("Don't Stop Believin'  (2022 Remaster)");

        borrow_times(&mut music, &track("Don't Stop Believin' (2022 Remaster)"), &mut clock);

        let times = music.timestamps.unwrap();
        let start = times.start.expect("a start time");
        assert_eq!(times.end, Some(start + 250_000));
        assert!(now_ms() - start >= 60_000);
    }

    #[test]
    fn leaves_a_different_song_alone() {
        let mut clock = TrackClock::new();
        let mut music = from_cider_via_discord("Any Way You Want It");

        borrow_times(&mut music, &track("Don't Stop Believin'"), &mut clock);

        assert_eq!(music.timestamps.unwrap().start, None);
    }
}
