use anyhow::{Context, Result, bail};

use super::catalog::ToolSpec;
use super::github::GitHubRelease;

#[derive(Debug)]
pub(crate) struct Artifact {
    pub version: String,
    pub release_tag: String,
    pub asset_name: String,
    pub download_url: String,
    pub archive_type: ArchiveType,
    pub binary_path: String,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchiveType {
    Binary,
    Zip,
    TarGz,
    TarXz,
}

pub(crate) fn artifact_for_release(spec: &ToolSpec, release: &GitHubRelease) -> Result<Artifact> {
    let tag = release.tag_name.as_deref().unwrap_or_default().trim();
    let version = release_version(spec, release)?;
    let (asset_name, binary_path) = asset_for_current_platform(spec, &version)?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .with_context(|| format!("release asset {asset_name} not found"))?;
    Ok(Artifact {
        version,
        release_tag: tag.to_owned(),
        asset_name: asset.name.clone(),
        download_url: asset.browser_download_url.clone(),
        archive_type: detect_archive_type(&asset.name),
        binary_path,
        checksum_sha256: None,
    })
}

fn release_version(spec: &ToolSpec, release: &GitHubRelease) -> Result<String> {
    let tag = release.tag_name.as_deref().unwrap_or_default().trim();
    let name = release.name.as_deref().unwrap_or_default().trim();
    let value = match spec.name {
        "universal-ctags" => tag
            .split('+')
            .next()
            .unwrap_or(tag)
            .trim_start_matches('v')
            .to_owned(),
        "scc" => tag.trim_start_matches('v').to_owned(),
        _ => bail!("unknown tool {}", spec.name),
    };
    if !value.is_empty() {
        Ok(value)
    } else if !name.is_empty() {
        Ok(name.trim_start_matches('v').to_owned())
    } else {
        bail!(
            "release metadata for {} is missing a version tag",
            spec.name
        )
    }
}

pub(crate) fn detect_archive_type(name: &str) -> ArchiveType {
    if name.ends_with(".tar.gz") {
        ArchiveType::TarGz
    } else if name.ends_with(".tar.xz") {
        ArchiveType::TarXz
    } else if name.ends_with(".zip") {
        ArchiveType::Zip
    } else {
        ArchiveType::Binary
    }
}

pub(crate) fn installed_filename(binary: &str) -> String {
    if cfg!(windows) && !binary.to_ascii_lowercase().ends_with(".exe") {
        format!("{binary}.exe")
    } else {
        binary.to_owned()
    }
}

fn asset_for_current_platform(spec: &ToolSpec, version: &str) -> Result<(String, String)> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let exe = installed_filename(spec.binary);
    let asset = match spec.name {
        "scc" => scc_asset(os, arch)?,
        "universal-ctags" => ctags_asset(version, os, arch)?,
        _ => bail!("unknown tool {}", spec.name),
    };
    let binary_path = match spec.name {
        "universal-ctags" => format!("bin/{exe}"),
        "scc" => exe,
        _ => bail!("unknown tool {}", spec.name),
    };
    Ok((asset, binary_path))
}

fn scc_asset(os: &str, arch: &str) -> Result<String> {
    let osn = match os {
        "macos" => "Darwin",
        "linux" => "Linux",
        "windows" => "Windows",
        _ => bail!("unsupported platform for scc: {os}/{arch}"),
    };
    let an = match arch {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        "x86" => "i386",
        _ => bail!("unsupported arch for scc: {os}/{arch}"),
    };
    Ok(format!(
        "scc_{osn}_{an}.{}",
        if os == "windows" { "zip" } else { "tar.gz" }
    ))
}

fn ctags_asset(version: &str, os: &str, arch: &str) -> Result<String> {
    let osn = match os {
        "macos" => "macos-10.15",
        "linux" => "linux",
        _ => bail!("unsupported platform for universal-ctags: {os}/{arch}"),
    };
    let an = match (os, arch) {
        ("macos", "aarch64") => "arm64",
        (_, "aarch64") => "aarch64",
        (_, "x86_64") => "x86_64",
        _ => bail!("unsupported arch for universal-ctags: {os}/{arch}"),
    };
    Ok(format!("uctags-{version}-{osn}-{an}.release.tar.xz"))
}
