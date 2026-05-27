use anyhow::Result;
use clap::Args;
use std::path::{Path, PathBuf};

use super::ensure_git_on_path;

#[derive(Debug, Args, Clone)]
pub struct ScanArgs {
    /// Directory to scan. Defaults to the current directory. By default, contacts upstream remotes.
    pub path: Option<String>,

    /// Descend into hidden (dot-prefixed) directories.
    #[arg(long)]
    pub hidden: bool,

    /// Compare against cached remote refs instead of contacting remotes.
    #[arg(long)]
    pub offline: bool,

    /// Maximum concurrent git operations.
    #[arg(short = 'j', default_value = "4")]
    pub jobs: usize,
}

pub fn run(args: &ScanArgs) -> Result<()> {
    ensure_git_on_path()?;
    let root = args.path.as_deref().unwrap_or(".");
    let root = Path::new(root);
    scan_path(root, args.hidden, args.jobs, args.offline)?;
    Ok(())
}

fn scan_path(root: &Path, include_hidden: bool, jobs: usize, offline: bool) -> Result<()> {
    let mut repos = Vec::new();
    walk(root, include_hidden, &mut repos)?;
    repos.sort_by(|a, b| a.path.cmp(&b.path));
    if offline {
        eprintln!("info: offline mode; using cached remote refs only");
    }

    let jobs = jobs.max(1);
    std::thread::scope(|scope| {
        for chunk in repos.chunks(jobs) {
            let handles: Vec<_> = chunk
                .iter()
                .map(|repo| {
                    scope.spawn(move || {
                        inspect_repo(&repo.path, offline).map(|status| (&repo.path, status))
                    })
                })
                .collect();

            for handle in handles {
                let (path, status) = handle
                    .join()
                    .unwrap_or_else(|_| panic!("git status worker panicked"))?;
                println!(
                    "{}  {}  {}; {}",
                    path.display(),
                    status.branch,
                    status.worktree,
                    render_upstream(&status)
                );
                if let FetchStatus::Failed(message) = &status.fetch {
                    eprintln!(
                        "  warning: git fetch failed for {}: {message}",
                        path.display()
                    );
                }
                if status.status_failed {
                    eprintln!("  warning: git status failed for {}", path.display());
                }
            }
        }
        Ok(())
    })
}

struct FoundRepo {
    path: PathBuf,
}

struct RepoStatus {
    branch: String,
    upstream: String,
    worktree: String,
    fetch: FetchStatus,
    status_failed: bool,
}

enum FetchStatus {
    Fetched,
    Offline,
    SkippedNoUpstream,
    SkippedDetachedHead,
    Failed(String),
}

fn render_upstream(status: &RepoStatus) -> String {
    match &status.fetch {
        FetchStatus::Fetched | FetchStatus::Offline => status.upstream.clone(),
        FetchStatus::SkippedNoUpstream => {
            format!("{}; fetch skipped (no upstream)", status.upstream)
        }
        FetchStatus::SkippedDetachedHead => {
            format!("{}; fetch skipped (detached HEAD)", status.upstream)
        }
        FetchStatus::Failed(_) => format!("fetch failed; status may be stale; {}", status.upstream),
    }
}

fn walk(dir: &Path, include_hidden: bool, repos: &mut Vec<FoundRepo>) -> Result<()> {
    let is_git_dir = dir.join(".git").exists();
    if is_git_dir {
        repos.push(FoundRepo {
            path: dir.to_path_buf(),
        });
    }

    // Continue walking inside repos so nested repos are reported too, but never
    // descend into Git's own metadata directory.
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("warning: cannot enter {}: {e}", dir.display());
            return Ok(());
        }
    };

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip .git directories
        if name_str == ".git" {
            continue;
        }

        // Skip dot-entries unless --hidden
        if !include_hidden && name_str.starts_with('.') {
            continue;
        }

        let path = entry.path();

        // Skip symlinks
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.is_symlink() {
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }

        walk(&path, include_hidden, repos)?;
    }

    Ok(())
}

fn inspect_repo(path: &Path, offline: bool) -> Result<RepoStatus> {
    let fetch = if offline {
        FetchStatus::Offline
    } else {
        fetch_upstream(path)
    };

    let output = match std::process::Command::new("git")
        .args([
            "-C",
            &path.to_string_lossy(),
            "status",
            "--porcelain=v1",
            "--branch",
        ])
        .output()
    {
        Ok(o) => o,
        Err(_) => {
            return Ok(RepoStatus {
                branch: "unknown".into(),
                upstream: "status failed".into(),
                worktree: "unknown".into(),
                fetch,
                status_failed: true,
            });
        }
    };

    if !output.status.success() {
        return Ok(RepoStatus {
            branch: "unknown".into(),
            upstream: "status failed".into(),
            worktree: "unknown".into(),
            fetch,
            status_failed: true,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut status = parse_git_status(&stdout)?;
    status.fetch = fetch;
    Ok(status)
}

fn fetch_upstream(path: &Path) -> FetchStatus {
    let repo = path.to_string_lossy();
    let branch = match std::process::Command::new("git")
        .args(["-C", &repo, "symbolic-ref", "--quiet", "--short", "HEAD"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => return FetchStatus::SkippedDetachedHead,
    };

    let remote = match std::process::Command::new("git")
        .args(["-C", &repo, "config", &format!("branch.{branch}.remote")])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => return FetchStatus::SkippedNoUpstream,
    };

    if remote.is_empty() {
        return FetchStatus::SkippedNoUpstream;
    }

    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args([
            "-C",
            &repo,
            "fetch",
            "--no-tags",
            "--no-recurse-submodules",
            &remote,
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => FetchStatus::Fetched,
        Ok(output) => FetchStatus::Failed(first_stderr_line(&output.stderr)),
        Err(error) => FetchStatus::Failed(error.to_string()),
    }
}

fn first_stderr_line(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("git fetch failed")
        .to_string()
}

fn parse_git_status(output: &str) -> Result<RepoStatus> {
    let mut branch = "unknown".to_string();
    let mut upstream = "unknown".to_string();
    let mut saw_change = false;

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            parse_branch_line(rest, &mut branch, &mut upstream);
        } else if !line.is_empty() {
            saw_change = true;
        }
    }

    let worktree = if saw_change {
        "has uncommitted changes".to_string()
    } else {
        "clean".to_string()
    };

    Ok(RepoStatus {
        branch,
        upstream,
        worktree,
        fetch: FetchStatus::Fetched,
        status_failed: false,
    })
}

fn parse_branch_line(line: &str, branch: &mut String, upstream: &mut String) {
    // Detached HEAD
    if line.starts_with("HEAD (no branch)") || line.starts_with("HEAD detached") {
        *branch = "detached HEAD".into();
        *upstream = "none".into();
        return;
    }

    let Some(tracking) = line.find("...") else {
        // No tracking branch
        *branch = line.to_string();
        *upstream = "none".into();
        return;
    };

    let branch_part = &line[..tracking];
    let rest = &line[tracking + 3..];

    *branch = branch_part.to_string();

    let Some(bracket) = rest.find('[') else {
        // Synced, no divergence info
        *upstream = format!("synced with {rest}");
        return;
    };

    let remote_part = &rest[..bracket - 1]; // -1 for the space before [
    let details = &rest[bracket..];

    let has_ahead = details.contains("ahead");
    let has_behind = details.contains("behind");

    if has_ahead && has_behind {
        *upstream = format!("diverged from {remote_part}; {details}");
    } else if has_ahead {
        *upstream = format!("ahead of {remote_part}; {details}");
    } else if has_behind {
        *upstream = format!("behind {remote_part}; {details}");
    } else {
        *upstream = format!("synced with {remote_part}");
    }
}
