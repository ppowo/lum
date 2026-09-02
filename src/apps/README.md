# Apps Subcommand

`lum apps` manages a hardcoded catalog of macOS applications installed into a per-user Applications folder. macOS only — every command refuses to run on other platforms.

## CLI

```sh
lum apps ls
lum apps install <app> [--force]
lum apps status <app>
lum apps update <app> [--force]
lum apps sync [--dry-run]
lum apps version <app>
```

`sync` is the all-apps operation: it installs missing apps, updates outdated apps, and skips up-to-date apps. `update` targets one named app only. There is no opt-in catalog behavior and no `--all` flag.

## Storage

- state: `config_dir()/apps-state.json` (XDG-aware, same policy as `tools-state.json`)
- install root: `$LUM_APPS_DIR` if set, else `~/Applications/lum` — created on demand by installs

Apps stay per-user, matching lum's per-user state and bin directories. No global `/Applications` writes and no admin rights required.

## Catalog

The managed app catalog contains only:

- `openemu` → `OpenEmu.app` (GitHub `OpenEmu/OpenEmu`, release asset `OpenEmu_<version>.zip`)

OpenEmu is an Intel-only build; on Apple Silicon lum prints the one-time Rosetta 2 install command after installing. lum only prints — it never shells out.

Apps distributed this way are unsigned, but files downloaded by lum never receive the `com.apple.quarantine` attribute, so they open without the right-click-Open dance. lum does not sign or notarize anything.

## Safety

- `install` errors when the app is already managed/installed unless `--force` is passed.
- An unmanaged bundle inside the install root is protected: `install`/`sync` never modify or delete it. `--force` takes it over and records ownership in `apps-state.json`.
- **Foreign copies**: before any install, update, or sync, lum scans `/Applications` and `~/Applications` (override with `$LUM_APPS_FOREIGN_SCAN_DIRS`) for an unmanaged bundle with the same name. If one exists, the command fails fast — lum never installs alongside or overwrites a foreign copy. Remove it first (for openemu: `brew uninstall --cask openemu`).
- `apps-state.json` is written only after the bundle is fully in place; a failed install leaves no managed state.

## Implementation

Pure Rust, mirroring `lum tools`: GitHub release lookup via the shared `crate::github` adapter, zip extraction with the `zip` crate, bundle discovery with `ignore`, recursive directory copy with `fs_extra`. Avoid shelling out to `curl`, `unzip`, `ditto`, or `softwareupdate`.

Integration tests live in `tests/apps_cli.rs` and run network-free via `LUM_APPS_TEST_ARTIFACT_<NAME>` (local zip fixture) plus the `$LUM_APPS_DIR` / `$LUM_APPS_FOREIGN_SCAN_DIRS` overrides.
