use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

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
    crate::paths::git_id_config_file()
}

pub fn data_dir() -> Result<PathBuf> {
    crate::paths::git_id_data_dir()
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
            let expanded = crate::paths::expand_path(folder);
            let normalized = crate::paths::normalize_path(&expanded);
            if !folders.insert(normalized) {
                anyhow::bail!("duplicate managed folder: {}", folder);
            }
        }
    }
    Ok(())
}

pub fn detect_identity<'a>(
    identities: &'a [Identity],
    dir: &std::path::Path,
) -> Option<&'a Identity> {
    let dir = crate::paths::normalize_path(dir);
    identities
        .iter()
        .filter_map(|identity| {
            identity
                .folders
                .iter()
                .map(|folder| crate::paths::normalize_path(&crate::paths::expand_path(folder)))
                .filter(|folder| is_path_prefix(folder, &dir))
                .map(|folder| (folder.components().count(), identity))
                .max_by_key(|(len, _)| *len)
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, identity)| identity)
}

fn is_path_prefix(prefix: &std::path::Path, path: &std::path::Path) -> bool {
    path == prefix || path.starts_with(prefix)
}

pub fn identity_private_key_path(identity: &Identity) -> Result<PathBuf> {
    Ok(crate::paths::home_dir()?
        .join(".ssh")
        .join(format!("lum-git-id-{}", identity.name)))
}

pub fn identity_public_key_path(identity: &Identity) -> Result<PathBuf> {
    Ok(identity_private_key_path(identity)?.with_extension("pub"))
}

pub fn identity_git_config_path(identity: &Identity) -> Result<PathBuf> {
    Ok(crate::paths::home_dir()?.join(format!(".gitconfig-lum-git-id-{}", identity.name)))
}

pub fn allowed_signers_path() -> Result<PathBuf> {
    crate::paths::home_path(".ssh/allowed_signers")
}

pub fn git_path(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
