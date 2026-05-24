use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct GitHubRelease {
    pub tag_name: Option<String>,
    pub name: Option<String>,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

pub(crate) fn latest_release(owner: &str, repo: &str) -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("lum-tools/1.0")
        .build()?;
    let text = client.get(&url).send()?.error_for_status()?.text()?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse GitHub release for {owner}/{repo}"))
}
