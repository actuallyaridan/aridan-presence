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

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use self::windows::NowPlaying;

// Until macOS has its own, it gets one that never finds anything.
#[cfg(not(any(target_os = "linux", windows)))]
mod unsupported;

#[cfg(not(any(target_os = "linux", windows)))]
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
        "edge" => "Microsoft Edge",
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
