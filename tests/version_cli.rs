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
fn short_version_flag_matches_long_version_flag() {
    let long = Command::cargo_bin("lum")
        .unwrap()
        .arg("--version")
        .output()
        .unwrap();
    let short = Command::cargo_bin("lum")
        .unwrap()
        .arg("-V")
        .output()
        .unwrap();
    assert_eq!(short.stdout, long.stdout);
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
