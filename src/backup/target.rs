use anyhow::{Result, bail};
use std::path::PathBuf;

pub(crate) const KEEP_BACKUPS: usize = 3;

pub(crate) const GLOBAL_EXCLUDE_GLOBS: &[&str] = &[
    "*.DS_Store",
    "._*",
    "Thumbs.db",
    "desktop.ini",
    "*.swp",
    "*.swo",
    "*~",
    ".Spotlight-V100",
    ".Trashes",
    ".fseventsd",
    ".TemporaryItems",
    ".git",
    ".svn",
];

#[derive(Clone, Copy, Debug)]
pub(crate) struct BackupTarget {
    pub(crate) name: &'static str,
    pub(crate) path: &'static str,
    pub(crate) allowed_os: &'static [&'static str],
    pub(crate) backup_prefix: &'static str,
    pub(crate) excludes: &'static [&'static str],
}

pub(crate) const BIO: BackupTarget = BackupTarget {
    name: "bio",
    path: ".bio",
    allowed_os: &["macos", "linux"],
    backup_prefix: ".bio.backup-",
    excludes: &[],
};

pub(crate) const OPENEMU: BackupTarget = BackupTarget {
    name: "openemu",
    path: "Library/Application Support/OpenEmu",
    allowed_os: &["macos"],
    backup_prefix: ".openemu.backup-",
    excludes: &[
        "Library/Application Support/OpenEmu/Cores",
        "Library/Application Support/OpenEmu/openvgdb.sqlite",
    ],
};

impl BackupTarget {
    pub(crate) fn ensure_current_os_allowed(self) -> Result<()> {
        let current = std::env::consts::OS;
        if self.allowed_os.contains(&current) {
            return Ok(());
        }

        bail!(
            "{} backup is only supported on {:?} (current OS: {})",
            self.name,
            self.allowed_os,
            current
        );
    }

    pub(crate) fn target_path(self, home: &std::path::Path) -> PathBuf {
        home.join(self.path)
    }
}
