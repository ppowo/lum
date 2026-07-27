# Git Identity Uses a Declarative HTTP Credential Helper

Lum git-id stores optional HTTP Basic passwords or access tokens as plaintext in the existing `git-identities.json` source of truth and exposes them to Git through a read-only lum credential helper. Sync writes only the username, an exact protocol-and-host credential context, and an absolute invocation of lum into each folder-scoped identity config. It does not use an OS keyring, embed credentials in remote URLs, or generate a second Git credential-store file.

Plain HTTP is supported for internal hosts that cannot provide SSH or HTTPS, but its credentials are visible to network observers. Such identities must explicitly set `allow_insecure_http` to `true`. On Unix, lum creates or tightens the secret-bearing config to mode `0600`, and the helper refuses to return credentials when group or other permission bits are present.

## Considered Options

- Embed username and password in a Git URL rewrite
- Generate a plaintext `git-credential-store` file
- Read the declarative config through a lum credential helper
- Integrate with platform keyrings or Git Credential Manager

## Consequences

The JSON config and any user-managed backups contain the plaintext secret. Password-only edits take effect without sync because the helper reads the config for each request; changing routing fields or moving the lum executable requires another sync. HTTP identities still receive an SSH key for commit signing, but they do not receive SSH authentication routing. The hidden credential-helper execution path bypasses application tracing so protocol input and passwords cannot enter lum logs, which is a deliberate exception to application-wide logging in ADR 0003.
