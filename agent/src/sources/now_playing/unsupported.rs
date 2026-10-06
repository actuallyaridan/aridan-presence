// Stands in on systems without a Now Playing reader yet.

use crate::config::Config;
use crate::track::Track;

pub struct NowPlaying;

impl NowPlaying {
    pub async fn new(_config: &Config) -> NowPlaying {
        NowPlaying
    }

    pub async fn current(&mut self) -> Option<Track> {
        None
    }
}
