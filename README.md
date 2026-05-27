# lum

`lum` is a small opinionated CLI toolbox for setting up shell environment variables, installing a curated set of developer tools, listening to a few internet radio stations, checking local Git repositories, and managing folder-based Git identities.

The most useful commands for new users are:

- `lum env` — manage API-key-style environment variables and add lum's managed tool directory to your shell `PATH`.
- `lum tools` — install and update curated CLI tools like `ripgrep`, `fd`, `jq`, `yq`, `difftastic`, `shellcheck`, and others.

## Install

Download the latest binary for your platform from the GitHub Releases page, then put it somewhere on your `PATH`.

### What is `PATH`?

Your `PATH` is the list of folders your terminal searches when you type a command. If the `lum` binary is inside one of those folders, you can run it from anywhere by typing:

```sh
lum --help
```

### macOS / Linux

1. Download the Linux or macOS binary from Releases.
2. Rename it to `lum` if needed.
3. Move it into `~/.local/bin`:

```sh
mkdir -p ~/.local/bin
mv ~/Downloads/lum-* ~/.local/bin/lum
chmod +x ~/.local/bin/lum
```

4. Make sure `~/.local/bin` is on your `PATH`.

For bash, add this to `~/.bashrc`:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

For zsh, add this to `~/.zshrc`:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Then restart your terminal, or run:

```sh
source ~/.bashrc   # bash
# or
source ~/.zshrc    # zsh
```

Check that it works:

```sh
lum --help
```

### Windows PowerShell

1. Download the Windows binary from Releases.
2. Rename it to `lum.exe` if needed.
3. Create a tools folder, for example:

```powershell
New-Item -ItemType Directory -Force "$HOME\bin"
Move-Item "$HOME\Downloads\lum-windows-x86_64.exe" "$HOME\bin\lum.exe"
```

4. Add that folder to your user `PATH`:

```powershell
[Environment]::SetEnvironmentVariable(
  "Path",
  [Environment]::GetEnvironmentVariable("Path", "User") + ";$HOME\bin",
  "User"
)
```

Restart PowerShell, then check:

```powershell
lum --help
```

## Shell setup with `lum env`

`lum env init` prints shell code that:

- adds lum's managed binary directory to `PATH`
- exports environment variables you set with `lum env set`
- applies lum's built-in default environment variables

Run this once in your shell startup file.

### macOS / Linux

For bash, add this to `~/.bashrc`:

```sh
eval "$(lum env init --shell posix)"
```

For zsh, add this to `~/.zshrc`:

```sh
eval "$(lum env init --shell posix)"
```

Then restart your terminal.

### Windows PowerShell

Add this to your PowerShell profile:

```powershell
lum env init --shell powershell | Invoke-Expression
```

If you do not know where your profile is, run:

```powershell
$PROFILE
```

Create it if it does not exist:

```powershell
New-Item -ItemType File -Force $PROFILE
notepad $PROFILE
```

## Environment variables

`lum env` stores common secrets behind short aliases.

Available aliases:

| Alias | Environment variable |
| --- | --- |
| `exa` | `EXA_API_KEY` |
| `neuralwatt` | `NEURALWATT_API_KEY` |
| `openrouter` | `OPENROUTER_API_KEY` |
| `synthetic` | `SYNTHETIC_API_KEY` |

Examples:

```sh
lum env aliases
lum env set openrouter sk-or-...
lum env list
lum env unset openrouter
lum env path
```

After setting or unsetting a value, restart your terminal or re-run your shell init command so the environment updates in the current shell.

## Managed tools

`lum tools` installs curated developer tools into lum's managed binary directory. If you ran `lum env init` from your shell startup file, that directory is automatically on your `PATH`.

Available tools:

| Tool | Binary | Description |
| --- | --- | --- |
| `difftastic` | `difft` | Structural diff that understands syntax |
| `fd` | `fd` | Fast alternative to `find` |
| `jq` | `jq` | JSON processor |
| `ripgrep` | `rg` | Fast recursive text search |
| `scc` | `scc` | Code counter with complexity |
| `shellcheck` | `shellcheck` | Shell script static analyzer |
| `universal-ctags` | `ctags` | Source code indexer |
| `yq` | `yq` | YAML/JSON/XML/CSV processor |

Common commands:

```sh
lum tools ls
lum tools install ripgrep
lum tools status ripgrep
lum tools version ripgrep
lum tools update ripgrep
lum tools sync
lum tools sync --dry-run
```

## Other commands

### Radio

List stations:

```sh
lum radio
```

Play a station by code:

```sh
lum radio <station>
```

### Repositories

Scan a directory tree for Git repository status:

```sh
lum repos scan <directory>
```

By default this fetches each current branch's upstream remote before reporting ahead/behind status. Use `--offline` to compare against cached remote refs only.

Mirror configured repositories:

```sh
lum repos mirror init
lum repos mirror config-path
lum repos mirror sync
lum repos mirror status
```

### Git identities

Manage folder-based Git author/SSH identities:

```sh
lum git-id init
lum git-id config-path
lum git-id sync
lum git-id status
lum git-id where
lum git-id info <identity>
lum git-id pubkey <identity>
lum git-id paths
```

See `src/git_id/README.md` for config shape, ownership markers, generated files, and routing behavior.

## Build from source

You need Rust installed.

```sh
git clone https://github.com/ppowo/lum.git
cd lum
cargo build --release
```

For local development installs in this repository, use:

```sh
cargo local-install
```

## License

See [`LICENSE`](LICENSE).
