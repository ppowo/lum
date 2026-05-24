use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result, anyhow, bail};
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use xz2::read::XzDecoder;

use crate::cli::ToolsCommand;

#[derive(Clone, Copy)]
struct ToolSpec {
    name: &'static str,
    binary: &'static str,
    description: &'static str,
    version_args: &'static [&'static str],
    owner: &'static str,
    repo: &'static str,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: Option<String>,
    name: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug)]
struct Artifact {
    version: String,
    release_tag: String,
    asset_name: String,
    download_url: String,
    archive_type: ArchiveType,
    binary_path: String,
    checksum_sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveType {
    Binary,
    Zip,
    TarGz,
    TarXz,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ToolsState {
    version: String,
    tools: BTreeMap<String, ToolState>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolState {
    installed: bool,
    path: PathBuf,
    installed_version: String,
    installed_at: SystemTime,
    updated_at: SystemTime,
    artifact: ArtifactState,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArtifactState {
    release_tag: String,
    asset_name: String,
    download_url: String,
    checksum: Option<String>,
}

struct LocalStatus {
    managed: bool,
    path: PathBuf,
    exists: bool,
    stored_version: Option<String>,
    runtime_version: Option<String>,
}

const CATALOG: &[ToolSpec] = &[
    ToolSpec {
        name: "difftastic",
        binary: "difft",
        description: "A structural diff that understands syntax",
        version_args: &["--version"],
        owner: "Wilfred",
        repo: "difftastic",
    },
    ToolSpec {
        name: "fd",
        binary: "fd",
        description: "A simple, fast and user-friendly alternative to find",
        version_args: &["--version"],
        owner: "sharkdp",
        repo: "fd",
    },
    ToolSpec {
        name: "jq",
        binary: "jq",
        description: "A lightweight and flexible command-line JSON processor",
        version_args: &["--version"],
        owner: "jqlang",
        repo: "jq",
    },
    ToolSpec {
        name: "ripgrep",
        binary: "rg",
        description: "Recursively searches directories for a regex pattern",
        version_args: &["--version"],
        owner: "BurntSushi",
        repo: "ripgrep",
    },
    ToolSpec {
        name: "scc",
        binary: "scc",
        description: "Fast code counter with complexity",
        version_args: &["--version"],
        owner: "boyter",
        repo: "scc",
    },
    ToolSpec {
        name: "shellcheck",
        binary: "shellcheck",
        description: "Static analysis for shell scripts",
        version_args: &["--version"],
        owner: "koalaman",
        repo: "shellcheck",
    },
    ToolSpec {
        name: "universal-ctags",
        binary: "ctags",
        description: "Maintained ctags implementation for source code indexing",
        version_args: &[],
        owner: "universal-ctags",
        repo: "ctags-nightly-build",
    },
    ToolSpec {
        name: "yq",
        binary: "yq",
        description: "YAML, JSON, XML, CSV, TSV and properties processor",
        version_args: &["--version"],
        owner: "mikefarah",
        repo: "yq",
    },
];

pub fn run(command: ToolsCommand) -> Result<()> {
    match command {
        ToolsCommand::Ls => list(),
        ToolsCommand::Install { tool, force } => install_cmd(&tool, force),
        ToolsCommand::Status { tool } => status_cmd(&tool),
        ToolsCommand::Sync { dry_run } => sync_cmd(dry_run),
        ToolsCommand::Update { tool, force } => update_cmd(&tool, force),
        ToolsCommand::Version { tool } => version_cmd(&tool),
    }
}

fn list() -> Result<()> {
    println!(
        "{:<18} {:<12} {:<12} DESCRIPTION",
        "TOOL", "BINARY", "STATE"
    );
    for tool in CATALOG {
        let status = local_status(tool)?;
        let state = if status.managed && status.exists {
            "installed"
        } else if status.managed {
            "missing"
        } else if status.exists {
            "unmanaged"
        } else {
            "available"
        };
        println!(
            "{:<18} {:<12} {:<12} {}",
            tool.name, tool.binary, state, tool.description
        );
    }
    Ok(())
}

fn status_cmd(tool: &str) -> Result<()> {
    let spec = lookup_tool(tool)?;
    let status = local_status(spec)?;
    println!("Tool:              {}", spec.name);
    println!("Binary:            {}", spec.binary);
    println!("Managed:           {}", yes_no(status.managed));
    println!("Path:              {}", status.path.display());
    println!("Exists:            {}", yes_no(status.exists));
    println!(
        "Installed version: {}",
        status.effective_version().unwrap_or("unknown")
    );
    Ok(())
}

fn install_cmd(tool: &str, force: bool) -> Result<()> {
    let spec = lookup_tool(tool)?;
    let status = local_status(spec)?;
    if status.exists && !status.managed && !force {
        bail!(
            "{} already exists at {} but is not managed by lum; rerun with --force to overwrite it",
            spec.name,
            status.path.display()
        );
    }
    if status.exists && status.managed && !force {
        bail!(
            "{} is already installed at {}; use 'lum tools update {}' or rerun with --force",
            spec.name,
            status.path.display(),
            spec.name
        );
    }
    let artifact = resolve_latest(spec)?;
    let state = install_artifact(spec, &artifact, None)?;
    println!(
        "✓ Installed {} {} to {}",
        spec.name,
        state.installed_version,
        state.path.display()
    );
    Ok(())
}

fn update_cmd(tool: &str, force: bool) -> Result<()> {
    let spec = lookup_tool(tool)?;
    let status = local_status(spec)?;
    if !status.managed {
        bail!(
            "{} is not installed; use 'lum tools install {}' first",
            spec.name,
            spec.name
        );
    }
    let artifact = resolve_latest(spec)?;
    let previous = status.effective_version().unwrap_or("unknown").to_owned();
    if !force && previous != "unknown" && compare_versions(&previous, &artifact.version) >= 0 {
        println!("{} is already up to date ({})", spec.name, previous);
        return Ok(());
    }
    let state = install_artifact(spec, &artifact, status.installed_at()?)?;
    println!(
        "✓ Updated {} {} -> {} at {}",
        spec.name,
        previous,
        state.installed_version,
        state.path.display()
    );
    Ok(())
}

fn sync_cmd(dry_run: bool) -> Result<()> {
    let mut installed = 0;
    let mut updated = 0;
    let mut up_to_date = 0;
    for spec in CATALOG {
        let status = local_status(spec)?;
        let artifact = resolve_latest(spec)?;
        match status.effective_version() {
            None if dry_run => println!("• {}: would install {}", spec.name, artifact.version),
            None => {
                install_artifact(spec, &artifact, None)?;
                println!("• {}: installed {}", spec.name, artifact.version);
                installed += 1;
            }
            Some(current) if compare_versions(current, &artifact.version) < 0 && dry_run => {
                println!(
                    "• {}: would update {} -> {}",
                    spec.name, current, artifact.version
                )
            }
            Some(current) if compare_versions(current, &artifact.version) < 0 => {
                install_artifact(spec, &artifact, status.installed_at()?)?;
                println!(
                    "• {}: updated {} -> {}",
                    spec.name, current, artifact.version
                );
                updated += 1;
            }
            Some(current) => {
                println!("• {}: up to date ({})", spec.name, current);
                up_to_date += 1;
            }
        }
    }
    println!("\nSummary: {installed} installed, {updated} updated, {up_to_date} up to date");
    Ok(())
}

fn version_cmd(tool: &str) -> Result<()> {
    let spec = lookup_tool(tool)?;
    let status = local_status(spec)?;
    let artifact = resolve_latest(spec)?;
    println!(
        "Installed: {}",
        status.effective_version().unwrap_or("unknown")
    );
    println!("Latest:    {}", artifact.version);
    Ok(())
}

impl LocalStatus {
    fn effective_version(&self) -> Option<&str> {
        self.runtime_version
            .as_deref()
            .or(self.stored_version.as_deref())
    }

    fn installed_at(&self) -> Result<Option<SystemTime>> {
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

fn lookup_tool(name: &str) -> Result<&'static ToolSpec> {
    CATALOG
        .iter()
        .find(|tool| tool.name == name)
        .with_context(|| {
            format!(
                "unknown managed tool {name:?} (available: {})",
                available_tools()
            )
        })
}

fn available_tools() -> String {
    CATALOG
        .iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>()
        .join(", ")
}

fn local_status(spec: &ToolSpec) -> Result<LocalStatus> {
    let state = load_state()?;
    let path = state
        .tools
        .get(spec.name)
        .map(|s| s.path.clone())
        .unwrap_or(tool_path(spec)?);
    let exists = path.exists();
    let managed = state.tools.get(spec.name).is_some_and(|s| s.installed);
    let stored_version = state
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

fn resolve_latest(spec: &ToolSpec) -> Result<Artifact> {
    if let Ok(path) = std::env::var(test_artifact_env(spec)) {
        return Ok(Artifact {
            version: "test".into(),
            release_tag: "test".into(),
            asset_name: Path::new(&path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            download_url: path,
            archive_type: ArchiveType::Binary,
            binary_path: installed_filename(spec.binary),
            checksum_sha256: None,
        });
    }
    let release = latest_release(spec.owner, spec.repo)?;
    artifact_for_release(spec, &release)
}

fn latest_release(owner: &str, repo: &str) -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("lum-tools/1.0")
        .build()?;
    let text = client.get(&url).send()?.error_for_status()?.text()?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse GitHub release for {owner}/{repo}"))
}

fn artifact_for_release(spec: &ToolSpec, release: &GitHubRelease) -> Result<Artifact> {
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

fn install_artifact(
    spec: &ToolSpec,
    artifact: &Artifact,
    preserve_installed_at: Option<SystemTime>,
) -> Result<ToolState> {
    fs::create_dir_all(bin_dir()?)?;
    let target = tool_path(spec)?;
    let temp_dir = tempfile::Builder::new().prefix("lum-tools-").tempdir()?;
    let download = temp_dir.path().join(&artifact.asset_name);
    download_artifact(&artifact.download_url, &download)?;
    verify_sha256(&download, artifact.checksum_sha256.as_deref())?;
    let source = materialize_binary(&download, temp_dir.path(), artifact)?;
    install_file(&source, &target)?;

    let mut state = load_state()?;
    let now = SystemTime::now();
    let installed_at = preserve_installed_at
        .or_else(|| state.tools.get(spec.name).map(|s| s.installed_at))
        .unwrap_or(now);
    let tool_state = ToolState {
        installed: true,
        path: target,
        installed_version: artifact.version.clone(),
        installed_at,
        updated_at: now,
        artifact: ArtifactState {
            release_tag: artifact.release_tag.clone(),
            asset_name: artifact.asset_name.clone(),
            download_url: artifact.download_url.clone(),
            checksum: artifact.checksum_sha256.clone(),
        },
    };
    state.tools.insert(spec.name.to_owned(), tool_state);
    save_state(&state)?;
    Ok(state.tools.remove(spec.name).unwrap())
}

fn download_artifact(url: &str, dest: &Path) -> Result<()> {
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

fn materialize_binary(download: &Path, temp_dir: &Path, artifact: &Artifact) -> Result<PathBuf> {
    if artifact.archive_type == ArchiveType::Binary {
        return Ok(download.to_owned());
    }
    let extract_dir = temp_dir.join("extract");
    fs::create_dir_all(&extract_dir)?;
    extract_archive(download, &extract_dir, artifact.archive_type)?;
    let expected = extract_dir.join(Path::new(&artifact.binary_path));
    if expected.is_file() {
        return Ok(expected);
    }
    let basename = Path::new(&artifact.binary_path)
        .file_name()
        .context("artifact binary path missing filename")?;
    find_file_named(&extract_dir, basename).with_context(|| {
        format!(
            "could not find {} in extracted archive",
            artifact.binary_path
        )
    })
}

fn extract_archive(src: &Path, dest: &Path, archive_type: ArchiveType) -> Result<()> {
    match archive_type {
        ArchiveType::Zip => {
            let file = fs::File::open(src)?;
            zip::ZipArchive::new(file)?.extract(dest)?;
        }
        ArchiveType::TarGz => {
            let file = fs::File::open(src)?;
            tar::Archive::new(GzDecoder::new(file)).unpack(dest)?;
        }
        ArchiveType::TarXz => {
            let file = fs::File::open(src)?;
            tar::Archive::new(XzDecoder::new(file)).unpack(dest)?;
        }
        ArchiveType::Binary => {}
    }
    Ok(())
}

fn find_file_named(dir: &Path, filename: &std::ffi::OsStr) -> Result<PathBuf> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(found) = find_file_named(&path, filename) {
                return Ok(found);
            }
        } else if path.file_name() == Some(filename) {
            return Ok(path);
        }
    }
    Err(anyhow!("not found"))
}

fn install_file(source: &Path, target: &Path) -> Result<()> {
    let temp = target.with_extension("lum-tmp");
    fs::copy(source, &temp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp, fs::Permissions::from_mode(0o755))?;
    }
    if cfg!(windows) && target.exists() {
        fs::remove_file(target)?;
    }
    fs::rename(&temp, target).or_else(|_| {
        fs::copy(&temp, target)?;
        fs::remove_file(&temp)?;
        Ok::<_, std::io::Error>(())
    })?;
    Ok(())
}

fn probe_version(spec: &ToolSpec, path: &Path) -> Result<String> {
    if spec.version_args.is_empty() {
        bail!("no version args configured");
    }
    let output = Command::new(path).args(spec.version_args).output()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if text.is_empty() {
        bail!("version command produced no output");
    }
    Ok(extract_version(&text).unwrap_or(text))
}

fn extract_version(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|part| part.chars().any(|c| c.is_ascii_digit()))
        .map(|s| s.trim_start_matches('v').to_owned())
}

fn compare_versions(a: &str, b: &str) -> i32 {
    let parse = |s: &str| {
        s.trim_start_matches('v')
            .split(['.', '-'])
            .take(3)
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let av = parse(a);
    let bv = parse(b);
    for i in 0..3 {
        let aa = *av.get(i).unwrap_or(&0);
        let bb = *bv.get(i).unwrap_or(&0);
        if aa < bb {
            return -1;
        }
        if aa > bb {
            return 1;
        }
    }
    0
}

fn verify_sha256(path: &Path, expected: Option<&str>) -> Result<()> {
    let Some(expected) = expected.filter(|s| !s.trim().is_empty()) else {
        return Ok(());
    };
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected.trim()) {
        bail!("checksum mismatch for {}", path.display());
    }
    Ok(())
}

fn load_state() -> Result<ToolsState> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(ToolsState {
            version: "1.0".into(),
            tools: BTreeMap::new(),
        });
    }
    let data = fs::read_to_string(&path)?;
    let mut state: ToolsState = serde_json::from_str(&data)?;
    if state.version.is_empty() {
        state.version = "1.0".into();
    }
    Ok(state)
}

fn save_state(state: &ToolsState) -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("dev", "ppowo", "lum").context("failed to determine platform directories")
}
fn state_path() -> Result<PathBuf> {
    Ok(dirs()?.config_dir().join("tools-state.json"))
}
fn bin_dir() -> Result<PathBuf> {
    Ok(dirs()?.data_dir().join("bin"))
}
fn tool_path(spec: &ToolSpec) -> Result<PathBuf> {
    Ok(bin_dir()?.join(installed_filename(spec.binary)))
}
fn installed_filename(binary: &str) -> String {
    if cfg!(windows) && !binary.to_ascii_lowercase().ends_with(".exe") {
        format!("{binary}.exe")
    } else {
        binary.to_owned()
    }
}
fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
fn test_artifact_env(spec: &ToolSpec) -> String {
    format!(
        "LUM_TOOLS_TEST_ARTIFACT_{}",
        spec.name.replace('-', "_").to_ascii_uppercase()
    )
}
fn detect_archive_type(name: &str) -> ArchiveType {
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
