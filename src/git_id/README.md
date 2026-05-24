# Git ID Subcommand

`lum git-id` manages folder-based Git identities. The user edits one JSON config, then `lum git-id sync` converges generated SSH keys and Git/SSH configuration from that source of truth.

## CLI Shape

```sh
lum git-id config-path
lum git-id init
lum git-id sync
lum git-id status
lum git-id where
lum git-id info <identity>
lum git-id pubkey <identity>
lum git-id paths
```

## Config

The config path is resolved with `directories::ProjectDirs` and printed by `lum git-id config-path`.

```json
{
  "identities": [
    {
      "name": "github-work",
      "author_name": "Jane Doe",
      "email": "jane@company.com",
      "domain": "github.com",
      "folders": ["~/Work/Github"]
    }
  ]
}
```

- `name` is the stable identity ID. Renaming creates a new identity and orphans old marked artifacts.
- `author_name` is Git's `user.name`; it is not a hosting-service username.
- Duplicate identity names are rejected.
- Duplicate exact managed folders are rejected.
- Duplicate `email + domain` is rejected.
- Duplicate `author_name + domain` is rejected.
- Same email or author name across different domains is allowed.

## Language

**Git identity**: A named profile that represents one Git commit author and authentication setup for repositories under one or more managed folders.

**Author name**: The human-readable Git commit author name written to `user.name`. Avoid using `user` or `username` for this field.

**Managed folder**: A directory tree where repositories should automatically use a specific Git identity. When managed folders overlap, the most specific matching folder wins.

**Default domain identity**: The deterministic fallback identity used for a hosting domain when no managed folder context exists, such as direct SSH checks. It is not folder-aware.

## Sync Behavior

`sync` is declarative: the JSON config is the source of truth.

For each configured identity, sync:

1. Creates managed folders.
2. Generates a namespaced Ed25519 SSH key via `ssh-keygen` when missing.
3. Writes a per-identity Git config.
4. Updates lum-marked sections in global `~/.gitconfig`.
5. Updates lum-marked sections in `~/.ssh/config`.
6. Updates lum-marked sections in `~/.ssh/allowed_signers`.
7. Backs up and removes orphaned lum-marked artifacts.
8. Deletes backups older than 30 days.

## Ownership and Safety

Git ID uses marker-only ownership, not a state file. Lum may mutate or delete an artifact only when it carries a valid `lum:git-id` marker.

Markers:

```text
# lum:git-id:managed identity=github-work
[lum:git-id identity=github-work]
# lum:git-id:begin
# lum:git-id:end
```

Unmarked files at generated paths are conflicts and must not be overwritten or deleted automatically.

## Generated Paths

Generated files are namespaced with `lum-git-id-`:

```text
~/.ssh/lum-git-id-<identity>
~/.ssh/lum-git-id-<identity>.pub
~/.gitconfig-lum-git-id-<identity>
```

`lum git-id pubkey <identity>` prints only the public key to stdout for clipboard piping.

## Git and SSH Routing

Folder-specific routing is done with Git `includeIf` sections and per-identity `core.sshCommand`.

Per-identity Git configs include:

```gitconfig
[user]
  name = Jane Doe
  email = jane@company.com
  signingkey = /absolute/path/to/.ssh/lum-git-id-github-work.pub

[core]
  sshCommand = "ssh -i /absolute/path/to/.ssh/lum-git-id-github-work -o IdentitiesOnly=yes"

[commit]
  gpgsign = true

[gpg]
  format = ssh

[gpg "ssh"]
  allowedSignersFile = /absolute/path/to/.ssh/allowed_signers

[url "ssh://git@github.com/"]
  insteadOf = https://github.com/
```

The HTTPS-to-SSH rewrite is scoped to managed folders through the per-identity config. Lum does not rewrite repository remotes.

Direct SSH commands such as `ssh -T git@github.com` are not folder-aware; they use the default domain identity from the generated SSH config.
