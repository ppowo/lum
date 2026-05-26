use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("dev", "ppowo", "lum").context("failed to determine platform directories")
}

fn xdg_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
}

fn xdg_data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
}

fn xdg_state_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_STATE_HOME").map(PathBuf::from)
}

pub(crate) fn config_file(filename: &str) -> Result<PathBuf> {
    let dir = match xdg_config_dir() {
        Some(dir) => dir.join("lum"),
        None => project_dirs()?.config_dir().to_path_buf(),
    };
    Ok(dir.join(filename))
}

pub(crate) fn data_dir(name: &str) -> Result<PathBuf> {
    let dir = match xdg_data_dir() {
        Some(dir) => dir.join("lum"),
        None => project_dirs()?.data_dir().to_path_buf(),
    };
    Ok(dir.join(name))
}

pub(crate) fn state_dir(name: &str) -> Result<PathBuf> {
    let dir = match xdg_state_dir() {
        Some(dir) => dir.join("lum"),
        None => {
            let dirs = project_dirs()?;
            dirs.state_dir()
                .unwrap_or_else(|| dirs.data_dir())
                .to_path_buf()
        }
    };
    Ok(dir.join(name))
}

pub(crate) fn bin_dir() -> Result<PathBuf> {
    data_dir("bin")
}

pub(crate) fn env_state_file() -> Result<PathBuf> {
    config_file("env-state.json")
}

pub(crate) fn tools_state_file() -> Result<PathBuf> {
    config_file("tools-state.json")
}

pub(crate) fn git_id_config_file() -> Result<PathBuf> {
    config_file("git-identities.json")
}

pub(crate) fn git_id_data_dir() -> Result<PathBuf> {
    data_dir("git-id")
}

pub(crate) fn repos_mirror_config_file() -> Result<PathBuf> {
    config_file("repos.json")
}

pub(crate) fn repos_mirror_watch_state_file() -> Result<PathBuf> {
    state_dir("repos-watch-state.json")
}

pub(crate) fn yt_deps_dir() -> Result<PathBuf> {
    data_dir("deps")
}

pub(crate) fn log_dir() -> Result<PathBuf> {
    state_dir("logs")
}

/// Resolve home directory or bail.
pub(crate) fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))
}

/// Join a relative path to the home directory.
pub(crate) fn home_path(relative: &str) -> Result<PathBuf> {
    Ok(home_dir()?.join(relative))
}

/// Expand a `~/` prefix to the home directory, if present.
pub(crate) fn expand_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

/// Canonicalize a path, falling back to a cleaned non-canonical version.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.components().collect())
}
