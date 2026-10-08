// What gets sent to the Worker.
//
// An Activity is shaped like the ones Lanyard sends, because that is what the
// site already knows how to show. The Worker's activity.js checks the same
// fields on its side.

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

// Lanyard's numbering for the "type" field.
pub const PLAYING: u8 = 0;
pub const LISTENING: u8 = 2;

// `#[derive(...)]` asks Rust to write some code for us:
//   Serialize  - turning it into JSON with serde_json
//   Clone      - making a copy with .clone()
//   PartialEq  - comparing two with ==, to tell whether anything changed
//   Debug      - printing it with {:?} while testing
//
// `Option<String>` is a String that may be missing. A missing one is left out
// of the JSON entirely, which is what `skip_serializing_if` is for.
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Activity {
    // "type" is a reserved word in Rust, so the field is called kind here and
    // renamed back when it becomes JSON.
    #[serde(rename = "type")]
    pub kind: u8,

    pub name: String,

    // "cider", "discord" or "nowplaying".
    pub source: &'static str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<Timestamps>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Assets>,
}

// Milliseconds since 1 January 1970, the way JavaScript's Date.now() counts.
#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub struct Timestamps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<u64>,
}

#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub struct Assets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_image: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

pub fn now_ms() -> u64 {
    let since_1970 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    since_1970.as_millis() as u64
}

// An empty string becomes None, so it is left out rather than sent as "".
pub fn non_empty(text: &str) -> Option<String> {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}
