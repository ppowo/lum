# Radio Subcommand

`lum radio` reimplements `ruv` as a Rust subcommand.

## CLI Shape

- `lum radio` lists built-in stations.
- `lum radio <code>` plays a station.
- Preserve existing `ruv` station codes and the plain output style.

## Playback Stack

Use the pure Rust stack recorded in ADR 0002:

- `reqwest` with `rustls` for direct HTTPS audio streams.
- `symphonia` for decoding.
- `cpal` for cross-platform audio output.
- `ringbuf` for the decoder-to-audio callback bridge.
- `rubato` for resampling when streams are not 44.1 kHz.
- `crossterm` for terminal controls.

Do not add an `ffmpeg` dependency for v1.

## Supported Streams

Built-in stations must be direct audio streams that Symphonia can decode. The current implementation supports the existing ruv station set, including streams that require resampling and mono-to-stereo conversion.

Out of scope unless a real station requires it:

- playlists
- HLS
- HE-AAC
- user-configurable stations
- station aliases

## Runtime Semantics

Controls:

- `space` / `p`: pause or resume
- `q` / `ctrl+c`: stop

Pause is a live pause: disconnect/stop decoding while paused and reconnect to the live stream on resume. Do not buffer paused audio.

Keep terminal raw mode scoped carefully. Print normal multiline output before entering raw mode, and restore cooked mode before printing shutdown text.

## Tests

Run normal tests:

```sh
cargo test --workspace
```

Run the manual live-stream compatibility probe:

```sh
cargo test built_in_stations_decode_initial_packets -- --ignored --nocapture
```

The ignored probe opens each built-in station URL and decodes initial packets with Symphonia. It does not require audio hardware.
