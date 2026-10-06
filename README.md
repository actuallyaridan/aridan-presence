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

A tray app, written in Rust with [Tauri](https://tauri.app). The window is
plain HTML, CSS and JavaScript in `agent/ui/`, styled with aridan.net's own
CSS. The first time it opens, it shows Settings: put the Worker's token in
Connections > Server, and switch on General > Start when I log in.

### Linux

```sh
cd agent
npm install
npx tauri build --no-bundle
./linux/install.sh
```

### Windows

Every push that changes `agent/` builds the installers on GitHub: open the
run under Actions and download `aridan-presence-windows` from its Artifacts.
The `.exe` inside installs for the current user without admin rights.

Windows will warn that the installer is from an unknown publisher, since it
is not signed: More info > Run anyway.

### While working on it

```sh
cd agent
npx tauri dev
```
