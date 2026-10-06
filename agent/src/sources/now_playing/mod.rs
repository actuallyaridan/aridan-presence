// Source 3: the system's Now Playing - whatever the OS shows in its media
// controls. Every OS does this differently, so each has its own file, and
// only the one for the OS being built is compiled in.
//
// What all of them share is here: which players are allowed, and what they
// are called on the site.

// `#[cfg(...)]` includes the next line only when building for that OS.
#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::NowPlaying;

// Until Windows and macOS have theirs, they get one that never finds anything.
#[cfg(not(target_os = "linux"))]
mod unsupported;

#[cfg(not(target_os = "linux"))]
pub use unsupported::NowPlaying;

// The name the site shows for a player, from the short name the settings file
// uses for it.
pub fn display_name(player: &str) -> String {
    let known = match player {
        "cider" => "Apple Music",
        "apple-music" => "Apple Music",
        "spotify" => "Spotify",
        "firefox" => "Firefox",
        "safari" => "Safari",
        "chrome" => "Chrome",
        _ => "",
    };

    if !known.is_empty() {
        return known.to_string();
    }

    // Something added to allowed_players that this list does not know:
    // "vlc" becomes "Vlc", which is at least readable.
    let mut characters = player.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}
