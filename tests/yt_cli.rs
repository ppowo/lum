use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use tempfile::TempDir;

fn lum_with_empty_path(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("PATH", ""); // ensure yt-dlp and ffmpeg are not found on PATH
    cmd
}

/// Create a fake yt-dlp binary on PATH so we can test ffmpeg check.
fn lum_with_fake_ytdlp(home: &TempDir) -> Command {
    let bin_dir = home.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    // Write a script that exits 0 so `which` finds it
    let yt_dlp_path = bin_dir.join("yt-dlp");
    #[cfg(unix)]
    {
        std::fs::write(&yt_dlp_path, "#!/bin/sh\nexit 0\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&yt_dlp_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("PATH", &bin_dir);
    cmd
}

fn lum_with_fake_ffmpeg_and_ytdlp_artifact(home: &TempDir) -> Command {
    let bin_dir = home.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ffmpeg_path = bin_dir.join("ffmpeg");
    let ytdlp_artifact = home.path().join("yt-dlp-artifact");
    #[cfg(unix)]
    {
        std::fs::write(&ffmpeg_path, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::write(&ytdlp_artifact, "#!/bin/sh\nexit 0\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&ffmpeg_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::set_permissions(&ytdlp_artifact, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("PATH", &bin_dir)
        .env("LUM_YT_DLP_TEST_ARTIFACT", &ytdlp_artifact);
    cmd
}

#[test]
fn yt_aud_auto_provisions_yt_dlp_when_missing() {
    let home = TempDir::new().unwrap();

    lum_with_fake_ffmpeg_and_ytdlp_artifact(&home)
        .args(["yt", "aud", "https://example.com/video"])
        .assert()
        .success();

    let deps_ytdlp = home
        .path()
        .join("data")
        .join("lum")
        .join("deps")
        .join("yt-dlp");
    let deps_state = home
        .path()
        .join("data")
        .join("lum")
        .join("deps")
        .join("yt-dlp.json");
    assert!(deps_ytdlp.exists());
    assert!(deps_state.exists());
}

#[test]
fn yt_aud_requires_at_least_one_url() {
    let home = TempDir::new().unwrap();

    lum_with_empty_path(&home)
        .args(["yt", "aud"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("required"));
}

#[test]
fn yt_vid_requires_at_least_one_url() {
    let home = TempDir::new().unwrap();

    lum_with_empty_path(&home)
        .args(["yt", "vid"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("required"));
}

#[test]
fn yt_alb_requires_at_least_one_url() {
    let home = TempDir::new().unwrap();

    lum_with_empty_path(&home)
        .args(["yt", "alb"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("required"));
}

#[test]
fn yt_rejects_unknown_subcommand() {
    let home = TempDir::new().unwrap();

    lum_with_empty_path(&home)
        .args(["yt", "download"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("download").or(predicates::str::contains("subcommand")));
}

#[test]
fn yt_aud_does_not_require_ffmpeg() {
    let home = TempDir::new().unwrap();

    // yt-dlp is available (fake), but ffmpeg is not. Audio should still run.
    lum_with_fake_ytdlp(&home)
        .args(["yt", "aud", "https://example.com/video"])
        .assert()
        .success();
}

#[test]
fn yt_vid_fails_when_ffmpeg_not_found() {
    let home = TempDir::new().unwrap();

    // yt-dlp is available (fake), but ffmpeg is not. Video needs ffmpeg for muxing.
    lum_with_fake_ytdlp(&home)
        .args(["yt", "vid", "https://example.com/video"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("ffmpeg"));
}

#[test]
fn yt_vid_accepts_height_flag() {
    let home = TempDir::new().unwrap();

    lum_with_fake_ytdlp(&home)
        .args(["yt", "vid", "--height", "2160", "https://example.com/video"])
        .assert()
        .failure()
        // Should fail because ffmpeg missing, not because --height is invalid
        .stderr(predicates::str::contains("ffmpeg"));
}
