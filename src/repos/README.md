# Repos Subcommand

`lum repos` scans directory trees for Git repositories and manages curated mirror clones.

## CLI Shape

```
lum repos scan [--hidden] [-j N] [PATH]
lum repos mirror config-path
lum repos mirror dir
lum repos mirror init
lum repos mirror list
lum repos mirror sync [-j N]
lum repos mirror status [-j N] [--offline]
```

## Scanner (`repos scan`)

Walks a directory tree looking for `.git/` directories. For each repo found, runs `git status --porcelain=v1 --branch` and parses the output into:

- **Branch** (including detached HEAD)
- **Worktree status** (clean / has uncommitted changes)
- **Upstream sync** (synced / ahead / behind / diverged)

Output: `<path>  <branch>  <status>; <upstream-sync>`

| Flag | Purpose |
|------|---------|
| `[PATH]` | Scan root; defaults to `.` |
| `--hidden` | Descend into dot-prefixed directories |
| `-j N` | Max concurrent git operations (default 4, not yet implemented) |

Skips symlinks, `.git/` directories, and (by default) hidden directories. Reports nested repos.

## Mirror (`repos mirror`)

Maintains shallow (`--depth 1 --no-single-branch`) read-only clones under `~/Documents/CodeMirror/`. Configuration lives in lum's centralized config directory as `repos.json`.

### Config File

```json
{
  "repos": [
    {
      "url": "https://github.com/example-org/example-repo.git",
      "branch": "main",
      "tags": ["sample"]
    }
  ]
}
```

- `url` is required and must start with `https://`, `git@`, or `ssh://`.
- `branch` defaults to `"main"`.
- `tags` are optional metadata appended to directory names.

### Directory Naming

`<basename>-<branch>[-<tag1>-<tag2>-...]`

Where `basename` is the repo URL's last path segment with `.git` stripped, and `/` in branch names is replaced with `-`.

### Exit Codes for `mirror status`

| Code | Meaning |
|------|---------|
| 0 | All mirrors up to date |
| 1 | An error occurred |
| 2 | At least one mirror is behind |

### Sync Behavior

- New repos: `git clone --depth 1 --branch <branch> --no-single-branch --no-tags`
- Existing repos: `git fetch --depth 1 --all --no-tags && git checkout <branch> && git reset --hard origin/<branch>`
- Continues past individual errors; exits non-zero if any failed

### Safety

`validate_in_mirror` ensures all git operations target paths inside the CodeMirror directory (path-traversal guard).
