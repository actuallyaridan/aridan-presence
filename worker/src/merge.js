// Several machines can report at once - the work laptop and the PC at home,
// say - and the site only has room for one picture. This turns every device's
// report into that one picture.

// How long a device counts as present after its last report. The agents
// report every 30 seconds whether or not anything changed, so this is three
// missed reports: long enough to ride out a flaky connection, short enough
// that a laptop shut mid-song stops showing it within a couple of minutes.
export const DEVICE_TTL_MS = 90_000;

const LISTENING = 2;

export function isLive(device, now) {
  return device.seenAt + DEVICE_TTL_MS > now;
}

// The device that changed most recently goes first, since that is the one
// in use. Only its music is kept: two songs at once is never what is
// actually happening, it is a machine left playing somewhere.
export function combine(devices, now) {
  const live = devices.filter((device) => isLive(device, now));
  live.sort((a, b) => b.changedAt - a.changedAt);

  const activities = [];
  let haveMusic = false;
  let updatedAt = 0;

  for (const device of live) {
    if (device.changedAt > updatedAt) updatedAt = device.changedAt;

    for (const activity of device.activities) {
      if (activity.type === LISTENING) {
        if (haveMusic) continue;
        haveMusic = true;
      }

      activities.push(activity);
    }
  }

  return { activities: activities, updated_at: updatedAt };
}

/* ---------- The last song played ---------- */

// With nothing playing, the site shows the last song instead of an empty
// space. This works out what to remember from the newest presence: the song
// while it plays, and once it stops, when that was. Null means the stored one
// is still right, so a heartbeat with the same song writes nothing.
export function nextLastPlayed(last, activities, now) {
  const music = activities.find((activity) => activity.type === LISTENING);

  if (music) {
    if (last && last.playing && sameActivity(last.activity, music)) return null;
    return { activity: music, playing: true, at: now };
  }

  if (!last) return null;
  if (!last.playing) return null;

  // A song that ran to its end stopped then, not when the device next
  // reported - which, for a laptop shut mid-queue, is up to DEVICE_TTL_MS
  // later. One stopped part way through has only "now" to go on.
  let stoppedAt = now;
  const end = last.activity.timestamps && last.activity.timestamps.end;
  if (end && end < now) {
    stoppedAt = end;
  }

  return { activity: last.activity, playing: false, at: stoppedAt };
}

// The presence as the site gets it: the live one, plus the last song while
// nothing is playing. Its timestamps go, since they describe a play that is
// over and the site would draw a progress bar from them.
export function withLastPlayed(presence, last) {
  if (!last) return presence;
  if (last.playing) return presence;

  const activity = { ...last.activity };
  delete activity.timestamps;

  return { ...presence, last_played: { activity: activity, at: last.at } };
}

function sameActivity(a, b) {
  return JSON.stringify(a) === JSON.stringify(b);
}

/* ---------- Recently played ---------- */

// How many songs the list keeps. Enough to see what an evening sounded like,
// few enough to stay one small storage value.
export const RECENT_MAX = 20;

// A song replaced sooner than this after it started was skipped, not
// listened to, so it comes off the list again. Last.fm draws its line at the
// same place.
export const SKIP_MS = 30_000;

// The list as it should be after the newest presence, newest first, or null
// if it has not changed. A song goes on the list when it starts. Coming back
// to the same song - a heartbeat, or pausing and playing again - is not a new
// entry, since nothing new was put on.
export function nextRecent(recent, activities, now) {
  const music = activities.find((activity) => activity.type === LISTENING);
  if (!music) return null;

  let list = recent || [];
  const newest = list[0];

  if (newest && sameSong(newest.activity, music)) return null;

  if (newest && now - newest.at < SKIP_MS) {
    list = list.slice(1);
  }

  const entry = { activity: withoutTimestamps(music), at: now };
  return [entry].concat(list).slice(0, RECENT_MAX);
}

// The list as the site gets it. The newest entry is marked while it is still
// the song playing, so the site can say so rather than "3 minutes ago".
export function recentForSite(recent, activities) {
  const list = recent || [];
  const music = activities.find((activity) => activity.type === LISTENING);

  const tracks = [];
  for (let i = 0; i < list.length; i++) {
    const track = { activity: list[i].activity, at: list[i].at };

    if (i === 0 && music && sameSong(list[i].activity, music)) {
      track.playing = true;
    }

    tracks.push(track);
  }

  return { tracks: tracks };
}

// The same song, whatever its timestamps or artwork say. Timestamps change
// every time it is paused and played again, and the artwork can come from a
// different host from one report to the next.
function sameSong(a, b) {
  if (a.details !== b.details) return false;
  if (a.state !== b.state) return false;

  const albumA = a.assets && a.assets.large_text;
  const albumB = b.assets && b.assets.large_text;
  return albumA === albumB;
}

function withoutTimestamps(activity) {
  const copy = { ...activity };
  delete copy.timestamps;
  return copy;
}
