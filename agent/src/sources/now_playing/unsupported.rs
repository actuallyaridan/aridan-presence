// Stands in on systems without a Now Playing reader yet.

use crate::config::Config;
use crate::state::PlayerSeen;
use crate::track::Track;

pub struct NowPlaying {
    pub seen: Vec<PlayerSeen>,
}

impl NowPlaying {
    pub async fn new(_config: &Config) -> NowPlaying {
        NowPlaying { seen: Vec::new() }
    }

    pub async fn current(&mut self) -> Option<Track> {
        None
    }
}
