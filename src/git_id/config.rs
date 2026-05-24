use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub author_name: String,
    pub email: String,
    pub domain: String,
    pub folders: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitIdentitiesConfig {
    pub identities: Vec<Identity>,
}

pub fn config_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "lum")
        .ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
    Ok(dirs.config_dir().join("git-identities.json"))
}

pub fn data_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "lum")
        .ok_or_else(|| anyhow::anyhow!("cannot determine data directory"))?;
    Ok(dirs.data_dir().join("git-id"))
}

pub fn load_config() -> Result<Vec<Identity>> {
    let path = config_path()?;
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let config: GitIdentitiesConfig =
        serde_json::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
    validate(&config.identities)?;
    Ok(config.identities)
}

fn validate(identities: &[Identity]) -> Result<()> {
    let mut names = HashSet::new();
    let mut folders = HashSet::new();
    let mut email_domains = HashSet::new();
    let mut author_domains = HashSet::new();

    for identity in identities {
        if identity.name.trim().is_empty() {
            anyhow::bail!("identity name must not be empty");
        }
        if !names.insert(identity.name.clone()) {
            anyhow::bail!("duplicate identity name: {}", identity.name);
        }
        if identity.author_name.trim().is_empty() {
            anyhow::bail!("identity {}: author_name must not be empty", identity.name);
        }
        if identity.email.trim().is_empty() {
            anyhow::bail!("identity {}: email must not be empty", identity.name);
        }
        if identity.domain.trim().is_empty() {
            anyhow::bail!("identity {}: domain must not be empty", identity.name);
        }
        if identity.folders.is_empty() {
            anyhow::bail!(
                "identity {}: at least one folder is required",
                identity.name
            );
        }
        let email_domain = (identity.email.clone(), identity.domain.clone());
        if !email_domains.insert(email_domain) {
            anyhow::bail!(
                "duplicate email+domain: {} on {}",
                identity.email,
                identity.domain
            );
        }
        let author_domain = (identity.author_name.clone(), identity.domain.clone());
        if !author_domains.insert(author_domain) {
            anyhow::bail!(
                "duplicate author_name+domain: {} on {}",
                identity.author_name,
                identity.domain
            );
        }
        for folder in &identity.folders {
            if folder.trim().is_empty() {
                anyhow::bail!("identity {}: folder must not be empty", identity.name);
            }
            let expanded = expand_path(folder);
            let normalized = normalize_path(&expanded);
            if !folders.insert(normalized) {
                anyhow::bail!("duplicate managed folder: {}", folder);
            }
        }
    }
    Ok(())
}

pub fn detect_identity<'a>(identities: &'a [Identity], dir: &Path) -> Option<&'a Identity> {
    let dir = normalize_path(dir);
    identities
        .iter()
        .filter_map(|identity| {
            identity
                .folders
                .iter()
                .map(|folder| normalize_path(&expand_path(folder)))
                .filter(|folder| is_path_prefix(folder, &dir))
                .map(|folder| (folder.components().count(), identity))
                .max_by_key(|(len, _)| *len)
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, identity)| identity)
}

fn is_path_prefix(prefix: &Path, path: &Path) -> bool {
    path == prefix || path.starts_with(prefix)
}

pub fn expand_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

pub fn normalize_path(path: &Path) -> PathBuf {
    path.components().collect()
}

pub fn identity_private_key_path(identity: &Identity) -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(".ssh")
        .join(format!("lum-git-id-{}", identity.name)))
}

pub fn identity_public_key_path(identity: &Identity) -> Result<PathBuf> {
    Ok(identity_private_key_path(identity)?.with_extension("pub"))
}

pub fn identity_git_config_path(identity: &Identity) -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(format!(".gitconfig-lum-git-id-{}", identity.name)))
}

pub fn allowed_signers_path() -> Result<PathBuf> {
    home_path(".ssh/allowed_signers")
}

pub fn home_path(relative: &str) -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(relative))
}

pub fn git_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
