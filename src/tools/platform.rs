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
    let (asset_name, binary_path) = asset_for_current_platform(spec, tag, &version)?;
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
        "jq" => tag
            .strip_prefix("jq-")
            .unwrap_or(tag)
            .trim_start_matches('v')
            .to_owned(),
        "universal-ctags" => tag
            .split('+')
            .next()
            .unwrap_or(tag)
            .trim_start_matches('v')
            .to_owned(),
        _ => tag.trim_start_matches('v').to_owned(),
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

fn asset_for_current_platform(
    spec: &ToolSpec,
    tag: &str,
    version: &str,
) -> Result<(String, String)> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let exe = installed_filename(spec.binary);
    let asset = match spec.name {
        "ripgrep" => format!("ripgrep-{tag}-{}.tar.gz", target_triple(os, arch, true)?),
        "fd" => format!("fd-{tag}-{}.tar.gz", fd_triple(os, arch)?),
        "jq" => jq_asset(os, arch)?,
        "yq" => yq_asset(os, arch)?,
        "difftastic" => difft_asset(os, arch)?,
        "scc" => scc_asset(os, arch)?,
        "shellcheck" => shellcheck_asset(tag, os, arch)?,
        "universal-ctags" => ctags_asset(version, os, arch)?,
        _ => bail!("unknown tool {}", spec.name),
    };
    let binary_path = match spec.name {
        "ripgrep" => format!("ripgrep-{tag}-{}/{}", target_triple(os, arch, true)?, exe),
        "fd" => format!("fd-{tag}-{}/{}", fd_triple(os, arch)?, exe),
        "shellcheck" => format!("{tag}/{exe}"),
        "universal-ctags" => format!("bin/{exe}"),
        _ => exe,
    };
    Ok((asset, binary_path))
}

fn target_triple(os: &str, arch: &str, musl_linux: bool) -> Result<&'static str> {
    match (os, arch, musl_linux) {
        ("macos", "aarch64", _) => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64", _) => Ok("x86_64-apple-darwin"),
        ("linux", "aarch64", true) => Ok("aarch64-unknown-linux-musl"),
        ("linux", "x86_64", true) => Ok("x86_64-unknown-linux-musl"),
        ("linux", "x86_64", false) => Ok("x86_64-unknown-linux-gnu"),
        ("windows", "aarch64", _) => Ok("aarch64-pc-windows-msvc"),
        ("windows", "x86_64", _) => Ok("x86_64-pc-windows-msvc"),
        _ => bail!("unsupported platform: {os}/{arch}"),
    }
}

fn fd_triple(os: &str, arch: &str) -> Result<&'static str> {
    target_triple(os, arch, false)
}

fn jq_asset(os: &str, arch: &str) -> Result<String> {
    let osn = match os {
        "macos" => "macos",
        "linux" => "linux",
        "windows" => "windows",
        _ => bail!("unsupported platform for jq: {os}/{arch}"),
    };
    let an = match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "x86" => "i386",
        _ => bail!("unsupported arch for jq: {os}/{arch}"),
    };
    Ok(if os == "windows" {
        format!("jq-{osn}-{an}.exe")
    } else {
        format!("jq-{osn}-{an}")
    })
}

fn yq_asset(os: &str, arch: &str) -> Result<String> {
    let osn = match os {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        _ => bail!("unsupported platform for yq: {os}/{arch}"),
    };
    let an = match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "x86" => "386",
        _ => bail!("unsupported arch for yq: {os}/{arch}"),
    };
    Ok(format!("yq_{osn}_{an}"))
}

fn difft_asset(os: &str, arch: &str) -> Result<String> {
    let triple = target_triple(os, arch, false)?;
    Ok(format!(
        "difft-{triple}.{}",
        if os == "windows" { "zip" } else { "tar.gz" }
    ))
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

fn shellcheck_asset(tag: &str, os: &str, arch: &str) -> Result<String> {
    let osn = match os {
        "macos" => "darwin",
        "linux" => "linux",
        _ => bail!("unsupported platform for shellcheck: {os}/{arch}"),
    };
    let an = match arch {
        "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        _ => bail!("unsupported arch for shellcheck: {os}/{arch}"),
    };
    Ok(format!("shellcheck-{tag}.{osn}.{an}.tar.xz"))
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
