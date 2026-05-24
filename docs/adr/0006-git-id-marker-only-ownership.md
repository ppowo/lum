# Git Identity Uses Marker-Only Ownership

Lum git-id treats the JSON config as the declarative source of truth and determines artifact ownership from embedded `lum:git-id` markers, not from a separate state file. Sync may create, update, or remove generated SSH keys, per-identity git configs, and managed global config sections only when those artifacts carry valid lum markers; this avoids stale state becoming authoritative while still allowing safe orphan cleanup.

## Considered Options

- Separate state file plus markers
- Embedded markers only

## Consequences

Renaming an identity is treated as removing the old identity and creating a new one, which means the user may need to upload a new public key to the hosting service. Lum does not track last sync times, historical identity values, or manually deleted generated artifacts; everything propagates from the JSON config and the current marker-bearing files on disk. Unmarked files at lum-owned paths are conflicts and must not be overwritten or deleted automatically.
