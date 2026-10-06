// Now Playing on Linux, which is called MPRIS.
//
// Every player that wants to appear in the media controls - Cider, Spotify,
// Firefox, Chrome - registers itself on D-Bus, the message bus programs on
// the desktop use to talk to each other, under a name like
// "org.mpris.MediaPlayer2.spotify". We list those names, skip the ones that
// are not allowed, and ask the rest what they are playing.

use super::display_name;
use crate::config::Config;
use crate::state::PlayerSeen;
use crate::track::Track;
use std::collections::HashMap;
use zbus::proxy::CacheProperties;
use zbus::zvariant::{OwnedValue, Value};
use zbus::Connection;

const NAME_PREFIX: &str = "org.mpris.MediaPlayer2.";
const OBJECT_PATH: &str = "/org/mpris/MediaPlayer2";
const ROOT_INTERFACE: &str = "org.mpris.MediaPlayer2";
const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

pub struct NowPlaying {
    // None if D-Bus could not be reached, which only happens outside a
    // desktop session - over SSH, say.
    connection: Option<Connection>,
    allowed: Vec<String>,

    // Every player found on the last look, allowed or not, for the window.
    pub seen: Vec<PlayerSeen>,
}

impl NowPlaying {
    pub async fn new(config: &Config) -> NowPlaying {
        let connection = match Connection::session().await {
            Ok(connection) => Some(connection),
            Err(error) => {
                eprintln!("Now Playing is off: could not reach D-Bus ({})", error);
                None
            }
        };

        NowPlaying {
            connection: connection,
            allowed: config.allowed_players.clone(),
            seen: Vec::new(),
        }
    }

    // The first allowed player that is playing, in the order allowed_players
    // lists them - so with Cider and Firefox both playing, Cider wins.
    pub async fn current(&mut self) -> Option<Track> {
        let connection = self.connection.as_ref()?;

        let mut candidates: Vec<(usize, String, String)> = Vec::new();
        let mut seen = Vec::new();

        for bus_name in list_players(connection).await {
            let Some(player) = player_of(connection, &bus_name).await else {
                continue;
            };

            let status: String = read_property(connection, &bus_name, PLAYER_INTERFACE, "PlaybackStatus")
                .await
                .unwrap_or_default();

            let rank = self.allowed.iter().position(|allowed| *allowed == player);

            seen.push(PlayerSeen {
                key: player.clone(),
                name: display_name(&player),
                allowed: rank.is_some(),
                playing: status == "Playing",
            });

            let Some(rank) = rank else {
                continue;
            };

            candidates.push((rank, player, bus_name));
        }

        self.seen = seen;

        candidates.sort();

        for (_rank, player, bus_name) in candidates {
            if let Some(track) = read_track(connection, &bus_name, &player).await {
                return Some(track);
            }
        }

        None
    }
}

async fn list_players(connection: &Connection) -> Vec<String> {
    let Ok(dbus) = zbus::fdo::DBusProxy::new(connection).await else {
        return Vec::new();
    };

    let Ok(names) = dbus.list_names().await else {
        return Vec::new();
    };

    let mut players = Vec::new();
    for name in names {
        let name = name.to_string();
        if name.starts_with(NAME_PREFIX) {
            players.push(name);
        }
    }

    players
}

// The short name from the settings file for a player on the bus, or None for
// one we would never share.
//
// Most players put their own name right after the prefix:
// "org.mpris.MediaPlayer2.firefox.instance_1_84" is Firefox. Every browser
// built on Chromium says "chromium" there, though - Brave and Edge included -
// so for those the player's Identity says which one it really is.
async fn player_of(connection: &Connection, bus_name: &str) -> Option<String> {
    let rest = bus_name.strip_prefix(NAME_PREFIX)?;
    let first_part = rest.split('.').next()?.to_lowercase();

    let is_chromium_based = first_part == "chromium" || first_part == "chrome" || first_part == "google-chrome";
    if !is_chromium_based {
        return Some(first_part);
    }

    let identity: String = read_property(connection, bus_name, ROOT_INTERFACE, "Identity").await?;
    let identity = identity.to_lowercase();

    if identity.contains("chrome") || identity == "chromium" {
        return Some(String::from("chrome"));
    }

    // "brave", "microsoft edge", ... - not on the list unless added to it.
    Some(identity.replace(' ', "-"))
}

async fn read_track(connection: &Connection, bus_name: &str, player: &str) -> Option<Track> {
    let status: String = read_property(connection, bus_name, PLAYER_INTERFACE, "PlaybackStatus").await?;
    if status != "Playing" {
        return None;
    }

    let metadata: HashMap<String, OwnedValue> =
        read_property(connection, bus_name, PLAYER_INTERFACE, "Metadata").await?;

    let title = text(&metadata, "xesam:title");
    if title.is_empty() {
        return None;
    }

    // MPRIS counts time in microseconds; the site counts in milliseconds.
    let position_us: i64 = read_property(connection, bus_name, PLAYER_INTERFACE, "Position")
        .await
        .unwrap_or(0);
    let position_ms = (position_us.max(0) / 1000) as u64;

    let mut length_ms = None;
    if let Some(length_us) = number(&metadata, "mpris:length") {
        if length_us > 0 {
            length_ms = Some(length_us as u64 / 1000);
        }
    }

    // Firefox and Chrome hand over a file in their own cache, not a web
    // address, so those are dropped and iTunes is asked instead.
    let mut artwork = None;
    let art_url = text(&metadata, "mpris:artUrl");
    if art_url.starts_with("https://") {
        artwork = Some(art_url);
    }

    let track = Track {
        player: display_name(player),
        source: "nowplaying",
        title: title,
        artist: list(&metadata, "xesam:artist").join(", "),
        album: text(&metadata, "xesam:album"),
        artwork: artwork,
        link: None,
        position_ms: position_ms,
        length_ms: length_ms,
    };

    Some(track)
}

// Reads one property from a player. Properties are fetched fresh every time:
// zbus would otherwise remember them, and Position in particular is never
// announced when it changes, so a remembered one would stay stuck.
//
// `T: TryFrom<OwnedValue>` means: any type that a D-Bus value can be turned
// into - String, i64, a HashMap... The caller picks which.
async fn read_property<T>(connection: &Connection, bus_name: &str, interface: &str, name: &str) -> Option<T>
where
    T: TryFrom<OwnedValue>,
    T::Error: Into<zbus::Error>,
{
    let proxy: zbus::Proxy = zbus::proxy::Builder::new(connection)
        .destination(bus_name.to_string())
        .ok()?
        .path(OBJECT_PATH)
        .ok()?
        .interface(interface.to_string())
        .ok()?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .ok()?;

    proxy.get_property(name).await.ok()
}

/* ---------- Reading the metadata ----------
 *
 * Metadata is a map from names like "xesam:title" to values that can be any
 * D-Bus type, so each one is checked for the type we expect before use.
 */

fn text(metadata: &HashMap<String, OwnedValue>, key: &str) -> String {
    let Some(value) = metadata.get(key) else {
        return String::new();
    };

    // `&**value` looks through the OwnedValue wrapper at the Value inside.
    match &**value {
        Value::Str(text) => text.to_string(),
        _ => String::new(),
    }
}

// Artists come as a list, since a song can have several.
fn list(metadata: &HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    let mut items = Vec::new();

    let Some(value) = metadata.get(key) else {
        return items;
    };

    match &**value {
        Value::Array(array) => {
            for item in array.iter() {
                if let Value::Str(text) = item {
                    items.push(text.to_string());
                }
            }
        }

        // Some players send a single artist as plain text instead.
        Value::Str(text) => items.push(text.to_string()),

        _ => {}
    }

    items
}

// The spec says length is a signed 64-bit number, but some players send it
// unsigned, or as a smaller type.
fn number(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<i64> {
    let value = metadata.get(key)?;

    match &**value {
        Value::I64(number) => Some(*number),
        Value::U64(number) => Some(*number as i64),
        Value::I32(number) => Some(*number as i64),
        Value::U32(number) => Some(*number as i64),
        Value::F64(number) => Some(*number as i64),
        _ => None,
    }
}
