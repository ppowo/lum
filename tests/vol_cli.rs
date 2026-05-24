use assert_cmd::Command;

#[test]
fn vol_is_a_subcommand() {
    Command::cargo_bin("lum")
        .unwrap()
        .args(["vol", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Set system volume"));
}

#[test]
fn vol_rejects_non_numeric_volume() {
    Command::cargo_bin("lum")
        .unwrap()
        .args(["vol", "abc"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid value"));
}

#[test]
fn vol_rejects_volume_above_100() {
    Command::cargo_bin("lum")
        .unwrap()
        .args(["vol", "200"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("volume must be between 0 and 100"));
}
