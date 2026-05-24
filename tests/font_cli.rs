use std::io::Write;

use assert_cmd::Command;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

fn lum_with_env(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"));
    cmd
}

/// Create a minimal zip containing fake TTF files for testing.
fn create_test_font_zip(dir: &std::path::Path) -> std::path::PathBuf {
    let zip_path = dir.join("test-font.zip");
    let file = std::fs::File::create(&zip_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    writer.start_file("FakeFont-Regular.ttf", options).unwrap();
    writer.write_all(b"fake-ttf-content").unwrap();

    writer
        .start_file("subdir/FakeFont-Bold.ttf", options)
        .unwrap();
    writer.write_all(b"fake-ttf-bold").unwrap();

    // Include a non-TTF file that should be skipped
    writer.start_file("readme.txt", options).unwrap();
    writer.write_all(b"not a font").unwrap();

    writer.finish().unwrap();
    zip_path
}

#[test]
fn font_ls_lists_the_managed_catalog() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["font", "ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dmca-sans-serif"))
        .stdout(predicates::str::contains("available"));
}

#[test]
fn font_ls_shows_installed_for_present_font_dir() {
    let home = TempDir::new().unwrap();
    let font_dir = home
        .path()
        .join("data")
        .join("fonts")
        .join("dmca-sans-serif");
    std::fs::create_dir_all(&font_dir).unwrap();

    lum_with_env(&home)
        .args(["font", "ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("installed"));
}

#[test]
fn font_install_rejects_unknown_font() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["font", "install", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown font \"nonexistent\""))
        .stderr(predicates::str::contains("dmca-sans-serif"));
}

#[test]
fn font_uninstall_rejects_unknown_font() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["font", "uninstall", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown font \"nonexistent\""));
}

#[test]
fn font_uninstall_bails_if_not_installed() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["font", "uninstall", "dmca-sans-serif"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not installed"));
}

#[test]
fn font_install_downloads_and_extracts_ttf_files() {
    let home = TempDir::new().unwrap();
    let zip_path = create_test_font_zip(home.path());

    lum_with_env(&home)
        .env("LUM_FONT_TEST_ARTIFACT_DMCA_SANS_SERIF", &zip_path)
        .args(["font", "install", "dmca-sans-serif"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Installed dmca-sans-serif"));

    // Verify TTF files were extracted (not the .txt)
    let font_dir = home
        .path()
        .join("data")
        .join("fonts")
        .join("dmca-sans-serif");
    assert!(font_dir.join("FakeFont-Regular.ttf").exists());
    assert!(font_dir.join("FakeFont-Bold.ttf").exists());
    assert!(!font_dir.join("readme.txt").exists());
    assert!(!font_dir.join("subdir").exists());

    // Verify ls now shows "installed"
    lum_with_env(&home)
        .args(["font", "ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("installed"));
}

#[test]
fn font_install_bails_if_already_installed_without_force() {
    let home = TempDir::new().unwrap();
    let zip_path = create_test_font_zip(home.path());

    // Install first time
    lum_with_env(&home)
        .env("LUM_FONT_TEST_ARTIFACT_DMCA_SANS_SERIF", &zip_path)
        .args(["font", "install", "dmca-sans-serif"])
        .assert()
        .success();

    // Second install without --force should fail
    lum_with_env(&home)
        .env("LUM_FONT_TEST_ARTIFACT_DMCA_SANS_SERIF", &zip_path)
        .args(["font", "install", "dmca-sans-serif"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already installed"))
        .stderr(predicates::str::contains("--force"));
}

#[test]
fn font_install_force_reinstalls() {
    let home = TempDir::new().unwrap();
    let zip_path = create_test_font_zip(home.path());

    // Install first time
    lum_with_env(&home)
        .env("LUM_FONT_TEST_ARTIFACT_DMCA_SANS_SERIF", &zip_path)
        .args(["font", "install", "dmca-sans-serif"])
        .assert()
        .success();

    // Force reinstall should succeed
    lum_with_env(&home)
        .env("LUM_FONT_TEST_ARTIFACT_DMCA_SANS_SERIF", &zip_path)
        .args(["font", "install", "dmca-sans-serif", "--force"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Installed dmca-sans-serif"));
}

#[test]
fn font_uninstall_removes_font_directory() {
    let home = TempDir::new().unwrap();
    let zip_path = create_test_font_zip(home.path());

    // Install first
    lum_with_env(&home)
        .env("LUM_FONT_TEST_ARTIFACT_DMCA_SANS_SERIF", &zip_path)
        .args(["font", "install", "dmca-sans-serif"])
        .assert()
        .success();

    // Uninstall
    lum_with_env(&home)
        .args(["font", "uninstall", "dmca-sans-serif"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Uninstalled dmca-sans-serif"));

    // Font directory should be gone
    let font_dir = home
        .path()
        .join("data")
        .join("fonts")
        .join("dmca-sans-serif");
    assert!(!font_dir.exists());

    // ls should show "available" again
    lum_with_env(&home)
        .args(["font", "ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("available"));
}
