# aridan-presence

What I'm listening to or playing, for the music widget on
[aridan.net](https://aridan.net), without needing Discord open.

- `worker/` is a Cloudflare Worker on `presence.aridan.net` that holds the
  current presence and pushes it to the site over a WebSocket.
- `agent/` runs in the background on each computer and reports to the
  Worker. It reads, in order: Cider, Discord rich presence from games and
  apps, and the system's Now Playing.

## Worker

```sh
cd worker
npm install
npx wrangler secret put AGENT_TOKEN   # once; anything long and random
npm run deploy
```

For local testing, put `AGENT_TOKEN=...` in `worker/.dev.vars` and run
`npm run dev`.

## Agent

Written in Rust. The first run writes a settings file and stops:

```sh
cd agent
cargo run
```

Fill in `token` in the file it names, then run it again. To have it start
with the desktop on Linux, see `agent/linux/aridan-presence.service`.
