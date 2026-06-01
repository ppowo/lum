# Radio Subcommand

`lum radio` reimplements `ruv` as a Rust subcommand.

## CLI Shape

- `lum radio` lists built-in stations.
- `lum radio <code>` plays a station.
- Preserve existing `ruv` station codes and the plain output style.

## Playback Stack

Direct audio stations use the pure Rust stack recorded in ADR 0002:

- `reqwest` with `rustls` for direct HTTPS audio streams.
- `symphonia` for decoding direct streams.
- `cpal` for cross-platform audio output.
- `ringbuf` for the decoder-to-audio callback bridge.
- `rubato` for resampling when streams are not 44.1 kHz.
- `crossterm` for terminal controls.

YouTube-backed live stations are resolved with yt-dlp and decoded with `ffmpeg` from `$PATH`. yt-dlp uses the existing auto-provisioning path shared with `lum yt`; ffmpeg is intentionally PATH-only for now and may be auto-provisioned later.

## Supported Streams

Built-in direct stations must be direct audio streams that Symphonia can decode. The existing ruv station set stays on this path, including streams that require resampling and mono-to-stereo conversion.

YouTube live stations are supported as normal built-in stations when yt-dlp can resolve the page URL and ffmpeg can decode the resulting stream or HLS playlist.

Out of scope unless a real station requires it:

- general playlist support outside the YouTube/ffmpeg path
- HE-AAC on the pure Rust direct-stream path
- user-configurable stations
- station aliases

## Blocking Decoder Adapter

`lum radio` runs inside the Tokio runtime for terminal control orchestration, but the stream decoder is intentionally blocking. Symphonia consumes `Read`/`Seek` media sources and CPAL invokes a real-time audio callback, so the current seam is:

- async command/control loop in `radio::mod`
- `tokio::task::spawn_blocking` for the Symphonia/HTTP decode loop
- a ring buffer between the blocking decoder and CPAL output callback

Do not move the decoder onto a normal Tokio task. If this area changes, preserve the explicit blocking adapter and focus on cancellation/backpressure behavior rather than making the entire audio path async.

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

Run the manual live-stream compatibility probes:

```sh
cargo test built_in_stations_decode_initial_packets -- --ignored --nocapture
cargo test youtube_station_decodes_initial_pcm_samples_via_ffmpeg -- --ignored --nocapture
```

The built-in direct-stream probe opens each direct station URL and decodes initial packets with Symphonia. The YouTube probe requires yt-dlp and ffmpeg on `$PATH` and verifies that the YouTube station can produce PCM samples through ffmpeg. Neither probe requires audio hardware.
