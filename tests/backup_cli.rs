use assert_cmd::Command;

#[test]
fn backup_is_a_subcommand_with_bio_and_openemu_targets() {
    Command::cargo_bin("lum")
        .unwrap()
        .args(["backup", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Backup and restore"))
        .stdout(predicates::str::contains("bio"))
        .stdout(predicates::str::contains("openemu"));
}

#[test]
fn backup_bio_reports_missing_source_directory() {
    let home = tempfile::TempDir::new().unwrap();

    Command::cargo_bin("lum")
        .unwrap()
        .env("HOME", home.path())
        .args(["backup", "bio"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("bio directory not found"));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn backup_openemu_reports_unsupported_os() {
    Command::cargo_bin("lum")
        .unwrap()
        .args(["backup", "openemu"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "openemu backup is only supported",
        ));
}
