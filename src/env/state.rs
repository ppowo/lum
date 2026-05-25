use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Context, Result};

pub(crate) fn read_state() -> Result<BTreeMap<String, String>> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let data =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&data).with_context(|| format!("failed to parse {}", path.display()))
}

pub(crate) fn write_state(state: &BTreeMap<String, String>) -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let data = serde_json::to_string_pretty(state)?;
    fs::write(&path, data).with_context(|| format!("failed to write {}", path.display()))
}

pub(crate) fn bin_dir() -> Result<PathBuf> {
    crate::paths::bin_dir()
}

fn state_path() -> Result<PathBuf> {
    crate::paths::env_state_file()
}
