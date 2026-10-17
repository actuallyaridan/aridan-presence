import { test } from "node:test";
import assert from "node:assert/strict";
import { cleanActivities } from "../src/activity.js";
import {
  combine,
  nextLastPlayed,
  withLastPlayed,
  nextRecent,
  recentForSite,
  RECENT_MAX,
  SKIP_MS,
  DEVICE_TTL_MS,
} from "../src/merge.js";

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

test("keeps the animated cover beside the still", () => {
  const [a] = cleanActivities([{
    type: 2, name: "Apple Music", source: "discord",
    assets: {
      large_image: "https://is1-ssl.mzstatic.com/image/thumb/a/600x600bb.jpg",
      large_image_animated: "https://exp-rise.cider.sh/api/v1/artwork/generate/abc.webp",
    },
  }]);
  assert.equal(a.assets.large_image_animated, "https://exp-rise.cider.sh/api/v1/artwork/generate/abc.webp");

  const [b] = cleanActivities([{ type: 2, name: "x", assets: { large_image_animated: "http://insecure/a.webp" } }]);
  assert.equal(b.assets, undefined);
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

test("remembers the last song, and when it stopped", () => {
  const song = { type: 2, name: "Apple Music", details: "Song", timestamps: { start: 1000, end: 5000 } };
  const game = { type: 0, name: "Game" };

  const playing = nextLastPlayed(null, [game, song], 2000);
  assert.deepEqual(playing, { activity: song, playing: true, at: 2000 });

  // A heartbeat with the same song writes nothing.
  assert.equal(nextLastPlayed(playing, [song], 3000), null);

  // Ran to the end: stopped at the end, not when the device was next heard.
  const ended = nextLastPlayed(playing, [game], 9000);
  assert.deepEqual(ended, { activity: song, playing: false, at: 5000 });

  // Stopped part way through: only "now" to go on.
  const stopped = nextLastPlayed(playing, [], 3000);
  assert.equal(stopped.at, 3000);

  // Already stopped, or never played: nothing to change.
  assert.equal(nextLastPlayed(ended, [], 10000), null);
  assert.equal(nextLastPlayed(null, [game], 10000), null);
});

test("sends the last song only while nothing plays, without its timestamps", () => {
  const song = { type: 2, name: "Apple Music", details: "Song", timestamps: { start: 1000, end: 5000 } };
  const presence = { activities: [], updated_at: 0 };

  assert.equal(withLastPlayed(presence, null), presence);
  assert.equal(withLastPlayed(presence, { activity: song, playing: true, at: 1 }), presence);

  const result = withLastPlayed(presence, { activity: song, playing: false, at: 5000 });
  assert.deepEqual(result.last_played, { activity: { type: 2, name: "Apple Music", details: "Song" }, at: 5000 });
  assert.equal(song.timestamps.end, 5000);
});

test("adds each new song once, without its timestamps", () => {
  const song = (t, start) => ({ type: 2, name: "Apple Music", details: t, state: "Artist", timestamps: { start: start } });
  const game = { type: 0, name: "Game" };

  const first = nextRecent(null, [game, song("A", 1)], 0);
  assert.deepEqual(first, [{ activity: { type: 2, name: "Apple Music", details: "A", state: "Artist" }, at: 0 }]);

  // Heartbeats, and pausing then playing again, are the same song.
  assert.equal(nextRecent(first, [song("A", 1)], 60_000), null);
  assert.equal(nextRecent(first, [song("A", 99)], 120_000), null);

  // Nothing playing changes nothing.
  assert.equal(nextRecent(first, [game], 120_000), null);

  const second = nextRecent(first, [song("B", 2)], 200_000);
  assert.deepEqual(second.map((t) => t.activity.details), ["B", "A"]);
});

test("drops a skipped song, and keeps the list short", () => {
  const song = (t) => ({ type: 2, name: "Apple Music", details: t });

  const played = nextRecent(null, [song("A")], 0);
  const skipped = nextRecent(played, [song("B")], 100_000);
  const next = nextRecent(skipped, [song("C")], 100_000 + SKIP_MS - 1);
  assert.deepEqual(next.map((t) => t.activity.details), ["C", "A"]);

  let list = null;
  for (let i = 0; i < RECENT_MAX + 5; i++) {
    list = nextRecent(list, [song(String(i))], i * SKIP_MS);
  }
  assert.equal(list.length, RECENT_MAX);
  assert.equal(list[0].activity.details, String(RECENT_MAX + 4));
});

test("marks the newest song while it is still playing", () => {
  const a = { type: 2, name: "Apple Music", details: "A" };
  const b = { type: 2, name: "Apple Music", details: "B" };
  const list = [{ activity: a, at: 2 }, { activity: b, at: 1 }];

  const playing = recentForSite(list, [a]);
  assert.equal(playing.tracks[0].playing, true);
  assert.equal(playing.tracks[1].playing, undefined);

  assert.equal(recentForSite(list, []).tracks[0].playing, undefined);
  assert.deepEqual(recentForSite(null, []), { tracks: [] });
});
