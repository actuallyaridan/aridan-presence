// Source 2: Discord rich presence, without Discord.
//
// Games and apps tell Discord what they are doing by connecting to a socket
// Discord opens on this computer, called discord-ipc-0, and sending it their
// activity. When Discord is not running, nobody is listening there - so we
// listen instead, answer the way Discord would, and keep what they tell us.
// The arRPC project does the same.
//
// When Discord is running, it gets the socket back: we stop listening, and
// the games reconnect to the real thing. The site still sees their activity
// then, through Lanyard.
//
// What arrives over the socket, in both directions, is a series of frames:
//
//   4 bytes   what kind of frame (an "opcode"), little-endian
//   4 bytes   how long the JSON after it is, little-endian
//   n bytes   the JSON
//
// A conversation goes:
//   app -> HANDSHAKE  { "v": 1, "client_id": "<the app's Discord id>" }
//   us  -> FRAME      READY, with a made-up user, which is all apps wait for
//   app -> FRAME      { "cmd": "SET_ACTIVITY", "args": { "activity": {...} } }
//   us  -> FRAME      the same activity back, meaning "done"
//   ...and so on, until the app quits and the connection closes.

use crate::activity::{now_ms, Activity, Assets, Timestamps, LISTENING, PLAYING};
use crate::state::Shared as AppShared;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::task::JoinSet;

const HANDSHAKE: u32 = 0;
const FRAME: u32 = 1;
const CLOSE: u32 = 2;
const PING: u32 = 3;
const PONG: u32 = 4;

// Real activities are well under 2 KB. Anything claiming to be bigger than
// this is not something we want to read into memory.
const MAX_FRAME_BYTES: u32 = 64 * 1024;

// How often to check whether Discord has been opened or closed.
const CHECK_INTERVAL: Duration = Duration::from_secs(10);

// Process names of Discord and the clients that stand in for it, lowercased
// and without Windows' ".exe". Each of them opens discord-ipc-0 itself.
const DISCORD_PROCESSES: [&str; 6] = [
    "discord",
    "discordcanary",
    "discordptb",
    "vesktop",
    "legcord",
    "armcord",
];

// What we know about an app from Discord's public API: its name, which the
// activity does not include, and the ids of its uploaded images, since apps
// refer to those by name.
#[derive(Clone)]
struct App {
    name: String,
    assets: HashMap<String, String>,
}

// Everything the connection tasks share. Arc lets several tasks hold the
// same thing; Mutex makes them take turns changing it.
#[derive(Clone)]
struct Shared {
    // One activity per connected app, by connection number. A BTreeMap keeps
    // them in the order the apps connected, so the list does not shuffle
    // around between reports.
    activities: Arc<Mutex<BTreeMap<u64, Activity>>>,
    apps: Arc<Mutex<HashMap<String, App>>>,
    client: reqwest::Client,

    // For the window: "starting", "listening", "discord-open" or
    // "unavailable".
    mode: Arc<Mutex<&'static str>>,

    // The app's own shared state, for the Log tab.
    app: Arc<AppShared>,
}

pub struct DiscordIpc {
    shared: Shared,
}

impl DiscordIpc {
    // Starts watching for Discord in the background and returns at once.
    pub fn start(app: Arc<AppShared>) -> DiscordIpc {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(6))
            .build()
            .expect("the HTTP client should always build");

        let shared = Shared {
            activities: Arc::new(Mutex::new(BTreeMap::new())),
            apps: Arc::new(Mutex::new(HashMap::new())),
            client: client,
            mode: Arc::new(Mutex::new("starting")),
            app: app,
        };

        tokio::spawn(supervise(shared.clone()));

        DiscordIpc { shared: shared }
    }

    pub fn activities(&self) -> Vec<Activity> {
        let activities = self.shared.activities.lock().unwrap();
        activities.values().cloned().collect()
    }

    pub fn mode(&self) -> &'static str {
        *self.shared.mode.lock().unwrap()
    }
}

/* ---------- Taking the socket, and giving it back ---------- */

async fn supervise(shared: Shared) {
    let mut system = System::new();

    // The task accepting connections while we hold the socket. Stopping it
    // closes every connection it opened too (see accept_loop).
    let mut serving: Option<tokio::task::JoinHandle<()>> = None;
    let mut reported_error = false;

    loop {
        let discord_open = discord_is_running(&mut system);

        if discord_open {
            *shared.mode.lock().unwrap() = "discord-open";

            if let Some(task) = serving.take() {
                shared.app.log("Discord is open: handing discord-ipc-0 back to it.");
                task.abort();
                platform::release();
                shared.activities.lock().unwrap().clear();
            }
        } else if serving.is_none() {
            match platform::listen() {
                Ok(listener) => {
                    shared.app.log("Discord is closed: listening on discord-ipc-0 for game activity.");
                    *shared.mode.lock().unwrap() = "listening";
                    serving = Some(tokio::spawn(accept_loop(listener, shared.clone())));
                    reported_error = false;
                }
                Err(error) => {
                    *shared.mode.lock().unwrap() = "unavailable";

                    if !reported_error {
                        shared.app.log(format!("Could not listen on discord-ipc-0: {}", error));
                        reported_error = true;
                    }
                }
            }
        }

        tokio::time::sleep(CHECK_INTERVAL).await;
    }
}

fn discord_is_running(system: &mut System) -> bool {
    // Only the process list itself is needed, not each one's CPU and memory.
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());

    for process in system.processes().values() {
        let name = process.name().to_string_lossy().to_lowercase();
        let name = name.trim_end_matches(".exe");

        if DISCORD_PROCESSES.contains(&name) {
            return true;
        }
    }

    false
}

async fn accept_loop(listener: platform::Listener, shared: Shared) {
    // A JoinSet stops every task in it when it is dropped, which happens when
    // supervise() aborts this task. That is what disconnects the apps when
    // Discord comes back.
    let mut connections = JoinSet::new();
    let mut next_id: u64 = 0;

    loop {
        // select! waits for whichever happens first.
        tokio::select! {
            accepted = platform::accept(&listener) => {
                match accepted {
                    Ok(stream) => {
                        next_id += 1;
                        connections.spawn(handle_connection(stream, next_id, shared.clone()));
                    }
                    Err(error) => {
                        shared.app.log(format!("discord-ipc-0: could not accept a connection: {}", error));
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }

            // Tidies up after connections that have finished.
            Some(_) = connections.join_next() => {}
        }
    }
}

/* ---------- One app's connection ---------- */

async fn handle_connection<S>(mut stream: S, id: u64, shared: Shared)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // Whatever way the conversation ends - the app quitting, a broken frame -
    // the app's activity goes with it, the same as with Discord.
    let _ = converse(&mut stream, id, &shared).await;
    shared.activities.lock().unwrap().remove(&id);
}

async fn converse<S>(stream: &mut S, id: u64, shared: &Shared) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (opcode, handshake) = read_frame(stream).await?;
    if opcode != HANDSHAKE {
        return Ok(());
    }

    let client_id = handshake["client_id"].as_str().unwrap_or_default().to_string();
    if client_id.is_empty() {
        return Ok(());
    }

    write_frame(stream, FRAME, &ready()).await?;

    loop {
        let (opcode, message) = read_frame(stream).await?;

        if opcode == PING {
            write_frame(stream, PONG, &message).await?;
        } else if opcode == FRAME {
            let reply = handle_command(&message, &client_id, id, shared).await;
            write_frame(stream, FRAME, &reply).await?;
        } else if opcode == CLOSE {
            return Ok(());
        } else {
            // Not something Discord ever receives. Hanging up is what Discord
            // would do too.
            return Ok(());
        }
    }
}

// What Discord sends once the handshake is done. Libraries only check that it
// arrived, so the user in it is a made-up one.
fn ready() -> Value {
    json!({
        "cmd": "DISPATCH",
        "evt": "READY",
        "nonce": null,
        "data": {
            "v": 1,
            "config": {
                "cdn_host": "cdn.discordapp.com",
                "api_endpoint": "//discord.com/api",
                "environment": "production"
            },
            "user": {
                "id": "1045800378228281345",
                "username": "aridan-presence",
                "discriminator": "0",
                "global_name": "aridan-presence",
                "avatar": null,
                "bot": false,
                "flags": 0,
                "premium_type": 0
            }
        }
    })
}

async fn handle_command(message: &Value, client_id: &str, id: u64, shared: &Shared) -> Value {
    let command = message["cmd"].as_str().unwrap_or_default();
    let nonce = message["nonce"].clone();

    // Apps also subscribe to events like join requests. Saying yes and then
    // never sending any is enough to keep them happy.
    if command != "SET_ACTIVITY" {
        return json!({ "cmd": command, "data": {}, "evt": null, "nonce": nonce });
    }

    let raw = &message["args"]["activity"];

    // A null activity is an app clearing its status.
    if raw.is_null() {
        shared.activities.lock().unwrap().remove(&id);
    } else if let Some(activity) = convert(raw, client_id, shared).await {
        shared.activities.lock().unwrap().insert(id, activity);
    }

    json!({ "cmd": command, "data": raw, "evt": null, "nonce": nonce })
}

/* ---------- Discord's activity to ours ---------- */

async fn convert(raw: &Value, client_id: &str, shared: &Shared) -> Option<Activity> {
    let app = look_up_app(client_id, shared).await;

    // The app's name from Discord, or failing that one the activity gives,
    // which newer apps are allowed to set.
    let mut name = raw["name"].as_str().unwrap_or_default().to_string();
    if let Some(app) = &app {
        if name.is_empty() {
            name = app.name.clone();
        }
    }
    if name.is_empty() {
        return None;
    }

    let mut kind = PLAYING;
    if let Some(number) = raw["type"].as_u64() {
        if number <= 5 {
            kind = number as u8;
        }
    }

    let mut timestamps = Timestamps::default();
    timestamps.start = time_ms(&raw["timestamps"]["start"]);
    timestamps.end = time_ms(&raw["timestamps"]["end"]);

    let mut assets = Assets::default();
    assets.large_image = image_url(&raw["assets"]["large_image"], client_id, &app);
    assets.large_text = string(&raw["assets"]["large_text"]);
    assets.small_image = image_url(&raw["assets"]["small_image"], client_id, &app);
    assets.small_text = string(&raw["assets"]["small_text"]);

    let mut activity = Activity {
        kind: kind,
        name: name,
        source: "discord",
        details: string(&raw["details"]),
        state: string(&raw["state"]),
        application_id: Some(client_id.to_string()),
        url: None,
        timestamps: None,
        assets: None,
    };

    if timestamps.start.is_some() || timestamps.end.is_some() {
        activity.timestamps = Some(timestamps);
    }

    if assets != Assets::default() {
        activity.assets = Some(assets);
    }

    // Cider fills in Discord the same way we fill in the site, so its
    // activity needs no special handling - but it does need to read as music.
    if activity.name == "Apple Music" {
        activity.kind = LISTENING;
    }

    Some(activity)
}

// Asked once per app, then remembered. Both addresses are public and need no
// login; they are what Discord itself uses to show an app's activity.
async fn look_up_app(client_id: &str, shared: &Shared) -> Option<App> {
    if let Some(app) = shared.apps.lock().unwrap().get(client_id) {
        return Some(app.clone());
    }

    let info_url = format!("https://discord.com/api/v9/applications/{}/rpc", client_id);
    let info: Value = shared.client.get(&info_url).send().await.ok()?.json().await.ok()?;
    let name = info["name"].as_str()?.to_string();

    let mut assets = HashMap::new();

    let assets_url = format!("https://discord.com/api/v9/oauth2/applications/{}/assets", client_id);
    if let Ok(response) = shared.client.get(&assets_url).send().await {
        if let Ok(Value::Array(list)) = response.json::<Value>().await {
            for asset in list {
                let asset_name = asset["name"].as_str().unwrap_or_default();
                let asset_id = asset["id"].as_str().unwrap_or_default();

                if !asset_name.is_empty() && !asset_id.is_empty() {
                    assets.insert(asset_name.to_lowercase(), asset_id.to_string());
                }
            }
        }
    }

    let app = App { name: name, assets: assets };
    shared.apps.lock().unwrap().insert(client_id.to_string(), app.clone());

    Some(app)
}

// Apps can name an image three ways:
//   "https://..."           a web address, used as it is
//   "mp:external/..."       an address Discord already proxies
//   "logo" or "12345..."    one of the images uploaded to the app
fn image_url(raw: &Value, client_id: &str, app: &Option<App>) -> Option<String> {
    let image = raw.as_str()?.trim();

    if image.is_empty() {
        return None;
    }

    if image.starts_with("https://") {
        return Some(image.to_string());
    }

    if let Some(path) = image.strip_prefix("mp:") {
        return Some(format!("https://media.discordapp.net/{}", path));
    }

    let mut asset_id = String::new();

    if image.chars().all(|character| character.is_ascii_digit()) {
        asset_id = image.to_string();
    } else if let Some(app) = app {
        if let Some(id) = app.assets.get(&image.to_lowercase()) {
            asset_id = id.clone();
        }
    }

    if asset_id.is_empty() {
        return None;
    }

    Some(format!("https://cdn.discordapp.com/app-assets/{}/{}.png", client_id, asset_id))
}

// Older apps send seconds, newer ones milliseconds. Anything before the year
// 2286 in milliseconds is a much bigger number than any date in seconds, so
// the size of the number says which it is.
fn time_ms(raw: &Value) -> Option<u64> {
    let number = raw.as_u64()?;

    if number == 0 {
        return None;
    }

    if number < 10_000_000_000 {
        return Some(number * 1000);
    }

    // A start time from the future is a broken clock in the game; the site
    // would show a negative "elapsed".
    if number > now_ms() + 24 * 60 * 60 * 1000 {
        return None;
    }

    Some(number)
}

fn string(raw: &Value) -> Option<String> {
    let text = raw.as_str()?.trim();

    if text.is_empty() {
        return None;
    }

    Some(text.to_string())
}

/* ---------- Frames ---------- */

async fn read_frame<S>(stream: &mut S) -> io::Result<(u32, Value)>
where
    S: AsyncRead + Unpin,
{
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).await?;

    let opcode = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);

    if length > MAX_FRAME_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too big"));
    }

    let mut body = vec![0u8; length as usize];
    stream.read_exact(&mut body).await?;

    // A frame that is not JSON is treated as empty rather than ending the
    // conversation. CLOSE frames, for one, often are not.
    let message = serde_json::from_slice(&body).unwrap_or(Value::Null);

    Ok((opcode, message))
}

async fn write_frame<S>(stream: &mut S, opcode: u32, message: &Value) -> io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(message).unwrap_or_default();

    let mut frame = Vec::with_capacity(8 + body.len());
    frame.extend_from_slice(&opcode.to_le_bytes());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(&body);

    stream.write_all(&frame).await
}

/* ---------- Where the socket is ---------- */

// On Linux and macOS it is a file in the user's runtime or temp folder. On
// Windows it is a named pipe. Both modules offer the same four things:
// Listener, listen(), accept() and release().
#[cfg(unix)]
mod platform {
    use std::io;
    use std::path::PathBuf;
    use tokio::net::{UnixListener, UnixStream};

    pub type Listener = UnixListener;

    // Discord and every library look in these, in this order, and use the
    // first one that is set.
    fn socket_path() -> PathBuf {
        let variables = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"];

        for variable in variables {
            if let Ok(folder) = std::env::var(variable) {
                if !folder.is_empty() {
                    return PathBuf::from(folder).join("discord-ipc-0");
                }
            }
        }

        PathBuf::from("/tmp").join("discord-ipc-0")
    }

    pub fn listen() -> io::Result<UnixListener> {
        let path = socket_path();

        // The socket file outlives whoever made it: Discord leaves one behind
        // when it quits, and so do we. If something answers on it, it is in
        // use and not ours to take. If nothing does, it is left over and is
        // removed so we can make our own.
        if path.exists() {
            if std::os::unix::net::UnixStream::connect(&path).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "something else is already listening on it",
                ));
            }

            std::fs::remove_file(&path)?;
        }

        UnixListener::bind(&path)
    }

    pub async fn accept(listener: &UnixListener) -> io::Result<UnixStream> {
        let (stream, _address) = listener.accept().await?;
        Ok(stream)
    }

    // So Discord finds the name free when it starts.
    pub fn release() {
        let _ = std::fs::remove_file(socket_path());
    }
}

#[cfg(windows)]
mod platform {
    use std::io;
    use std::sync::Mutex;
    use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};

    const PIPE: &str = r"\\.\pipe\discord-ipc-0";

    // What Windows answers when asked to open a pipe that does not exist.
    const ERROR_FILE_NOT_FOUND: i32 = 2;

    // A named pipe takes one connection per copy of it, so there is always a
    // fresh copy waiting for the next app. This holds it.
    pub struct Listener {
        waiting: Mutex<Option<NamedPipeServer>>,
    }

    pub fn listen() -> io::Result<Listener> {
        // Unlike a socket file, a pipe goes away with whoever made it, so
        // there is nothing left over to clear. If it can be opened, someone is
        // serving it; if Windows says there is no such pipe, it is free.
        match ClientOptions::new().open(PIPE) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "something else is already listening on it",
                ));
            }
            Err(error) => {
                if error.raw_os_error() != Some(ERROR_FILE_NOT_FOUND) {
                    // Busy, or not ours to open: in use either way.
                    return Err(error);
                }
            }
        }

        // first_pipe_instance makes this fail rather than share the name, in
        // case Discord made the pipe in the moment since the check above.
        let first = ServerOptions::new().first_pipe_instance(true).create(PIPE)?;

        Ok(Listener {
            waiting: Mutex::new(Some(first)),
        })
    }

    pub async fn accept(listener: &Listener) -> io::Result<NamedPipeServer> {
        // Taken out of the lock before waiting, since a lock cannot be held
        // across an await. If this wait is ever cancelled, the copy goes with
        // it, which is why an empty slot just gets a new one.
        let taken = listener.waiting.lock().unwrap().take();

        let server = match taken {
            Some(server) => server,
            None => ServerOptions::new().create(PIPE)?,
        };

        let connected = server.connect().await;

        // A fresh copy for the next app, whatever happened to this one.
        let next = ServerOptions::new().create(PIPE)?;
        *listener.waiting.lock().unwrap() = Some(next);

        connected?;
        Ok(server)
    }

    // Nothing to tidy: the pipe goes away when the last copy of it is closed.
    pub fn release() {}
}
