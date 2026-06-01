# Yt Subcommand

`lum yt` downloads audio, video, or albums from YouTube using yt-dlp as a thin wrapper. It constructs the right arguments for each mode and shells out to yt-dlp directly.

## CLI Shape

```sh
lum yt aud <URL...>             # Audio → ~/Music
lum yt vid [--height N] <URL...> # Video → ~/Movies
lum yt alb <URL...>             # Album/playlist → ~/Music
```

## Dependencies

- **yt-dlp** — required. If not on `$PATH`, lum auto-provisions it to `data_dir()/deps/` (see ADR 0005).
- **ffmpeg** — required for video muxing. If not on `$PATH`, lum auto-provisions it on Linux/Windows from BtbN FFmpeg-Builds to `data_dir()/deps/`; macOS remains PATH-only.

## Architecture

- `mod.rs` — CLI dispatch, ffmpeg check, yt-dlp invocation with `-P` for output directory
- `args.rs` — argument construction for each subcommand (base flags, format selectors, output templates, metadata cleanup)
- `deps.rs` — yt-dlp binary resolution (`$PATH` → auto-provisioned → error)

## What We Dropped vs the Go Version

- **aria2c** — replaced by `--concurrent-fragments 8`. YouTube uses DASH fragments; aria2c's per-file split settings were ineffective.
- **Screen height detection** — replaced by a default 1080p ceiling with optional `--height` override.
- **`--tmp` flag** — removed. Use `--simulate` or run yt-dlp directly for testing.
- **`os.Chdir`** — replaced by yt-dlp's `-P` flag to set output path without process-wide side effects.
