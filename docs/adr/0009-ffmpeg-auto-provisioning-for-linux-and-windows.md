# ffmpeg Auto-Provisioning for Linux and Windows

Lum resolves ffmpeg through the same broad dependency flow as yt-dlp: prefer a system binary on `$PATH`, fall back to a managed copy in `data_dir()/deps/`, then fail with install guidance.

The managed ffmpeg copy is supported only on Linux and Windows. macOS remains PATH-only because the evaluated public binary providers either do not publish Apple Silicon builds or publish outdated ffmpeg versions. Linux and Windows builds come from BtbN/FFmpeg-Builds using GPL, non-shared/static assets:

- `ffmpeg-master-latest-linux64-gpl.tar.xz`
- `ffmpeg-master-latest-linuxarm64-gpl.tar.xz`
- `ffmpeg-master-latest-win64-gpl.zip`
- `ffmpeg-master-latest-winarm64-gpl.zip`

BtbN publishes these under a rolling `latest` release tag, so lum does not compare versions. Instead, `data_dir()/deps/ffmpeg.json` records the last successful download timestamp. A managed copy older than 14 days is considered stale and is refreshed. If refresh fails and a cached binary exists, lum logs a warning and keeps using the cached copy.

For minimal parity with yt-dlp provisioning, downloads are trusted through HTTPS/GitHub release assets and do not currently verify BtbN's `checksums.sha256` file. An escape hatch, `LUM_FFMPEG_DISABLE_AUTO_PROVISION`, disables automatic provisioning and preserves PATH-only behavior. Tests can use `LUM_FFMPEG_TEST_ARTIFACT` to install a local fake executable without network access.
