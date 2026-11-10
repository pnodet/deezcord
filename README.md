# deezcord

Terminal voice chat over WebRTC.

- `server/`: Bun + Hono WebSocket signaling server (`ws://localhost:3030/ws`) with a built-in STUN server (UDP 3479 for IPv4, 3478 for IPv6). Users and rooms live in SQLite.
- `client/`: Rust terminal client (webrtc-rs, cpal, opus). It connects to `ws://localhost:3030/ws`.
- `protocol-fixtures/`: one JSON file per WebSocket message. Both halves test against these files.

## Run

Server (needs [Bun](https://bun.sh)):

```sh
cd server
bun install
bun run dev
```

Environment variables: `DEEZCORD_DB` (SQLite file, default `deezcord.sqlite` in the working directory), `APP_PORT_UDP4` / `APP_PORT_UDP6` (STUN ports).

Client (needs Rust; on Linux also `libasound2-dev`, `libopus-dev` and `pkg-config`). Start the server first:

```sh
cd client
cargo run
```

## Checks

```sh
cd server && bun install && bun run typecheck && bun test
cd client && cargo check && cargo test
```

`cd server && bun run lint` runs ESLint. It still reports existing problems and is not part of the gate yet.

When you change a WebSocket message, update both type definitions (`server/src/commands/mod-*.ts` and `client/src/commands/mod.rs`), the matching file in `protocol-fixtures/`, and both tests (`server/src/commands/protocol.test.ts` and the `tests` module in `client/src/commands/mod.rs`).

## Running two clients on one machine

The client normally loads the user saved in its config file and asks for a name only on the first
launch. The file is `deezcord/config.json` in your data directory:
`~/Library/Application Support/deezcord/config.json` on macOS, `~/.local/share/deezcord/config.json`
on Linux.

Two clients on the same machine would share that user. Start any extra client with a throwaway user
that is never saved:

```sh
cd client && DEEZCORD_EPHEMERAL=1 cargo run
```

Only the value `1` turns this on.
