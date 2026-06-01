# YouTube Radio Uses yt-dlp and ffmpeg

YouTube-backed `lum radio` stations are treated as normal built-in radio stations at the CLI layer, but they use a separate media path from direct audio streams. Direct stations continue to use the pure Rust stack from ADR 0002: reqwest, Symphonia, CPAL, ringbuf, and rubato. YouTube station page URLs are first resolved with yt-dlp, reusing the existing yt-dlp auto-provisioning from ADR 0005. The resolved media URL may be an HLS playlist rather than a direct audio container; for this path, lum resolves ffmpeg through the shared dependency resolver and asks it to emit stereo `f32le` PCM at the active output sample rate, which is then written to the existing CPAL audio buffer.

ffmpeg was originally PATH-only for this path. ADR 0009 supersedes that packaging decision: ffmpeg remains PATH-preferred, but lum can auto-provision static BtbN builds on Linux and Windows. macOS remains PATH-only.
