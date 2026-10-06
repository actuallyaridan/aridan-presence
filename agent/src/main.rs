// aridan-presence: tells presence.aridan.net what I am listening to and
// playing, so the music widget on aridan.net works without Discord open.
//
// Every few seconds it asks each source what is going on, puts the answers
// together into one list of activities, and sends that to the Worker - when
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

// Each `mod` line pulls in the file of that name.
mod activity;
mod artwork;
mod config;
mod server;
mod sources;
mod track;

use activity::{Activity, LISTENING};
use artwork::Artwork;
use server::Server;
use sources::cider::Cider;
use sources::discord_ipc::DiscordIpc;
use sources::now_playing::NowPlaying;
use std::time::{Duration, Instant};
use track::TrackClock;

const POLL_INTERVAL: Duration = Duration::from_secs(3);

// Must stay well under the Worker's 90 seconds (DEVICE_TTL_MS in merge.js),
// or this computer would blink out between reports.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

// Everything the main loop asks, kept together so gather() can take one thing.
struct Sources {
    cider: Cider,
    discord: DiscordIpc,
    now_playing: NowPlaying,
    artwork: Artwork,
    clock: TrackClock,
}

// `#[tokio::main]` sets up tokio, the library that lets one thread wait on
// many things at once - the network, D-Bus, timers - which is what `async`
// and `.await` are about. current_thread keeps it to a single thread, which
// is plenty for something that is mostly asleep.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let config = match config::load() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{}", message);
            std::process::exit(1);
        }
    };

    println!("Reporting to {} as \"{}\".", config.server, config.device);

    let server = Server::new(&config);

    let mut sources = Sources {
        cider: Cider::new(&config),
        discord: DiscordIpc::start(),
        now_playing: NowPlaying::new(&config).await,
        artwork: Artwork::new(config.itunes_countries.clone()),
        clock: TrackClock::new(),
    };

    // None until the first report gets through.
    let mut last_sent: Option<Vec<Activity>> = None;
    let mut last_sent_at = Instant::now();
    let mut server_was_ok = true;

    // Made once and checked on every turn of the loop, so Ctrl+C or the
    // system shutting us down is noticed straight away, even mid-wait.
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    let mut ticker = tokio::time::interval(POLL_INTERVAL);

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = &mut shutdown => break,
        }

        let activities = gather(&mut sources).await;

        let changed = last_sent.as_ref() != Some(&activities);
        let heartbeat_due = last_sent_at.elapsed() >= HEARTBEAT_INTERVAL;

        if !changed && !heartbeat_due {
            continue;
        }

        if changed {
            describe(&activities);
        }

        match server.report(&activities).await {
            Ok(()) => {
                if !server_was_ok {
                    println!("Reaching the server again.");
                }
                server_was_ok = true;

                last_sent = Some(activities);
                last_sent_at = Instant::now();
            }
            Err(message) => {
                // Said once, not every three seconds while the internet is out.
                if server_was_ok {
                    eprintln!("Could not report: {}. Will keep trying.", message);
                }
                server_was_ok = false;
            }
        }
    }

    println!("Stopping. Clearing this computer from the site.");
    if let Err(message) = server.clear().await {
        eprintln!("Could not clear: {}", message);
    }
}

async fn gather(sources: &mut Sources) -> Vec<Activity> {
    let from_discord = sources.discord.activities();

    // 1. Cider
    let mut music: Option<Activity> = None;

    if let Some(track) = sources.cider.current().await {
        let start = sources.clock.start_of(&track);
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

    // 3. Now Playing
    if music.is_none() {
        if let Some(mut track) = sources.now_playing.current().await {
            sources.artwork.fill_in(&mut track).await;

            let start = sources.clock.start_of(&track);
            music = Some(track.into_activity(start));
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

// One line per activity in the terminal, so it is easy to see what is being
// sent without reading JSON.
fn describe(activities: &[Activity]) {
    if activities.is_empty() {
        println!("Now: nothing");
        return;
    }

    for activity in activities {
        let details = activity.details.clone().unwrap_or_default();
        let state = activity.state.clone().unwrap_or_default();

        println!("Now: [{}] {} - {} / {}", activity.source, activity.name, details, state);
    }
}

// Ctrl+C in a terminal, or - on Linux and macOS - the polite "please stop"
// that systemd and launchd send when logging out or shutting down.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut terminate = signal(SignalKind::terminate()).expect("should be able to listen for SIGTERM");

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
