// Now Playing on macOS: asking the players themselves.
//
// macOS has a Now Playing of its own (the one in Control Center), but since
// macOS 15.4 only Apple's own apps may read it. So instead each player that
// can be asked is asked directly, over AppleScript, the way Shortcuts and
// Automator ask apps things. That is Music and Spotify; Cider has its own API
// (sources/cider.rs).
//
// The first time, macOS asks whether aridan-presence may control Music (or
// Spotify). Saying no turns this source off for that player; it can be
// turned back on in System Settings › Privacy & Security › Automation.
//
// Each question is a small script run with osascript. A player that is not
// open is never opened by asking: the script checks first.

use super::display_name;
use crate::config::Config;
use crate::state::PlayerSeen;
use crate::track::Track;
use tokio::process::Command;

// Fields in a script's answer are separated by this, a character no song
// title has in it.
const SEPARATOR: char = '\u{1f}';

struct Player {
    // The short name in the settings file.
    key: &'static str,
    script: &'static str,
}

// Each script answers with one line:
//   "closed"                                      not open
//   state, title, artist, album, position, length, cover
// with the times in milliseconds, worked out in the script as whole numbers
// so they never come back with a decimal comma.
const PLAYERS: [Player; 2] = [
    Player {
        key: "apple-music",
        script: r#"
            if application id "com.apple.Music" is not running then return "closed"
            set s to character id 31
            tell application id "com.apple.Music"
                set playerState to player state as text
                if playerState is not "playing" and playerState is not "paused" then return playerState
                set t to current track
                set songLength to ""
                try
                    set songLength to ((duration of t) * 1000) div 1
                end try
                return playerState & s & (name of t) & s & (artist of t) & s & (album of t) & s & ((player position * 1000) div 1) & s & songLength & s & ""
            end tell
        "#,
    },
    Player {
        key: "spotify",
        script: r#"
            if application id "com.spotify.client" is not running then return "closed"
            set s to character id 31
            tell application id "com.spotify.client"
                set playerState to player state as text
                if playerState is not "playing" and playerState is not "paused" then return playerState
                set t to current track
                return playerState & s & (name of t) & s & (artist of t) & s & (album of t) & s & ((player position * 1000) div 1) & s & (duration of t) & s & (artwork url of t)
            end tell
        "#,
    },
];

pub struct NowPlaying {
    allowed: Vec<String>,

    // Every player found open on the last look, allowed or not, for the
    // window.
    pub seen: Vec<PlayerSeen>,

    // Players macOS would not let us ask, so the log says so once rather
    // than every three seconds.
    refused: Vec<&'static str>,
}

impl NowPlaying {
    pub async fn new(config: &Config) -> NowPlaying {
        NowPlaying {
            allowed: config.allowed_players.clone(),
            seen: Vec::new(),
            refused: Vec::new(),
        }
    }

    // The first allowed player that is playing, in the order allowed_players
    // lists them - so with Music and Spotify both playing, Music wins.
    pub async fn current(&mut self) -> Option<Track> {
        let mut seen = Vec::new();
        let mut candidates: Vec<(usize, Track)> = Vec::new();

        for player in &PLAYERS {
            let rank = self.allowed.iter().position(|allowed| allowed == player.key);

            // A player that may not be shared is still looked at, so the
            // window can say it was seen - but only if it is open anyway.
            let Some(answer) = self.ask(player).await else {
                continue;
            };

            if answer == "closed" {
                continue;
            }

            let track = read_track(&answer, player.key);
            let playing = track.is_some();

            seen.push(PlayerSeen {
                key: player.key.to_string(),
                name: display_name(player.key),
                allowed: rank.is_some(),
                playing: playing,
            });

            if let (Some(rank), Some(track)) = (rank, track) {
                candidates.push((rank, track));
            }
        }

        self.seen = seen;

        candidates.sort_by_key(|candidate| candidate.0);
        candidates.into_iter().next().map(|(_rank, track)| track)
    }

    async fn ask(&mut self, player: &Player) -> Option<String> {
        let output = Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(player.script)
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);

            // -1743: the person said no to "control Music".
            if error.contains("-1743") && !self.refused.contains(&player.key) {
                self.refused.push(player.key);
                eprintln!(
                    "Now Playing: not allowed to ask {}. Allow it in System Settings > Privacy & Security > Automation.",
                    display_name(player.key)
                );
            }

            return None;
        }

        let answer = String::from_utf8_lossy(&output.stdout).trim_end_matches('\n').to_string();
        Some(answer)
    }
}

// A track from a script's answer, or None if nothing is playing.
fn read_track(answer: &str, player: &str) -> Option<Track> {
    let fields: Vec<&str> = answer.split(SEPARATOR).collect();

    if fields.len() < 7 || fields[0] != "playing" {
        return None;
    }

    let title = fields[1].trim().to_string();
    if title.is_empty() {
        return None;
    }

    let position_ms = fields[4].trim().parse::<u64>().unwrap_or(0);
    let length_ms = fields[5].trim().parse::<u64>().ok().filter(|length| *length > 0);

    // Spotify hands over a web address for its cover; Music does not, so
    // iTunes is asked instead - see artwork.rs.
    let cover = fields[6].trim();
    let artwork = if cover.starts_with("https://") {
        Some(cover.to_string())
    } else {
        None
    };

    let track = Track {
        player: display_name(player),
        source: "nowplaying",
        title: title,
        artist: fields[2].trim().to_string(),
        album: fields[3].trim().to_string(),
        artwork: artwork,
        link: None,
        position_ms: position_ms,
        length_ms: length_ms,
    };

    Some(track)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_playing_song() {
        let answer = ["playing", "Popila", "Severina", "Dobrodošli na Balkan", "61000", "215000", ""].join("\u{1f}");
        let track = read_track(&answer, "apple-music").expect("a track");

        assert_eq!(track.player, "Apple Music");
        assert_eq!(track.title, "Popila");
        assert_eq!(track.artist, "Severina");
        assert_eq!(track.position_ms, 61000);
        assert_eq!(track.length_ms, Some(215000));
        assert_eq!(track.artwork, None);
    }

    #[test]
    fn leaves_out_a_paused_song() {
        let answer = ["paused", "Popila", "Severina", "", "0", "215000", ""].join("\u{1f}");
        assert!(read_track(&answer, "apple-music").is_none());
    }
}
