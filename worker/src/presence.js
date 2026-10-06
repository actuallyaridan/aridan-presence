// The one place my presence lives. Agents update it, the site listens to it.
//
// There is exactly one of these (see index.js), so every report and every
// open page meets in the same object and a report reaches the pages the
// moment it lands.
//
// Pages connect with the hibernation API: the object can be evicted from
// memory while their sockets stay open, and is only billed while it is
// actually doing something. Between songs that is almost never.

import { DurableObject } from "cloudflare:workers";
import { combine, isLive, DEVICE_TTL_MS } from "./merge.js";

const DEVICE_PREFIX = "device:";

// What the pages were last sent. Changes are measured against this rather
// than against a snapshot taken a moment before, because a snapshot already
// leaves out devices that have timed out - so when the alarm comes to remove
// one, "before" would look just like "after" and the pages would never hear.
const SENT_KEY = "sent";

export class Presence extends DurableObject {
  constructor(ctx, env) {
    super(ctx, env);

    // Answered by Cloudflare without waking the object, so a page keeping its
    // socket alive costs nothing.
    ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }

  /* ---------- Pages ---------- */

  async fetch(request) {
    if (request.headers.get("upgrade") !== "websocket") {
      return new Response("Expected a WebSocket", { status: 426 });
    }

    const pair = new WebSocketPair();
    const client = pair[0];
    const server = pair[1];

    this.ctx.acceptWebSocket(server);

    // A page wants something to show straight away, not at the next change.
    const presence = await this.snapshot();
    server.send(message(presence));

    return new Response(null, { status: 101, webSocket: client });
  }

  // Pages only ever send "ping", which never gets here. Anything else is
  // ignored rather than answered.
  async webSocketMessage() {
  }

  async webSocketClose(ws, code, reason) {
    try {
      ws.close(code, reason);
    } catch {
      // Already closed from the other side.
    }
  }

  async snapshot() {
    const devices = await this.readDevices();
    return combine(devices, Date.now());
  }

  /* ---------- Agents ---------- */

  async update(deviceId, activities) {
    const now = Date.now();

    const key = DEVICE_PREFIX + deviceId;
    const previous = await this.ctx.storage.get(key);

    // A heartbeat with nothing new keeps the old changedAt, so it does not
    // push this device in front of one that is actually in use.
    let changedAt = now;
    if (previous && isLive(previous, now) && sameActivities(previous.activities, activities)) {
      changedAt = previous.changedAt;
    }

    await this.ctx.storage.put(key, {
      id: deviceId,
      activities: activities,
      seenAt: now,
      changedAt: changedAt,
    });

    await this.afterChange();
  }

  // An agent shutting down says so, rather than leaving its last song up
  // until it times out.
  async clear(deviceId) {
    await this.ctx.storage.delete(DEVICE_PREFIX + deviceId);
    await this.afterChange();
  }

  /* ---------- Devices going quiet ---------- */

  // Set for the moment the next device times out. A machine that sleeps or
  // loses its connection never says goodbye, so this is what notices.
  async alarm() {
    const now = Date.now();

    const devices = await this.readDevices();
    for (const device of devices) {
      if (!isLive(device, now)) {
        await this.ctx.storage.delete(DEVICE_PREFIX + device.id);
      }
    }

    await this.afterChange();
  }

  async afterChange() {
    const after = await this.snapshot();
    const sent = await this.ctx.storage.get(SENT_KEY);

    if (!sent || !sameActivities(sent, after.activities)) {
      await this.ctx.storage.put(SENT_KEY, after.activities);
      this.broadcast(after);
    }

    await this.scheduleExpiry();
  }

  async scheduleExpiry() {
    const devices = await this.readDevices();

    let next = 0;
    for (const device of devices) {
      const expires = device.seenAt + DEVICE_TTL_MS;
      if (!next || expires < next) next = expires;
    }

    if (next) {
      await this.ctx.storage.setAlarm(next);
    } else {
      await this.ctx.storage.deleteAlarm();
    }
  }

  /* ---------- Helpers ---------- */

  async readDevices() {
    const stored = await this.ctx.storage.list({ prefix: DEVICE_PREFIX });
    return Array.from(stored.values());
  }

  broadcast(presence) {
    const text = message(presence);

    for (const ws of this.ctx.getWebSockets()) {
      try {
        ws.send(text);
      } catch {
        // A socket that is going away; its close handler tidies it up.
      }
    }
  }
}

function message(presence) {
  return JSON.stringify({ op: "presence", d: presence });
}

function sameActivities(a, b) {
  return JSON.stringify(a) === JSON.stringify(b);
}
