use anyhow::Result;
use std::path::{Path, PathBuf};

use super::config::{Identity, data_dir, home_path};

pub fn read_optional(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

pub fn replace_marked_section(path: &Path, begin: &str, end: &str, section: &str) -> Result<()> {
    let existing = read_optional(path)?;
    let stripped = strip_marked_section(&existing, begin, end);
    std::fs::write(path, append_managed_section(&stripped, section))?;
    Ok(())
}

pub fn is_lum_managed_file(path: &Path, marker: &str) -> Result<bool> {
    Ok(read_optional(path)?.starts_with(marker))
}

fn strip_marked_section(content: &str, begin: &str, end: &str) -> String {
    let mut output = String::new();
    let mut in_section = false;
    for line in content.lines() {
        if line.trim() == begin {
            in_section = true;
            continue;
        }
        if line.trim() == end {
            in_section = false;
            continue;
        }
        if !in_section {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn append_managed_section(existing_without_section: &str, section: &str) -> String {
    let existing = existing_without_section.trim_end();
    if existing.is_empty() {
        section.to_string()
    } else {
        format!("{existing}\n\n{section}")
    }
}

pub fn cleanup_old_backups() -> Result<()> {
    let backup_dir = data_dir()?.join("backups");
    if !backup_dir.exists() {
        return Ok(());
    }
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 60 * 60);
    for entry in std::fs::read_dir(backup_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.modified().unwrap_or(std::time::SystemTime::now()) < cutoff {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(())
}

pub fn cleanup_orphan_identity_configs<F>(
    identities: &[Identity],
    identity_git_config_path: F,
) -> Result<()>
where
    F: Fn(&Identity) -> Result<PathBuf>,
{
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let active: std::collections::HashSet<_> = identities
        .iter()
        .map(|identity| identity.name.clone())
        .collect();
    for entry in std::fs::read_dir(&home)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(identity_name) = name.strip_prefix(".gitconfig-lum-git-id-") {
            if !active.contains(identity_name)
                && is_lum_managed_file(
                    &entry.path(),
                    &format!("# lum:git-id:managed identity={identity_name}"),
                )?
            {
                backup_and_remove(&[entry.path()])?;
            }
        }
    }
    let _ = identity_git_config_path;
    Ok(())
}

pub fn cleanup_orphan_key_pairs(identities: &[Identity]) -> Result<()> {
    let ssh_dir = home_path(".ssh")?;
    if !ssh_dir.exists() {
        return Ok(());
    }
    let active: std::collections::HashSet<_> = identities
        .iter()
        .map(|identity| identity.name.clone())
        .collect();
    for entry in std::fs::read_dir(&ssh_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(identity_name) = file_name
            .strip_prefix("lum-git-id-")
            .and_then(|name| name.strip_suffix(".pub"))
        else {
            continue;
        };
        if active.contains(identity_name) {
            continue;
        }
        let marker = format!("[lum:git-id identity={identity_name}]");
        let content = read_optional(&path)?;
        if !content.contains(&marker) {
            continue;
        }
        let private_key = ssh_dir.join(format!("lum-git-id-{identity_name}"));
        let mut paths = vec![path];
        if private_key.exists() {
            paths.push(private_key);
        }
        backup_and_remove(&paths)?;
    }
    Ok(())
}

fn backup_and_remove(paths: &[PathBuf]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let backup_dir = data_dir()?.join("backups");
    std::fs::create_dir_all(&backup_dir)?;
    let timestamp = chrono_like_timestamp();
    let backup_path = backup_dir.join(format!("git-id-orphans-{timestamp}.tar.gz"));
    let file = std::fs::File::create(&backup_path)?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    for path in paths {
        if path.exists() {
            tar.append_path_with_name(path, path.file_name().unwrap_or_default())?;
        }
    }
    tar.finish()?;
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

fn chrono_like_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    seconds.to_string()
}
