# Radio Subcommand

`lum radio` reimplements `ruv` as a Rust subcommand while delegating playback to `ffplay`.

## CLI Shape

- `lum radio` lists built-in stations.
- `lum radio list` explicitly lists built-in stations.
- `lum radio <code>` starts a station in the background, replacing any current station.
- `lum radio status` prints the remembered playback state.
- `lum radio stop` stops playback.
- `lum radio pause` stops the live stream and remembers the station as paused.
- `lum radio resume` reconnects to the paused station.
- Preserve existing `ruv` station codes and the plain output style.

## Playback Stack

Radio playback uses the existing ffmpeg dependency path:

- `ffplay` does audio playback for direct streams.
- `yt-dlp` resolves YouTube live station pages to stream URLs.
- `ffplay` then plays the resolved YouTube stream URL.

`ffplay` is preferred from `$PATH`. If it is not on `$PATH`, lum looks for a provisioned `ffplay` next to its managed ffmpeg binary.

The old pure-Rust foreground audio path is no longer the product direction for `lum radio`. Do not reintroduce CPAL/Symphonia/ring-buffer terminal playback unless an ADR reverses this decision.

## Supported Streams

Built-in direct stations are passed to `ffplay` as URLs. YouTube live stations are supported when yt-dlp can resolve the page URL and ffplay can play the resulting stream or HLS playlist.

Out of scope unless a real station requires it:

- user-configurable stations
- station aliases
- custom audio decoding inside lum

## Runtime Semantics

Controls are process-backed, not terminal-key backed:

- `pause` is a live pause: stop `ffplay` and keep station state as paused. No audio is buffered.
- `resume` starts a new `ffplay` process for the remembered station.
- `stop` kills the remembered `ffplay` process only if the current process still matches ffplay identity/start-time state, then clears state.
- Starting a new station stops the remembered process and starts the new station.

State is stored in lum's platform-native state directory as `radio-player.json`, including the remembered ffplay PID and process start time when available.

User-facing output remains plain/script-friendly:

- `playing <code> <description>`
- `paused <code> <description>`
- `stopped`

## Tests

Run normal tests:

```sh
cargo test --workspace
```

Manual live-stream testing requires `ffplay` and, for YouTube stations, `yt-dlp`:

```sh
cargo run -- radio atma
cargo run -- radio status
cargo run -- radio stop
```
