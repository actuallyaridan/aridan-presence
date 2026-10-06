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
