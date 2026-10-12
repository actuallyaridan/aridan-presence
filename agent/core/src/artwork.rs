// Finding a cover for a song whose player did not give us one, by searching
// iTunes - the same place the site looks things up.

use crate::activity::Activity;
use crate::track::Track;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

// Apple's CDN can resize any cover by changing this part of the address.
const SMALL_SIZE: &str = "100x100bb";
const WANTED_SIZE: &str = "600x600bb";

// The players whose tracks are looked up, by the name the site shows them
// under - Cider shows as "Apple Music". A browser plays videos and podcasts
// as often as songs, and iTunes would happily find a cover for whatever
// song has a similar name, so anything else goes without.
const MUSIC_PLAYERS: [&str; 2] = ["Apple Music", "Spotify"];

// A few hundred songs is days of listening. Past that it starts over rather
// than growing forever.
const CACHE_LIMIT: usize = 500;

// Only the fields we use. serde ignores the rest of iTunes' answer.
#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<SearchResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResult {
    artwork_url_100: Option<String>,
    track_view_url: Option<String>,
}

#[derive(Clone)]
struct Found {
    image: String,
    link: Option<String>,
}

pub struct Artwork {
    client: reqwest::Client,
    countries: Vec<String>,

    // Every song is looked up once. A song iTunes does not have is remembered
    // as None, so it is not searched for again every three seconds.
    cache: HashMap<String, Option<Found>>,
}

impl Artwork {
    pub fn new(countries: Vec<String>) -> Artwork {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(6))
            .build()
            .expect("the HTTP client should always build");

        Artwork {
            client: client,
            countries: countries,
            cache: HashMap::new(),
        }
    }

    // Fills in the cover, and the song's link while at it, if the track came
    // without them. A track that already has a cover is left alone, and so
    // is one that is not from a music player.
    pub async fn fill_in(&mut self, track: &mut Track) {
        if track.artwork.is_some() {
            return;
        }

        if !is_music_player(&track.player) {
            return;
        }

        // `if let Some(x) = ...` runs the block only when there is something
        // inside, and names it x.
        if let Some(found) = self.find(&track.title, &track.artist).await {
            track.artwork = Some(found.image);

            if track.link.is_none() {
                track.link = found.link;
            }
        }
    }

    // Cider tells Discord about some songs with its own animated cover, from
    // exp-rise.cider.sh. Discord passes that through its own image proxy, but
    // with Discord closed this agent is the one sending it, and as it is the
    // site gets a file of tens of megabytes it can neither shrink nor read
    // the colours of. So Apple's still of the same cover goes in large_image,
    // and Cider's moves to large_image_animated.
    //
    // A cover already on Apple's CDN is left as it is, and so is a song
    // iTunes cannot find, which keeps the animated one.
    //
    // Only Apple Music: Spotify's covers come from its own CDN, which is fine.
    pub async fn still_for(&mut self, activity: &mut Activity) {
        if activity.name != "Apple Music" {
            return;
        }

        let Some(assets) = activity.assets.as_ref() else {
            return;
        };

        let Some(image) = assets.large_image.as_ref() else {
            return;
        };

        if !needs_still(image) {
            return;
        }

        let title = activity.details.clone().unwrap_or_default();
        let artist = activity.state.clone().unwrap_or_default();

        let Some(found) = self.find(&title, &artist).await else {
            return;
        };

        // Checked above, so these are both there.
        let assets = activity.assets.as_mut().unwrap();
        assets.large_image_animated = assets.large_image.take();
        assets.large_image = Some(found.image);

        if activity.url.is_none() {
            activity.url = found.link;
        }
    }

    // The cached answer for a song, asking iTunes the first time. None both
    // when iTunes has nothing and when it could not be reached.
    async fn find(&mut self, title: &str, artist: &str) -> Option<Found> {
        if title.is_empty() {
            return None;
        }

        let term = format!("{} {}", title, artist);
        let key = term.to_lowercase();

        if !self.cache.contains_key(&key) {
            // A network error is not remembered, so the next tick tries again.
            let Ok(found) = self.search(&term).await else {
                return None;
            };

            if self.cache.len() >= CACHE_LIMIT {
                self.cache.clear();
            }
            self.cache.insert(key.clone(), found);
        }

        let cached = self.cache.get(&key)?;
        cached.clone()
    }

    // Ok(None) means iTunes answered and has nothing. Err means it did not
    // answer at all.
    async fn search(&self, term: &str) -> Result<Option<Found>, reqwest::Error> {
        for country in &self.countries {
            let query = [
                ("term", term),
                ("entity", "song"),
                ("limit", "1"),
                ("country", country.as_str()),
            ];

            let response: SearchResponse = self
                .client
                .get("https://itunes.apple.com/search")
                .query(&query)
                .send()
                .await?
                .json()
                .await?;

            // `?` above means: if this step failed, stop here and hand the
            // error back to whoever called search().

            let Some(first) = response.results.into_iter().next() else {
                continue;
            };

            let Some(small) = first.artwork_url_100 else {
                continue;
            };

            let found = Found {
                image: small.replace(SMALL_SIZE, WANTED_SIZE),
                link: first.track_view_url,
            };
            return Ok(Some(found));
        }

        Ok(None)
    }
}

fn is_music_player(player: &str) -> bool {
    MUSIC_PLAYERS.contains(&player)
}

// Apple's covers can be shrunk and read by the site; anything else from an
// Apple Music activity is Cider's own copy.
fn needs_still(image: &str) -> bool {
    !image.contains(".mzstatic.com/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_music_players_are_looked_up() {
        assert!(is_music_player("Apple Music"));
        assert!(is_music_player("Spotify"));

        assert!(!is_music_player("Firefox"));
        assert!(!is_music_player("Chrome"));
        assert!(!is_music_player("Microsoft Edge"));
    }

    #[test]
    fn only_cider_covers_are_swapped() {
        assert!(needs_still("https://exp-rise.cider.sh/api/v1/artwork/generate/abc.webp"));

        assert!(!needs_still(
            "https://is1-ssl.mzstatic.com/image/thumb/Music/v4/a/b/600x600bb.jpg"
        ));
    }
}
