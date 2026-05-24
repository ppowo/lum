use assert_cmd::Command;
use tempfile::TempDir;

fn lum_with_env(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"));
    cmd
}

#[test]
fn tools_ls_lists_the_managed_catalog() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["tools", "ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("ripgrep"))
        .stdout(predicates::str::contains("rg"))
        .stdout(predicates::str::contains("shellcheck"));
}

#[test]
fn tools_status_reports_missing_tool() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["tools", "status", "ripgrep"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Tool:              ripgrep"))
        .stdout(predicates::str::contains("Managed:           no"))
        .stdout(predicates::str::contains("Exists:            no"));
}

#[test]
fn tools_rejects_unknown_tools_with_available_names() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["tools", "status", "missing"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "unknown managed tool \"missing\"",
        ))
        .stderr(predicates::str::contains("ripgrep"));
}

#[test]
fn tools_status_reports_unmanaged_existing_binary() {
    let home = TempDir::new().unwrap();
    let bin = home.path().join("data").join("lum").join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("rg"), "manual").unwrap();

    lum_with_env(&home)
        .args(["tools", "status", "ripgrep"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Managed:           no"))
        .stdout(predicates::str::contains("Exists:            yes"));
}

#[test]
fn tools_install_uses_local_artifact_override_and_records_managed_state() {
    let home = TempDir::new().unwrap();
    let artifact = home.path().join("rg-source");
    std::fs::write(&artifact, "fake rg").unwrap();

    lum_with_env(&home)
        .env("LUM_TOOLS_TEST_ARTIFACT_RIPGREP", &artifact)
        .args(["tools", "install", "ripgrep"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Installed ripgrep"));

    lum_with_env(&home)
        .args(["tools", "status", "ripgrep"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Managed:           yes"))
        .stdout(predicates::str::contains("Exists:            yes"));
}

#[test]
fn tools_install_protects_unmanaged_files_without_force() {
    let home = TempDir::new().unwrap();
    let bin = home.path().join("data").join("lum").join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("rg"), "manual").unwrap();
    let artifact = home.path().join("rg-source");
    std::fs::write(&artifact, "fake rg").unwrap();

    lum_with_env(&home)
        .env("LUM_TOOLS_TEST_ARTIFACT_RIPGREP", &artifact)
        .args(["tools", "install", "ripgrep"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not managed by lum"));
}

#[test]
fn tools_install_force_takes_over_unmanaged_files() {
    let home = TempDir::new().unwrap();
    let bin = home.path().join("data").join("lum").join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("rg"), "manual").unwrap();
    let artifact = home.path().join("rg-source");
    std::fs::write(&artifact, "fake rg").unwrap();

    lum_with_env(&home)
        .env("LUM_TOOLS_TEST_ARTIFACT_RIPGREP", &artifact)
        .args(["tools", "install", "ripgrep", "--force"])
        .assert()
        .success();

    assert_eq!(std::fs::read_to_string(bin.join("rg")).unwrap(), "fake rg");
}
