use assert_cmd::Command;

#[test]
fn version_includes_package_version() {
    Command::cargo_bin("lum")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "lum {}",
            env!("CARGO_PKG_VERSION")
        )));
}

#[test]
fn version_includes_commit_hash() {
    Command::cargo_bin("lum")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::is_match(r"lum \d+\.\d+\.\d+ \([a-f0-9]{7,40}\)").unwrap());
}

#[test]
fn version_includes_build_timestamp() {
    Command::cargo_bin("lum")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("built"));
}
