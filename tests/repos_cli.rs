use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use tempfile::TempDir;

/// Helper: create a git repo with one commit (clean state).
fn create_clean_repo(dir: &std::path::Path) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("git init");
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .output()
        .expect("git config email");
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .expect("git config name");
    std::process::Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(dir)
        .output()
        .expect("git config gpgsign");
    std::fs::write(dir.join("README"), "hello").expect("write README");
    std::process::Command::new("git")
        .args(["add", "README"])
        .current_dir(dir)
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir)
        .output()
        .expect("git commit");
}

/// Helper: create a git repo with uncommitted changes (dirty state).
fn create_dirty_repo(dir: &std::path::Path) {
    create_clean_repo(dir);
    std::fs::write(dir.join("README"), "changed").expect("modify README");
}

fn repos_scan_in(dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.args(["repos", "scan"]).current_dir(dir);
    cmd
}

#[test]
fn scan_reports_clean_repo() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("my-repo");
    std::fs::create_dir_all(&repo).unwrap();
    create_clean_repo(&repo);

    repos_scan_in(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("my-repo"))
        .stdout(predicates::str::contains("clean"));
}

#[test]
fn scan_reports_dirty_repo() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("dirty-repo");
    std::fs::create_dir_all(&repo).unwrap();
    create_dirty_repo(&repo);

    repos_scan_in(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("dirty-repo"))
        .stdout(predicates::str::contains("has uncommitted changes"));
}

#[test]
fn scan_finds_nested_repos() {
    let tmp = TempDir::new().unwrap();
    let outer = tmp.path().join("outer");
    let inner = outer.join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    create_clean_repo(&outer);
    create_clean_repo(&inner);

    repos_scan_in(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("outer"))
        .stdout(predicates::str::contains("inner"));
}

#[test]
fn scan_skips_hidden_dirs_by_default() {
    let tmp = TempDir::new().unwrap();
    let visible = tmp.path().join("visible-repo");
    let hidden = tmp.path().join(".hidden-repo");
    std::fs::create_dir_all(&visible).unwrap();
    std::fs::create_dir_all(&hidden).unwrap();
    create_clean_repo(&visible);
    create_clean_repo(&hidden);

    repos_scan_in(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("visible-repo"))
        .stdout(predicates::str::contains("hidden-repo").not());
}

#[test]
fn scan_hidden_flag_includes_hidden_dirs() {
    let tmp = TempDir::new().unwrap();
    let visible = tmp.path().join("visible-repo");
    let hidden = tmp.path().join(".hidden-repo");
    std::fs::create_dir_all(&visible).unwrap();
    std::fs::create_dir_all(&hidden).unwrap();
    create_clean_repo(&visible);
    create_clean_repo(&hidden);

    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.args(["repos", "scan", "--hidden"])
        .current_dir(tmp.path());
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("visible-repo"))
        .stdout(predicates::str::contains("hidden-repo"));
}

#[test]
fn scan_with_explicit_path() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("my-repo");
    std::fs::create_dir_all(&repo).unwrap();
    create_clean_repo(&repo);

    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.args(["repos", "scan", &tmp.path().to_string_lossy()]);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("my-repo"));
}

#[test]
fn scan_accepts_jobs_limit_flag() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("my-repo");
    std::fs::create_dir_all(&repo).unwrap();
    create_clean_repo(&repo);

    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.args(["repos", "scan", "-j", "1", &tmp.path().to_string_lossy()]);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("my-repo"));
}

#[test]
fn scan_reports_missing_git_before_scanning() {
    let tmp = TempDir::new().unwrap();
    let empty_path = tmp.path().join("empty-path");
    std::fs::create_dir_all(&empty_path).unwrap();

    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.env("PATH", empty_path)
        .args(["repos", "scan", tmp.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "git executable not found on PATH",
        ));
}

fn lum_with_xdg(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_DOCUMENTS_DIR", home.path().join("Documents"));
    cmd
}

#[test]
fn mirror_sync_reports_missing_git_before_loading_work() {
    let tmp = TempDir::new().unwrap();
    let empty_path = tmp.path().join("empty-path");
    std::fs::create_dir_all(&empty_path).unwrap();

    let mut cmd = lum_with_xdg(&tmp);
    cmd.env("PATH", empty_path)
        .args(["repos", "mirror", "sync"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "git executable not found on PATH",
        ));
}

// --- mirror tests ---

#[test]
fn mirror_config_path_prints_path() {
    let home = TempDir::new().unwrap();
    lum_with_xdg(&home)
        .args(["repos", "mirror", "config-path"])
        .assert()
        .success()
        .stdout(predicates::str::contains("repos.json"));
}

#[test]
fn mirror_dir_prints_documents_codemirror() {
    let home = TempDir::new().unwrap();
    lum_with_xdg(&home)
        .args(["repos", "mirror", "dir"])
        .assert()
        .success()
        .stdout(predicates::str::contains("CodeMirror"));
}

#[test]
fn mirror_init_creates_config() {
    let home = TempDir::new().unwrap();
    lum_with_xdg(&home)
        .args(["repos", "mirror", "init"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Created"));

    // Config file should now exist
    let config_path = home.path().join("config").join("lum").join("repos.json");
    assert!(config_path.exists());
}

#[test]
fn mirror_init_does_not_overwrite() {
    let home = TempDir::new().unwrap();
    lum_with_xdg(&home)
        .args(["repos", "mirror", "init"])
        .assert()
        .success();

    lum_with_xdg(&home)
        .args(["repos", "mirror", "init"])
        .assert()
        .success()
        .stdout(predicates::str::contains("already exists"));
}

#[test]
fn mirror_list_shows_configured_repos() {
    let home = TempDir::new().unwrap();
    lum_with_xdg(&home)
        .args(["repos", "mirror", "init"])
        .assert()
        .success();

    lum_with_xdg(&home)
        .args(["repos", "mirror", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("example-repo-main-sample"));
}

#[test]
fn mirror_list_reports_no_config() {
    let home = TempDir::new().unwrap();
    lum_with_xdg(&home)
        .args(["repos", "mirror", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("not found"));
}

#[test]
fn mirror_sync_reports_json_parse_details() {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join("config").join("lum");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("repos.json"),
        r#"{"repos":[{"url":"https://github.com/example/one.git"}]"#,
    )
    .unwrap();

    lum_with_xdg(&home)
        .args(["repos", "mirror", "sync"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("parsing"))
        .stderr(predicates::str::contains("repos.json"))
        .stderr(predicates::str::contains("EOF"));
}

fn write_mirror_config(home: &TempDir, repos: &[(&str, &str)]) {
    let config_dir = home.path().join("config").join("lum");
    std::fs::create_dir_all(&config_dir).unwrap();
    let entries: Vec<_> = repos
        .iter()
        .map(|(url, branch)| serde_json::json!({ "url": url, "branch": branch }))
        .collect();
    let config = serde_json::json!({ "repos": entries });
    std::fs::write(
        config_dir.join("repos.json"),
        serde_json::to_string(&config).unwrap(),
    )
    .unwrap();
}

#[cfg(unix)]
fn fake_git_dir(home: &TempDir) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let bin = home.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let git = bin.join("git");
    std::fs::write(
        &git,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo git version 2.0.0
  exit 0
fi
if [ "$1" = "clone" ]; then
  dst=""
  for arg in "$@"; do dst="$arg"; done
  sleep 1
  mkdir -p "$dst/.git"
  exit 0
fi
if [ "$1" = "-C" ] && [ "$3" = "rev-parse" ]; then
  sleep 1
  if [ "$4" = "--abbrev-ref" ]; then
    echo main
  else
    echo abc
  fi
  exit 0
fi
echo "unexpected git invocation: $@" >&2
exit 1
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&git).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&git, perms).unwrap();
    bin
}

#[cfg(unix)]
#[test]
fn mirror_sync_runs_multiple_clone_jobs_concurrently() {
    let home = TempDir::new().unwrap();
    write_mirror_config(
        &home,
        &[
            ("https://github.com/example/one.git", "main"),
            ("https://github.com/example/two.git", "main"),
        ],
    );
    let fake_bin = fake_git_dir(&home);
    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let started = std::time::Instant::now();
    lum_with_xdg(&home)
        .env("PATH", path)
        .args(["repos", "mirror", "sync", "-j", "2"])
        .assert()
        .success();

    assert!(
        started.elapsed() < std::time::Duration::from_millis(1800),
        "two one-second clone jobs should overlap when -j 2 is used"
    );
}

#[cfg(unix)]
#[test]
fn mirror_status_runs_multiple_checks_concurrently() {
    let home = TempDir::new().unwrap();
    write_mirror_config(
        &home,
        &[
            ("https://github.com/example/one.git", "main"),
            ("https://github.com/example/two.git", "main"),
        ],
    );
    let mirror = home.path().join("Documents").join("CodeMirror");
    std::fs::create_dir_all(mirror.join("one-main/.git")).unwrap();
    std::fs::create_dir_all(mirror.join("two-main/.git")).unwrap();
    let fake_bin = fake_git_dir(&home);
    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let started = std::time::Instant::now();
    lum_with_xdg(&home)
        .env("PATH", path)
        .args(["repos", "mirror", "status", "--offline", "-j", "2"])
        .assert()
        .success();

    assert!(
        started.elapsed() < std::time::Duration::from_millis(2800),
        "two two-second status checks should overlap when -j 2 is used"
    );
}
