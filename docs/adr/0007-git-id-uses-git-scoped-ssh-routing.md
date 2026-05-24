# Git Identity Uses Git-Scoped SSH Routing

Lum git-id routes folder-specific authentication through Git `includeIf` sections and per-identity `core.sshCommand` values instead of SSH `Match exec` directory checks. This keeps the feature portable and debuggable across Linux, macOS, and Windows while satisfying the main use case: Git operations inside managed folders use the correct SSH key.

## Considered Options

- SSH `Match exec` with a helper command for directory-aware SSH routing
- SSH host aliases plus URL rewriting
- Git `includeIf` plus per-identity `core.sshCommand`

## Consequences

Direct SSH commands such as `ssh -T git@github.com` are not folder-aware and use the default domain identity. Folder-specific identity selection applies to Git operations in managed folders.
