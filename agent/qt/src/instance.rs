// Only one copy at a time. Two would fight over discord-ipc-0 and report
// over each other.
//
// The first copy takes the name net.aridan.presence on D-Bus, the
// desktop's message bus, and answers "Show" there. A second copy finds the
// name taken, asks the first one to Show its window, and stops.

use crate::ffi;
use zbus::fdo::{RequestNameFlags, RequestNameReply};
use zbus::{connection, interface, Connection};

const NAME: &str = "net.aridan.presence";
const PATH: &str = "/net/aridan/presence";

struct Service;

#[interface(name = "net.aridan.presence")]
impl Service {
    // zbus offers this on D-Bus as "Show".
    fn show(&self) {
        ffi::post_show();
    }
}

pub enum Instance {
    // This is the only copy. The connection has to be kept for as long as
    // the app runs, or the name goes with it - it is held, never read.
    First(#[allow(dead_code)] Connection),

    // Another copy was already running, and has been asked to show itself.
    AlreadyRunning,

    // No D-Bus to ask - unusual outside a desktop session. Runs anyway.
    Unknown,
}

pub async fn claim() -> Instance {
    match try_claim().await {
        Ok(instance) => instance,
        Err(error) => {
            eprintln!("Could not check for another copy over D-Bus: {}", error);
            Instance::Unknown
        }
    }
}

async fn try_claim() -> zbus::Result<Instance> {
    let connection = connection::Builder::session()?.serve_at(PATH, Service)?.build().await?;

    let reply = connection
        .request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
        .await?;

    if reply == RequestNameReply::PrimaryOwner || reply == RequestNameReply::AlreadyOwner {
        return Ok(Instance::First(connection));
    }

    connection
        .call_method(Some(NAME), PATH, Some(NAME), "Show", &())
        .await?;

    Ok(Instance::AlreadyRunning)
}
