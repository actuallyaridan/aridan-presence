// Source 1: Cider's own API.
//
// Cider runs a small web server on this computer (port 10767) that answers
// questions about what it is playing. It knows more than Now Playing does -
// most usefully the song's Apple Music link - so it is asked first.
//
// It wants an app token, made in Cider > Settings > Connectivity. Without
// one this source stays quiet and Cider is picked up by Now Playing instead.

use crate::config::Config;
use crate::track::Track;
use serde::Deserialize;
use std::time::Duration;

const API: &str = "http://127.0.0.1:10767/api/v1/playback";

// Apple's artwork addresses have the size left as a blank to fill in:
// ".../{w}x{h}bb.jpg".
const SIZE_PLACEHOLDER: &str = "{w}x{h}";
const WANTED_SIZE: &str = "600x600";

#[derive(Deserialize)]
struct IsPlaying {
    is_playing: bool,
}

#[derive(Deserialize)]
struct NowPlayingResponse {
    info: Option<Info>,
}

// Cider passes on Apple's own names for these, which are camelCase:
// "artistName", "durationInMillis". rename_all matches them up with Rust's
// snake_case: artist_name, duration_in_millis.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Info {
    name: Option<String>,
    artist_name: Option<String>,
    album_name: Option<String>,
    artwork: Option<ArtworkInfo>,
    url: Option<String>,
    duration_in_millis: Option<u64>,

    // In seconds, with a fraction.
    current_playback_time: Option<f64>,
}

#[derive(Deserialize)]
struct ArtworkInfo {
    url: Option<String>,
}

pub struct Cider {
    client: reqwest::Client,
    token: String,

    // So a wrong token is mentioned once, not every three seconds.
    warned: bool,
}

impl Cider {
    pub fn new(config: &Config) -> Cider {
        // It is on this computer, so anything slower than this means Cider is
        // stuck rather than busy.
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("the HTTP client should always build");

        Cider {
            client: client,
            token: config.cider_token.trim().to_string(),
            warned: false,
        }
    }

    // None when Cider is closed, paused, or there is no token.
    pub async fn current(&mut self) -> Option<Track> {
        if self.token.is_empty() {
            return None;
        }

        let playing: IsPlaying = self.get("is-playing").await?;
        if !playing.is_playing {
            return None;
        }

        let response: NowPlayingResponse = self.get("now-playing").await?;
        let info = response.info?;

        let mut artwork = None;
        if let Some(art) = info.artwork {
            if let Some(url) = art.url {
                artwork = Some(url.replace(SIZE_PLACEHOLDER, WANTED_SIZE));
            }
        }

        let seconds_in = info.current_playback_time.unwrap_or(0.0);
        let position_ms = (seconds_in * 1000.0) as u64;

        let track = Track {
            player: String::from("Apple Music"),
            source: "cider",
            title: info.name.unwrap_or_default(),
            artist: info.artist_name.unwrap_or_default(),
            album: info.album_name.unwrap_or_default(),
            artwork: artwork,
            link: info.url,
            position_ms: position_ms,
            length_ms: info.duration_in_millis,
        };

        Some(track)
    }

    // `<T: Deserialize>` means this works for any of the response types
    // above: the caller says which one it expects back.
    async fn get<T: for<'de> Deserialize<'de>>(&mut self, path: &str) -> Option<T> {
        let url = format!("{}/{}", API, path);

        // Cider not running at all is the usual case, and not worth a message.
        let response = self
            .client
            .get(&url)
            .header("apptoken", &self.token)
            .send()
            .await
            .ok()?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED
            || response.status() == reqwest::StatusCode::FORBIDDEN
        {
            if !self.warned {
                eprintln!("Cider did not accept cider_token. Using Now Playing for Cider instead.");
                self.warned = true;
            }
            return None;
        }

        if !response.status().is_success() {
            return None;
        }

        response.json().await.ok()
    }
}
