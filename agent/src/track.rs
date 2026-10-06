// A song, as Cider or Now Playing describes it, and how it becomes an
// Activity for the site.

use crate::activity::{non_empty, now_ms, Activity, Assets, Timestamps, LISTENING};

pub struct Track {
    // What the site shows as the app: "Apple Music", "Spotify", "Firefox"...
    pub player: String,

    // "cider" or "nowplaying".
    pub source: &'static str,

    pub title: String,
    pub artist: String,
    pub album: String,

    // Only https addresses are kept. Some players hand over a file on disk
    // instead, which is no use to anyone looking at the website.
    pub artwork: Option<String>,

    // The song's own page, like a music.apple.com link.
    pub link: Option<String>,

    // How far into the song, and how long it is, in milliseconds.
    pub position_ms: u64,
    pub length_ms: Option<u64>,
}

impl Track {
    pub fn into_activity(self, start_ms: u64) -> Activity {
        let mut timestamps = Timestamps::default();
        timestamps.start = Some(start_ms);

        if let Some(length) = self.length_ms {
            timestamps.end = Some(start_ms + length);
        }

        let mut assets = Assets::default();
        assets.large_image = self.artwork;
        assets.large_text = non_empty(&self.album);

        // Laid out the way Cider fills in Discord's fields, so the site reads
        // it the same: song on the first line, artist on the second, album on
        // the cover.
        Activity {
            kind: LISTENING,
            name: self.player,
            source: self.source,
            details: non_empty(&self.title),
            state: non_empty(&self.artist),
            application_id: None,
            url: self.link,
            timestamps: Some(timestamps),
            assets: Some(assets),
        }
    }

    // Two reports of the same song from the same player count as one song.
    fn key(&self) -> String {
        format!("{}\n{}\n{}\n{}", self.player, self.title, self.artist, self.album)
    }
}

// Works out when the song started, and keeps that answer steady.
//
// The start is "now minus how far in", but every reading of "how far in" is a
// few milliseconds off, so working it out fresh each time gives a slightly
// different answer every time. The Worker would see a change in every single
// report and push it to every open page. So the first answer is kept until
// the song changes, or the gap is big enough to be a real skip or a pause.
pub struct TrackClock {
    key: String,
    start_ms: u64,
}

// Anything closer than this is the same moment, measured twice.
const DRIFT_MS: u64 = 2000;

impl TrackClock {
    pub fn new() -> TrackClock {
        TrackClock {
            key: String::new(),
            start_ms: 0,
        }
    }

    pub fn start_of(&mut self, track: &Track) -> u64 {
        let measured = now_ms().saturating_sub(track.position_ms);
        let key = track.key();

        let same_song = key == self.key;
        let close_enough = measured.abs_diff(self.start_ms) < DRIFT_MS;

        if same_song && close_enough {
            return self.start_ms;
        }

        self.key = key;
        self.start_ms = measured;
        measured
    }
}
