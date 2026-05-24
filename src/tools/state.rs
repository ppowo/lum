use std::{collections::BTreeMap, fs, path::PathBuf, time::SystemTime};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use super::catalog::{CATALOG, ToolSpec};
use super::platform::installed_filename;
use super::version::probe_version;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct ToolsState {
    #[serde(default)]
    pub version: String,
    pub tools: BTreeMap<String, ToolState>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ToolState {
    pub installed: bool,
    pub path: PathBuf,
    pub installed_version: String,
    pub installed_at: SystemTime,
    pub updated_at: SystemTime,
    pub artifact: ArtifactState,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ArtifactState {
    pub release_tag: String,
    pub asset_name: String,
    pub download_url: String,
    pub checksum: Option<String>,
}

pub(crate) struct LocalStatus {
    pub managed: bool,
    pub path: PathBuf,
    pub exists: bool,
    pub stored_version: Option<String>,
    pub runtime_version: Option<String>,
}

impl LocalStatus {
    pub fn effective_version(&self) -> Option<&str> {
        self.runtime_version
            .as_deref()
            .or(self.stored_version.as_deref())
    }

    pub fn installed_at(&self) -> Result<Option<SystemTime>> {
        Ok(load_state()?
            .tools
            .get(&self.spec_name()?)
            .map(|s| s.installed_at))
    }

    fn spec_name(&self) -> Result<String> {
        CATALOG
            .iter()
            .find(|s| tool_path(s).ok().as_ref() == Some(&self.path))
            .map(|s| s.name.to_owned())
            .context("failed to identify tool status")
    }
}

pub(crate) fn local_status(spec: &ToolSpec) -> Result<LocalStatus> {
    let stored = load_state()?;
    let path = stored
        .tools
        .get(spec.name)
        .map(|s| s.path.clone())
        .unwrap_or(tool_path(spec)?);
    let exists = path.exists();
    let managed = stored.tools.get(spec.name).is_some_and(|s| s.installed);
    let stored_version = stored
        .tools
        .get(spec.name)
        .map(|s| s.installed_version.clone());
    let runtime_version = if exists {
        probe_version(spec, &path).ok()
    } else {
        None
    };
    Ok(LocalStatus {
        managed,
        path,
        exists,
        stored_version,
        runtime_version,
    })
}

pub(crate) fn load_state() -> Result<ToolsState> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(ToolsState {
            version: "1.0".into(),
            tools: BTreeMap::new(),
        });
    }
    let data = fs::read_to_string(&path)?;
    let mut stored: ToolsState = serde_json::from_str(&data)?;
    if stored.version.is_empty() {
        stored.version = "1.0".into();
    }
    Ok(stored)
}

pub(crate) fn save_state(state: &ToolsState) -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

pub(crate) fn bin_dir() -> Result<PathBuf> {
    Ok(dirs()?.data_dir().join("bin"))
}

pub(crate) fn tool_path(spec: &ToolSpec) -> Result<PathBuf> {
    Ok(bin_dir()?.join(installed_filename(spec.binary)))
}

fn state_path() -> Result<PathBuf> {
    Ok(dirs()?.config_dir().join("tools-state.json"))
}

fn dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("dev", "ppowo", "lum").context("failed to determine platform directories")
}
