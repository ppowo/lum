// Shared git command helpers for mirror operations.

use anyhow::{Context, Result};
use std::path::Path;

/// Run a git command, capturing stdout and stderr. Returns error on non-zero exit.
pub fn run_git(args: &[&str], err_context: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .with_context(|| err_context.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{err_context}: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Clone a repository with shallow depth and no single-branch restriction.
pub fn git_clone_shallow(url: &str, branch: &str, dst: &Path, mirror_dir: &Path) -> Result<()> {
    validate_in_mirror(mirror_dir, dst)?;
    let dst_str = dst.to_string_lossy();
    run_git(
        &[
            "clone",
            "--depth",
            "1",
            "--branch",
            branch,
            "--no-single-branch",
            "--no-tags",
            url,
            &dst_str,
        ],
        &format!("cloning {}", url),
    )?;
    Ok(())
}

/// Update an existing shallow clone to the latest remote tip.
pub fn git_update_shallow(repo_path: &Path, branch: &str, mirror_dir: &Path) -> Result<()> {
    validate_in_mirror(mirror_dir, repo_path)?;
    let repo_str = repo_path.to_string_lossy();
    let origin_branch = format!("origin/{branch}");

    // Fetch all branches with depth 1
    run_git(
        &[
            "-C",
            &repo_str,
            "fetch",
            "--depth",
            "1",
            "--all",
            "--no-tags",
        ],
        &format!("fetching {}", repo_str),
    )?;

    // Checkout the target branch
    run_git(
        &["-C", &repo_str, "checkout", branch],
        &format!("checking out {} in {}", branch, repo_str),
    )?;

    // Hard reset to the remote tracking branch
    run_git(
        &["-C", &repo_str, "reset", "--hard", &origin_branch],
        &format!("resetting {} to {}", repo_str, origin_branch),
    )?;

    Ok(())
}

/// Get the remote HEAD SHA for a branch.
pub fn git_remote_head(url: &str, branch: &str) -> Result<String> {
    let ref_spec = format!("refs/heads/{branch}");
    let output = run_git(
        &["ls-remote", url, &ref_spec],
        &format!("querying remote HEAD for {branch} at {url}"),
    )?;

    if output.is_empty() {
        anyhow::bail!("remote branch {branch} not found at {url}");
    }

    // Output format: "<sha>\trefs/heads/<branch>"
    let sha = output.split('\t').next().unwrap_or("");
    if sha.is_empty() {
        anyhow::bail!("no SHA returned for {branch} at {url}");
    }
    Ok(sha.to_string())
}

/// Get the local HEAD SHA for a repository.
pub fn git_local_head(repo_path: &Path) -> Result<String> {
    let repo_str = repo_path.to_string_lossy();
    run_git(
        &["-C", &repo_str, "rev-parse", "HEAD"],
        &format!("reading local HEAD for {}", repo_str),
    )
}

/// Get the local origin/<branch> SHA for a repository (offline status check).
pub fn git_local_origin_head(repo_path: &Path, branch: &str) -> Result<String> {
    let repo_str = repo_path.to_string_lossy();
    let origin_branch = format!("origin/{branch}");
    run_git(
        &["-C", &repo_str, "rev-parse", &origin_branch],
        &format!("reading origin/{branch} for {}", repo_str),
    )
}

/// Validate that a target path is inside the mirror directory (path-traversal guard).
pub fn validate_in_mirror(mirror_dir: &Path, target: &Path) -> Result<()> {
    let abs_mirror = mirror_dir
        .canonicalize()
        .with_context(|| format!("resolving mirror directory {}", mirror_dir.display()))?;
    let abs_target = if target.exists() {
        target
            .canonicalize()
            .with_context(|| format!("resolving target path {}", target.display()))?
    } else {
        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        let abs_parent = parent
            .canonicalize()
            .with_context(|| format!("resolving target parent {}", parent.display()))?;
        abs_parent.join(target.file_name().unwrap_or_default())
    };

    if !abs_target.starts_with(&abs_mirror) {
        anyhow::bail!(
            "path {} is not inside mirror directory {}",
            target.display(),
            mirror_dir.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_in_mirror_rejects_sibling_with_same_prefix() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mirror = tmp.path().join("CodeMirror");
        let sibling = tmp.path().join("CodeMirror-evil");
        std::fs::create_dir_all(&mirror).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();

        assert!(validate_in_mirror(&mirror, &sibling).is_err());
    }
}
