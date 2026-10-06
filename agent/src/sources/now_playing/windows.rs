// Now Playing on Windows: the media controls that show over the volume
// flyout and on the lock screen. Windows calls them Global System Media
// Transport Controls, and every player that shows up there - Cider, Spotify,
// the Apple Music app, browsers - is a "session" we can ask about.
//
// Each session says which app it belongs to with an app id. For a classic
// program that is usually its file name ("Spotify.exe"), for a Store app a
// long package name ("AppleInc.AppleMusicWin_nzyj5cx40ttqa!App"), and a few
// pick their own ("Chrome"). player_of() turns those into the short names
// the settings file uses.

use super::display_name;
use crate::config::Config;
use crate::state::PlayerSeen;
use crate::track::Track;
use std::time::{SystemTime, UNIX_EPOCH};
use ::windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};

// Windows counts time in ticks of 100 nanoseconds. Dates count from 1601,
// not 1970, so this many ticks have to come off to line them up.
const TICKS_PER_MS: i64 = 10_000;
const TICKS_FROM_1601_TO_1970: i64 = 116_444_736_000_000_000;

// Firefox does not use its file name as its app id but this, a hash of where
// it is installed, which is the same on every normal install.
const FIREFOX_APP_ID: &str = "308046b0af4a39cb";

pub struct NowPlaying {
    // None if Windows would not hand over the media controls at all, which
    // only happens on versions older than Windows 10 1809.
    manager: Option<SessionManager>,
    allowed: Vec<String>,

    // Every player found on the last look, allowed or not, for the window.
    pub seen: Vec<PlayerSeen>,
}

impl NowPlaying {
    pub async fn new(config: &Config) -> NowPlaying {
        join_windows_threads();

        let manager = match request_manager().await {
            Ok(manager) => Some(manager),
            Err(error) => {
                eprintln!("Now Playing is off: Windows did not hand over its media controls ({})", error);
                None
            }
        };

        NowPlaying {
            manager: manager,
            allowed: config.allowed_players.clone(),
            seen: Vec::new(),
        }
    }

    // The first allowed player that is playing, in the order allowed_players
    // lists them - so with Cider and Firefox both playing, Cider wins.
    pub async fn current(&mut self) -> Option<Track> {
        let manager = self.manager.as_ref()?;
        let sessions = list_sessions(manager);

        let mut candidates: Vec<(usize, String, Session)> = Vec::new();
        let mut seen = Vec::new();

        for session in sessions {
            let app_id = match session.SourceAppUserModelId() {
                Ok(id) => id.to_string(),
                Err(_) => continue,
            };

            let Some(player) = player_of(&app_id) else {
                continue;
            };

            let playing = is_playing(&session);
            let rank = self.allowed.iter().position(|allowed| *allowed == player);

            seen.push(PlayerSeen {
                key: player.clone(),
                name: display_name(&player),
                allowed: rank.is_some(),
                playing: playing,
            });

            let Some(rank) = rank else {
                continue;
            };

            if playing {
                candidates.push((rank, player, session));
            }
        }

        self.seen = seen;

        // Sorted by rank only. Sessions themselves cannot be compared.
        candidates.sort_by_key(|candidate| candidate.0);

        for (_rank, player, session) in candidates {
            if let Some(track) = read_track(&session, &player).await {
                return Some(track);
            }
        }

        None
    }
}

// Windows' media controls are COM objects, and COM wants every thread that
// uses it to have said so first. This says so once for every thread in the
// app at the same time, so the background threads tokio runs the engine on
// do not each have to.
fn join_windows_threads() {
    // `unsafe` because it is a raw Windows call; it is safe to make any
    // number of times. Failing only means COM was already set up some other
    // way, which is just as good.
    let _ = unsafe { ::windows::Win32::System::Com::CoIncrementMTAUsage() };
}

// Copied out into a plain list. The list Windows hands back may only be used
// on the thread that asked for it, and the engine can carry on on another
// thread after any await - the sessions in it have no such limit.
fn list_sessions(manager: &SessionManager) -> Vec<Session> {
    let mut list = Vec::new();

    let Ok(sessions) = manager.GetSessions() else {
        return list;
    };

    let count = sessions.Size().unwrap_or(0);
    for index in 0..count {
        if let Ok(session) = sessions.GetAt(index) {
            list.push(session);
        }
    }

    list
}

async fn request_manager() -> ::windows::core::Result<SessionManager> {
    SessionManager::RequestAsync()?.await
}

fn is_playing(session: &Session) -> bool {
    let Ok(info) = session.GetPlaybackInfo() else {
        return false;
    };

    match info.PlaybackStatus() {
        Ok(status) => status == PlaybackStatus::Playing,
        Err(_) => false,
    }
}

// The short name from the settings file for an app id, or None for one
// without a usable name.
fn player_of(app_id: &str) -> Option<String> {
    let id = app_id.to_lowercase();

    if id.is_empty() {
        return None;
    }

    if id.contains("cider") {
        return Some(String::from("cider"));
    }
    if id.contains("applemusic") {
        return Some(String::from("apple-music"));
    }
    if id.contains("spotify") {
        return Some(String::from("spotify"));
    }
    if id.contains("firefox") || id == FIREFOX_APP_ID {
        return Some(String::from("firefox"));
    }
    if id == "chrome" || id.contains("chrome.exe") {
        return Some(String::from("chrome"));
    }
    if id == "msedge" || id.contains("msedge.exe") {
        return Some(String::from("edge"));
    }

    // Anything else: its file name without the folder and ".exe", or a Store
    // app's name without the publisher's code - "vlc.exe" becomes "vlc".
    let mut name = id.rsplit(['\\', '/']).next().unwrap_or(&id).to_string();
    if let Some(stripped) = name.strip_suffix(".exe") {
        name = stripped.to_string();
    }
    if let Some((before, _)) = name.split_once('_') {
        name = before.to_string();
    }
    if let Some((_, after)) = name.rsplit_once('.') {
        name = after.to_string();
    }

    if name.is_empty() {
        return None;
    }

    Some(name)
}

async fn read_track(session: &Session, player: &str) -> Option<Track> {
    let properties = session.TryGetMediaPropertiesAsync().ok()?.await.ok()?;

    let title = properties.Title().ok()?.to_string();
    if title.trim().is_empty() {
        return None;
    }

    let artist = properties.Artist().map(|text| text.to_string()).unwrap_or_default();
    let album = properties.AlbumTitle().map(|text| text.to_string()).unwrap_or_default();

    let (position_ms, length_ms) = read_timeline(session);

    // Windows hands over the cover as an image file in memory, not a web
    // address the site could load, so it is left out and iTunes is asked
    // instead - see artwork.rs.
    let track = Track {
        player: display_name(player),
        source: "nowplaying",
        title: title,
        artist: artist,
        album: album,
        artwork: None,
        link: None,
        position_ms: position_ms,
        length_ms: length_ms,
    };

    Some(track)
}

// How far into the song, and how long it is, in milliseconds.
//
// Players only tell Windows their position now and then - when a song
// starts, on a seek, on pause - along with when they did. So the position is
// what they last said plus however long it has been since.
fn read_timeline(session: &Session) -> (u64, Option<u64>) {
    let Ok(timeline) = session.GetTimelineProperties() else {
        return (0, None);
    };

    let start = timeline.StartTime().map(|time| time.Duration).unwrap_or(0);
    let end = timeline.EndTime().map(|time| time.Duration).unwrap_or(0);
    let position = timeline.Position().map(|time| time.Duration).unwrap_or(0);

    let mut length_ms = None;
    if end > start {
        length_ms = Some(((end - start) / TICKS_PER_MS) as u64);
    }

    let mut position_ms = ((position - start).max(0) / TICKS_PER_MS) as u64;

    if let Ok(updated) = timeline.LastUpdatedTime() {
        let updated_ms = (updated.UniversalTime - TICKS_FROM_1601_TO_1970) / TICKS_PER_MS;
        let since_ms = now_ms() - updated_ms;

        // A time in the future, or one from days ago, is a player that never
        // set it properly. Better to leave the position as it was than to
        // add nonsense to it.
        if updated_ms > 0 && since_ms > 0 && since_ms < 24 * 60 * 60 * 1000 {
            position_ms += since_ms as u64;
        }
    }

    // Never past the end of the song.
    if let Some(length) = length_ms {
        position_ms = position_ms.min(length);
    }

    (position_ms, length_ms)
}

fn now_ms() -> i64 {
    let since_1970 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    since_1970.as_millis() as i64
}
