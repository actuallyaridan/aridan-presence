import { test } from "node:test";
import assert from "node:assert/strict";
import { cleanActivities } from "../src/activity.js";
import { combine, DEVICE_TTL_MS } from "../src/merge.js";

test("drops unknown fields, bad images and bad timestamps", () => {
  const [a] = cleanActivities([{
    type: 2, name: "Apple Music", source: "cider", details: "  Song  ", state: "x".repeat(300),
    evil: "<script>", url: "javascript:alert(1)",
    timestamps: { start: 2000, end: 1000 },
    assets: { large_image: "http://insecure/a.jpg", large_text: "Album", small_image: "https://ok/b.png" },
  }]);
  assert.deepEqual(Object.keys(a).sort(), ["assets", "details", "name", "source", "state", "timestamps", "type"]);
  assert.equal(a.details, "Song");
  assert.equal(a.state.length, 128);
  assert.deepEqual(a.timestamps, { start: 2000 });
  assert.deepEqual(a.assets, { large_text: "Album", small_image: "https://ok/b.png" });
});

test("rejects non-arrays and nameless activities", () => {
  assert.equal(cleanActivities("nope"), null);
  assert.deepEqual(cleanActivities([{ type: 0 }, null, 5]), []);
});

test("newest device first, one song only, stale devices dropped", () => {
  const now = 1_000_000;
  const song = (t) => ({ type: 2, name: "Apple Music", details: t });
  const devices = [
    { id: "pc", activities: [song("old"), { type: 0, name: "Game" }], seenAt: now, changedAt: now - 5000 },
    { id: "laptop", activities: [song("new")], seenAt: now, changedAt: now - 1000 },
    { id: "gone", activities: [song("stale")], seenAt: now - DEVICE_TTL_MS, changedAt: now },
  ];
  const result = combine(devices, now);
  assert.deepEqual(result.activities.map((a) => a.details || a.name), ["new", "Game"]);
  assert.equal(result.updated_at, now - 1000);
});
