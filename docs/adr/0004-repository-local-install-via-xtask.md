# Repository Local Install via xtask

Lum uses a repository-local Cargo alias, `cargo local-install`, that runs an `xtask` helper to build the release binary and install it into a user-owned bin directory. The installer logic lives in Rust so it works across Linux, macOS, and Windows without maintaining a POSIX shell installer or requiring an external task runner such as `cargo-make`. `cargo install` remains a standard Rust fallback, but the canonical local workflow for this repo is `cargo local-install`.
