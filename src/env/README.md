# Env Subcommand

`lum env` manages a small, hardcoded set of shell environment aliases and adds lum's bin directory to `PATH`.

## CLI Shape

```sh
eval "$(lum env init)"
lum env set openrouter sk-or-v1-xxxx
lum env unset openrouter
lum env list
lum env aliases
lum env path
```

`lum env init` emits shell code. On Unix shells, put `eval "$(lum env init)"` in `.bashrc`, `.zshrc`, or another POSIX-ish shell startup file. The emitted wrapper makes `lum env set` and `lum env unset` affect the current shell immediately without re-sourcing the startup file.

On Windows, PowerShell is the supported shell. Add this to `$PROFILE`:

```powershell
Invoke-Expression (& lum env init)
```

For testing or explicit selection, `lum env init`, `lum env set`, and `lum env unset` accept `--shell posix` or `--shell powershell`. The default is PowerShell on Windows and POSIX elsewhere.

## Storage

Use `directories::ProjectDirs::from("dev", "ppowo", "lum")`:

- state: `config_dir()/env-state.json`
- bin directory: `data_dir()/bin`

`lum env path` only prints the bin directory. `lum env init` creates it and prepends it to `PATH` with deduplication.

No vex state is read or migrated. If moving from vex, set values again manually with `lum env set`.

## Aliases

Aliases are hardcoded in source by design. Unknown aliases are rejected to avoid typo-created persistent environment variables.

Current aliases:

| Alias | Variable |
|-------|----------|
| exa | EXA_API_KEY |
| neuralwatt | NEURALWATT_API_KEY |
| openrouter | OPENROUTER_API_KEY |
| synthetic | SYNTHETIC_API_KEY |

## Shell Output

Commands intended for eval write shell statements to stdout and human messages to stderr.

POSIX exports use single-quote escaping, including embedded single quotes:

```sh
export OPENROUTER_API_KEY='abc'\''def'
```

PowerShell exports use PowerShell single-quote escaping:

```powershell
$env:OPENROUTER_API_KEY = 'abc''def'
```

## Forced Defaults

`lum env init` also emits these forced defaults. They are lum-managed, not user-configurable env-state entries:

| Variable | Value |
|----------|-------|
| PI_HASHLINE_GREP_MAX_LINES | 150 |
| PI_HASHLINE_GREP_MAX_BYTES | 10000 |
| PI_HASHLINE_BASH_CONTEXT_GUARD | 1 |
| PI_HASHLINE_BASH_CONTEXT_GUARD_MAX_LINES | 400 |
| PI_HASHLINE_BASH_CONTEXT_GUARD_MAX_BYTES | 25000 |
| PI_HASHLINE_BASH_CONTEXT_GUARD_HEAD_LINES | 60 |
| PI_HASHLINE_BASH_CONTEXT_GUARD_TAIL_LINES | 150 |
| npm_config_ignore_scripts | true |

## Listing

`lum env list` shows every variable lum manages: alias-backed variables plus forced defaults. Persisted alias values are masked by default because they are usually secrets.
