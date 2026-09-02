use std::{fs, io, path::Path, time::Duration};

use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

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

/// Download "url" to "dest". A url that names an existing local file is copied
/// instead of fetched — the network-free test hook shared by "tools" and "apps".
pub(crate) fn download_asset(url: &str, dest: &Path) -> Result<()> {
    let path = Path::new(url);
    if path.exists() {
        fs::copy(path, dest)?;
        return Ok(());
    }
    let mut response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(600))
        .user_agent("lum-tools/1.0")
        .build()?
        .get(url)
        .send()?
        .error_for_status()?;
    let mut out = fs::File::create(dest)?;
    io::copy(&mut response, &mut out)?;
    Ok(())
}

pub(crate) fn verify_sha256(path: &Path, expected: Option<&str>) -> Result<()> {
    let Some(expected) = expected.filter(|s| !s.trim().is_empty()) else {
        return Ok(());
    };
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = io::Read::read(&mut file, &mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if !actual.eq_ignore_ascii_case(expected.trim()) {
        anyhow::bail!("checksum mismatch for {}", path.display());
    }
    Ok(())
}
