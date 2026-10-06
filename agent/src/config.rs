// The settings, as the Settings tab shows them.
//
// Saved in:
//   Linux    ~/.config/aridan-presence/config.toml
//   Windows  %APPDATA%\aridan-presence\config.toml
//   macOS    ~/Library/Application Support/aridan-presence/config.toml
//
// It is a plain text file, so it can still be edited by hand, but the app
// writes it whenever Settings is saved, and anything else in it - comments
// included - is not kept.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const HEADER: &str = "# Settings for aridan-presence. Written by the app's Settings tab.\n\n";

// `Serialize` and `Deserialize` let this struct be turned into TOML for the
// file and JSON for the window, and back. `#[serde(default)]` means a setting
// missing from either takes its value from `Default` below.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    // Where to report to, and the token the Worker was given with
    // `wrangler secret put AGENT_TOKEN`.
    pub server: String,
    pub token: String,

    // This computer's name as the Worker sees it. Empty means one made from
    // the hostname.
    pub device: String,

    // Cider's API token, from Cider > Settings > Connectivity.
    pub cider_token: String,

    // The iTunes storefronts searched for artwork, in order.
    pub itunes_countries: Vec<String>,

    // Which players' Now Playing may be shared.
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

impl Config {
    // Nothing can be reported until there is a token.
    pub fn is_set_up(&self) -> bool {
        !self.token.trim().is_empty()
    }

    // The name actually used: the one chosen, or one made from the hostname.
    pub fn device_name(&self) -> String {
        if self.device.is_empty() {
            return device_from_hostname();
        }

        self.device.clone()
    }

    // Tidies up what the Settings tab sends before it is saved: stray spaces,
    // a trailing slash on the server, uppercase player names.
    fn tidy(&mut self) {
        self.server = self.server.trim().trim_end_matches('/').to_string();
        self.token = self.token.trim().to_string();
        self.device = self.device.trim().to_string();
        self.cider_token = self.cider_token.trim().to_string();

        let mut countries = Vec::new();
        for country in &self.itunes_countries {
            let country = country.trim().to_lowercase();
            if !country.is_empty() {
                countries.push(country);
            }
        }
        self.itunes_countries = countries;

        let mut players = Vec::new();
        for player in &self.allowed_players {
            let player = player.trim().to_lowercase();
            if !player.is_empty() && !players.contains(&player) {
                players.push(player);
            }
        }
        self.allowed_players = players;
    }

    fn check(&self) -> Result<(), String> {
        if !self.server.starts_with("https://") && !self.server.starts_with("http://") {
            return Err(String::from("The server has to start with https://"));
        }

        if !self.device.is_empty() && !is_valid_device(&self.device) {
            return Err(String::from(
                "The computer name may only use lowercase letters, digits and dashes, up to 32 of them.",
            ));
        }

        Ok(())
    }
}

pub fn path() -> PathBuf {
    // dirs::config_dir() is the right folder for whichever OS this runs on.
    // It only comes back empty on a system with no home folder at all.
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("aridan-presence").join("config.toml")
}

// No file yet is not an error: it is the first run, and the defaults are
// what the Settings tab starts from.
//
// `Result<Config, String>` means: either a Config, or a message explaining
// what is wrong with the file.
pub fn load() -> Result<Config, String> {
    let path = path();

    if !path.exists() {
        return Ok(Config::default());
    }

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => return Err(format!("Could not read {}: {}", path.display(), error)),
    };

    let mut config: Config = match toml::from_str(&text) {
        Ok(config) => config,
        Err(error) => return Err(format!("{} has a mistake in it:\n{}", path.display(), error)),
    };

    config.tidy();
    Ok(config)
}

// Checks, tidies and writes the settings. Returns them as saved, so the
// window can show exactly what ended up in the file.
pub fn save(mut config: Config) -> Result<Config, String> {
    config.tidy();
    config.check()?;

    let path = path();

    if let Some(folder) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(folder) {
            return Err(format!("Could not create {}: {}", folder.display(), error));
        }
    }

    let body = match toml::to_string_pretty(&config) {
        Ok(body) => body,
        Err(error) => return Err(format!("Could not write the settings: {}", error)),
    };

    let text = String::from(HEADER) + &body;

    if let Err(error) = std::fs::write(&path, text) {
        return Err(format!("Could not write {}: {}", path.display(), error));
    }

    // The token is in here, so only this user may read it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(config)
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
