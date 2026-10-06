// What an agent may report, cut down to what the site can use.
//
// Activities are kept in Lanyard's shape - name, details, state, timestamps,
// assets - because that is what every part of the site already reads. An
// agent passing on a game's Discord rich presence is passing on whatever that
// game chose to send, so nothing goes through unchecked: unknown fields are
// dropped, text is trimmed to Discord's own limits, and images must be https.

const MAX_ACTIVITIES = 8;

// Discord refuses longer than this in any rich presence field, so nothing
// real is ever cut short by it.
const MAX_TEXT = 128;

// Lanyard's numbering: 0 playing, 1 streaming, 2 listening, 3 watching,
// 4 custom status, 5 competing.
const MAX_TYPE = 5;

// Where an activity came from, so the site can prefer one over another.
const SOURCES = ["cider", "discord", "nowplaying"];

const SNOWFLAKE = /^\d{5,25}$/;

export function cleanActivities(input) {
  if (!Array.isArray(input)) return null;

  const activities = [];

  for (const raw of input.slice(0, MAX_ACTIVITIES)) {
    const activity = cleanActivity(raw);
    if (activity) activities.push(activity);
  }

  return activities;
}

function cleanActivity(raw) {
  if (!raw || typeof raw !== "object") return null;

  const name = text(raw.name);
  if (!name) return null;

  let type = Number(raw.type);
  if (!Number.isInteger(type) || type < 0 || type > MAX_TYPE) {
    type = 0;
  }

  const activity = { type: type, name: name };

  if (SOURCES.includes(raw.source)) activity.source = raw.source;

  const details = text(raw.details);
  if (details) activity.details = details;

  const state = text(raw.state);
  if (state) activity.state = state;

  if (typeof raw.application_id === "string" && SNOWFLAKE.test(raw.application_id)) {
    activity.application_id = raw.application_id;
  }

  // The track's own page, like an Apple Music song link.
  const url = httpsUrl(raw.url);
  if (url) activity.url = url;

  const timestamps = cleanTimestamps(raw.timestamps);
  if (timestamps) activity.timestamps = timestamps;

  const assets = cleanAssets(raw.assets);
  if (assets) activity.assets = assets;

  return activity;
}

// Milliseconds since the epoch, as Lanyard sends them.
function cleanTimestamps(raw) {
  if (!raw || typeof raw !== "object") return null;

  const timestamps = {};

  if (isTime(raw.start)) timestamps.start = raw.start;
  if (isTime(raw.end)) timestamps.end = raw.end;

  // An end before the start would put the progress bar past 100%.
  if (timestamps.start && timestamps.end && timestamps.end <= timestamps.start) {
    delete timestamps.end;
  }

  if (!timestamps.start && !timestamps.end) return null;
  return timestamps;
}

// Images are full https URLs. Lanyard's other forms - "mp:external/..." and
// an app's asset name - only mean anything to Discord, so the agents resolve
// those to a URL before sending.
function cleanAssets(raw) {
  if (!raw || typeof raw !== "object") return null;

  const assets = {};

  const largeImage = httpsUrl(raw.large_image);
  if (largeImage) assets.large_image = largeImage;

  const largeText = text(raw.large_text);
  if (largeText) assets.large_text = largeText;

  const smallImage = httpsUrl(raw.small_image);
  if (smallImage) assets.small_image = smallImage;

  const smallText = text(raw.small_text);
  if (smallText) assets.small_text = smallText;

  if (Object.keys(assets).length === 0) return null;
  return assets;
}

function text(value) {
  if (typeof value !== "string") return "";
  return value.trim().slice(0, MAX_TEXT);
}

function isTime(value) {
  if (!Number.isSafeInteger(value)) return false;
  return value > 0;
}

function httpsUrl(value) {
  if (typeof value !== "string") return "";
  if (value.length > 2048) return "";

  let url;
  try {
    url = new URL(value);
  } catch {
    return "";
  }

  if (url.protocol !== "https:") return "";
  return url.href;
}
