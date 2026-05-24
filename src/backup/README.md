# Backup

`lum backup` backs up and restores two hardcoded personal data targets:

- `bio`: `~/.bio` on macOS and Linux
- `openemu`: `~/Library/Application Support/OpenEmu` on macOS

Running `lum backup <target>` creates a `.tar.gz` archive, uploads it to the hardcoded x0.at file host, downloads it back, and verifies that the uploaded file is a valid archive containing the target path.

Running `lum backup <target> <code>` downloads `https://x0.at/<code>.tar.gz`, verifies it, moves any existing target directory to a timestamped local backup, and extracts the archive into the home directory. Restore keeps the three newest local backups per target.

The command is intentionally not backward-compatible with the old zzk `.tar.xz` backup codes.
