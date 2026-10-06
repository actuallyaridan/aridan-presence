// presence.aridan.net
//
//   GET    /                 WebSocket for the site. Sends the presence on
//                            connect and again whenever it changes.
//   GET    /presence         The presence once, as JSON, for a page that has
//                            live updates turned off.
//   PUT    /devices/<id>     An agent's report: { "activities": [...] }.
//   DELETE /devices/<id>     An agent shutting down.
//
// The two device routes need "Authorization: Bearer <AGENT_TOKEN>". Reading
// is open to anyone, the same as Lanyard.

import { Presence } from "./presence.js";
import { cleanActivities } from "./activity.js";

export { Presence };

// A full report is a few KB at most. Anything far bigger is not an agent.
const MAX_BODY_BYTES = 32 * 1024;

// Short, so it can go in a log line or a URL without escaping:
// "work-laptop", "desktop", "macbook".
const DEVICE_ID = /^[a-z0-9-]{1,32}$/;

const DEVICE_PATH = /^\/devices\/([^/]+)$/;

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const presence = env.PRESENCE.get(env.PRESENCE.idFromName("aridan"));

    if (url.pathname === "/") {
      return presence.fetch(request);
    }

    if (url.pathname === "/presence") {
      if (request.method !== "GET") return status(405);

      const snapshot = await presence.snapshot();
      return json(snapshot);
    }

    const match = url.pathname.match(DEVICE_PATH);
    if (match) {
      return handleDevice(request, env, presence, match[1]);
    }

    return status(404);
  },
};

async function handleDevice(request, env, presence, deviceId) {
  if (!DEVICE_ID.test(deviceId)) return status(400, "Bad device id");

  const allowed = await authorised(request, env);
  if (!allowed) return status(401);

  if (request.method === "DELETE") {
    await presence.clear(deviceId);
    return status(204);
  }

  if (request.method !== "PUT") return status(405);

  const length = Number(request.headers.get("content-length") || 0);
  if (length > MAX_BODY_BYTES) return status(413);

  const text = await request.text();
  if (text.length > MAX_BODY_BYTES) return status(413);

  let body;
  try {
    body = JSON.parse(text);
  } catch {
    return status(400, "Body is not JSON");
  }

  const activities = cleanActivities(body?.activities);
  if (!activities) return status(400, "Expected { activities: [...] }");

  await presence.update(deviceId, activities);
  return status(204);
}

// Compared in constant time, so the response time says nothing about how
// much of a guessed token was right.
async function authorised(request, env) {
  if (!env.AGENT_TOKEN) return false;

  const encoder = new TextEncoder();
  const given = encoder.encode(request.headers.get("authorization") || "");
  const expected = encoder.encode("Bearer " + env.AGENT_TOKEN);

  if (given.byteLength !== expected.byteLength) return false;
  return crypto.subtle.timingSafeEqual(given, expected);
}

function json(body) {
  return new Response(JSON.stringify(body), {
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
      // Read by aridan.net from another origin.
      "access-control-allow-origin": "*",
    },
  });
}

function status(code, text = "") {
  if (code === 204) return new Response(null, { status: 204 });
  return new Response(text, { status: code });
}
