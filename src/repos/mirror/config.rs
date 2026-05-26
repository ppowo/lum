use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A configured mirror repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoEntry {
    /// Clone URL. Must start with https://, git@, or ssh://.
    pub url: String,
    /// Branch to track. Defaults to "main" if absent.
    #[serde(default = "default_branch")]
    pub branch: String,
    /// Optional tags appended to the directory name.
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_branch() -> String {
    "main".into()
}

impl RepoEntry {
    /// Derive the directory name: `<basename>-<branch>[-<tag1>-<tag2>-...]`.
    pub fn directory_name(&self) -> String {
        let base = repo_basename(&self.url);
        let branch = self.branch.replace('/', "-");
        let mut name = format!("{base}-{branch}");
        for tag in &self.tags {
            let sanitized = sanitize_tag(tag);
            if !sanitized.is_empty() {
                name.push('-');
                name.push_str(&sanitized);
            }
        }
        name
    }
}

/// Top-level config file shape.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReposConfig {
    pub repos: Vec<RepoEntry>,
}

/// Resolve the config file path via lum platform directories.
pub fn config_path() -> Result<PathBuf> {
    crate::paths::repos_mirror_config_file()
}

/// Load and validate the config file.
pub fn load(path: &std::path::Path) -> Result<Vec<RepoEntry>> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let config: ReposConfig =
        serde_json::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
    validate_repos(&config.repos)?;
    Ok(config.repos)
}

fn validate_repos(repos: &[RepoEntry]) -> Result<()> {
    for (i, repo) in repos.iter().enumerate() {
        validate_url(&repo.url, i + 1)?;
    }
    Ok(())
}

fn validate_url(url: &str, entry: usize) -> Result<()> {
    if url.starts_with("https://")
        || url.starts_with("git@")
        || url.starts_with("ssh://")
        || url.starts_with("file://")
    {
        Ok(())
    } else {
        anyhow::bail!(
            "config entry {}: url must start with https://, git@, or ssh:// (got: {})",
            entry,
            url
        )
    }
}

fn repo_basename(url: &str) -> String {
    // Extract last segment after / or :
    let base = url.rsplit(['/', ':']).next().unwrap_or(url);
    // Strip .git suffix
    let stripped = base.strip_suffix(".git").unwrap_or(base);
    stripped.to_string()
}

fn sanitize_tag(tag: &str) -> String {
    tag.chars()
        .map(|c| {
            if c == ' ' {
                '-'
            } else if c == '/' {
                '\0'
            } else {
                c
            }
        })
        .filter(|c| *c != '\0')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_name_basic() {
        let repo = RepoEntry {
            url: "https://github.com/org/my-repo.git".into(),
            branch: "main".into(),
            tags: vec![],
        };
        assert_eq!(repo.directory_name(), "my-repo-main");
    }

    #[test]
    fn directory_name_with_tags() {
        let repo = RepoEntry {
            url: "https://github.com/org/my-repo.git".into(),
            branch: "main".into(),
            tags: vec!["internal".into(), "v2".into()],
        };
        assert_eq!(repo.directory_name(), "my-repo-main-internal-v2");
    }

    #[test]
    fn directory_name_branch_with_slash() {
        let repo = RepoEntry {
            url: "https://github.com/org/my-repo.git".into(),
            branch: "feature/foo".into(),
            tags: vec![],
        };
        assert_eq!(repo.directory_name(), "my-repo-feature-foo");
    }

    #[test]
    fn url_without_git_suffix() {
        let repo = RepoEntry {
            url: "https://github.com/org/my-repo".into(),
            branch: "dev".into(),
            tags: vec![],
        };
        assert_eq!(repo.directory_name(), "my-repo-dev");
    }

    #[test]
    fn validate_url_accepts_https() {
        assert!(validate_url("https://github.com/org/repo.git", 1).is_ok());
    }

    #[test]
    fn validate_url_accepts_git_at() {
        assert!(validate_url("git@github.com:org/repo.git", 1).is_ok());
    }

    #[test]
    fn validate_url_accepts_ssh() {
        assert!(validate_url("ssh://git@github.com/org/repo.git", 1).is_ok());
    }

    #[test]
    fn validate_url_rejects_bare_host() {
        assert!(validate_url("github.com/org/repo.git", 1).is_err());
    }

    #[test]
    fn parse_config_with_defaults() {
        let json = r#"{"repos": [{"url": "https://github.com/org/repo.git"}]}"#;
        let config: ReposConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.repos[0].branch, "main");
        assert!(config.repos[0].tags.is_empty());
    }

    #[test]
    fn parse_config_rejects_invalid_url() {
        let entries = vec![RepoEntry {
            url: "github.com/org/repo.git".into(),
            branch: "main".into(),
            tags: vec![],
        }];
        assert!(validate_repos(&entries).is_err());
    }
}
