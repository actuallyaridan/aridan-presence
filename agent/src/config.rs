// The settings file.
//
// Lives at:
//   Linux    ~/.config/aridan-presence/config.toml
//   Windows  %APPDATA%\aridan-presence\config.toml
//   macOS    ~/Library/Application Support/aridan-presence/config.toml
//
// The first time the agent runs it writes a commented template there and
// stops, because it cannot do anything useful until the token is filled in.

use serde::Deserialize;
use std::path::PathBuf;

const TEMPLATE: &str = r#"# Settings for aridan-presence.

# Where to report to, and the token the Worker was given with
# `wrangler secret put AGENT_TOKEN`.
server = "https://presence.aridan.net"
token = ""

# This computer's name as the Worker sees it. Lowercase letters, digits and
# dashes. Left empty, it is made from the computer's hostname.
device = ""

# Cider's API token, from Cider > Settings > Connectivity. Left empty, Cider is
# still picked up through Now Playing, just without its Apple Music link.
cider_token = ""

# The iTunes storefronts to search for artwork, in order. Most songs are in
# every store, but not all: plenty of Balkan music is only in the Swedish one.
itunes_countries = ["se", "us"]

# Which players' Now Playing may be shared. Anything else playing - a video
# call, a training video in Edge - is ignored.
allowed_players = ["cider", "apple-music", "spotify", "firefox", "safari", "chrome"]
"#;

// `#[derive(Deserialize)]` lets the toml crate fill this struct in from the
// file. `#[serde(default)]` means a setting missing from the file takes its
// value from `Default` below instead of being an error.
#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: String,
    pub token: String,
    pub device: String,
    pub cider_token: String,
    pub itunes_countries: Vec<String>,
    pub allowed_players: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: String::from("https://presence.aridan.net"),
            token: String::new(),
            device: String::new(),
            cider_token: String::new(),
            itunes_countries: vec![String::from("se"), String::from("us")],
            allowed_players: vec![
                String::from("cider"),
                String::from("apple-music"),
                String::from("spotify"),
                String::from("firefox"),
                String::from("safari"),
                String::from("chrome"),
            ],
        }
    }
}

pub fn path() -> PathBuf {
    // dirs::config_dir() is the right folder for whichever OS this runs on.
    // It only comes back empty on a system with no home folder at all.
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("aridan-presence").join("config.toml")
}

// `Result<Config, String>` means: either a Config, or an error message
// explaining what is wrong. main() prints the message and stops.
pub fn load() -> Result<Config, String> {
    let path = path();

    if !path.exists() {
        write_template(&path)?;

        let message = format!(
            "Wrote a settings file to {}\nFill in the token there and start me again.",
            path.display()
        );
        return Err(message);
    }

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => return Err(format!("Could not read {}: {}", path.display(), error)),
    };

    let mut config: Config = match toml::from_str(&text) {
        Ok(config) => config,
        Err(error) => return Err(format!("{} has a mistake in it:\n{}", path.display(), error)),
    };

    if config.token.trim().is_empty() {
        return Err(format!("The token in {} is empty.", path.display()));
    }

    // A trailing slash would make every address below it "//devices/...".
    config.server = config.server.trim_end_matches('/').to_string();

    if config.device.is_empty() {
        config.device = device_from_hostname();
    }

    if !is_valid_device(&config.device) {
        return Err(format!(
            "device = \"{}\" in {} may only use lowercase letters, digits and dashes, up to 32 of them.",
            config.device,
            path.display()
        ));
    }

    // Matched against lowercase names later, so "Spotify" in the file works too.
    for player in config.allowed_players.iter_mut() {
        *player = player.to_lowercase();
    }

    Ok(config)
}

fn write_template(path: &PathBuf) -> Result<(), String> {
    if let Some(folder) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(folder) {
            return Err(format!("Could not create {}: {}", folder.display(), error));
        }
    }

    if let Err(error) = std::fs::write(path, TEMPLATE) {
        return Err(format!("Could not write {}: {}", path.display(), error));
    }

    Ok(())
}

// "Adnan's MacBook Pro" becomes "adnan-s-macbook-pro", which the Worker accepts.
fn device_from_hostname() -> String {
    let hostname = sysinfo::System::host_name().unwrap_or_default();

    let mut device = String::new();
    for character in hostname.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            device.push(character);
        } else if !device.ends_with('-') {
            device.push('-');
        }
    }

    let mut device = device.trim_matches('-').to_string();
    device.truncate(32);

    if device.is_empty() {
        device = String::from("computer");
    }

    device
}

fn is_valid_device(device: &str) -> bool {
    if device.is_empty() || device.len() > 32 {
        return false;
    }

    for character in device.chars() {
        let allowed = character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-';
        if !allowed {
            return false;
        }
    }

    true
}
