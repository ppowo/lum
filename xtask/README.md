# xtask

`xtask` contains repository maintenance tasks that should not live in the main `lum` binary.

## Local Install

Canonical command:

```sh
cargo local-install
```

The repo-local Cargo alias runs:

```sh
cargo run --package xtask -- install
```

The install task:

1. builds `lum` with `cargo build --release`
2. selects a user-owned install directory from `PATH`
3. copies `target/release/lum` or `target/release/lum.exe` into that directory
4. sets executable permissions on Unix
5. prints the installed path

## Install Directory Policy

Without `PREFIX`, choose the first preferred directory that appears in `PATH`:

1. `$HOME/.bio/bin`
2. `$HOME/.local/bin`
3. `$HOME/bin`

If none are present in `PATH`, fall back to the first existing user directory among:

1. `$HOME/Desktop`
2. `$HOME/Downloads`
3. `$HOME`

Fallback installs warn because the installed command may not be directly runnable.

With `PREFIX`, install to `PREFIX/bin`. `PREFIX` must be absolute, inside the user's home directory, and `PREFIX/bin` must already appear in `PATH`.

## Rules

- Do not use sudo or system install directories.
- Do not replace Cargo's built-in `cargo install`.
- Keep this installer cross-platform in Rust rather than shell.
