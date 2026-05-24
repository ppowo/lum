use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use tempfile::TempDir;

fn lum_with_home(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .env("XDG_DATA_HOME", home.path().join(".local/share"));
    cmd
}

#[test]
fn init_creates_sample_git_identity_config() {
    let home = TempDir::new().unwrap();

    lum_with_home(&home)
        .args(["git-id", "init"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Created"));

    let config_path = home.path().join(".config/lum/git-identities.json");
    let config = std::fs::read_to_string(config_path).unwrap();

    assert!(config.contains("\"identities\""));
    assert!(config.contains("\"name\""));
    assert!(config.contains("\"author_name\""));
    assert!(config.contains("\"folders\""));
}

#[test]
fn config_path_prints_git_identity_config_location() {
    let home = TempDir::new().unwrap();

    lum_with_home(&home)
        .args(["git-id", "config-path"])
        .assert()
        .success()
        .stdout(predicates::str::contains("git-identities.json"));
}

#[test]
fn where_uses_most_specific_managed_folder() {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join(".config/lum");
    std::fs::create_dir_all(&config_dir).unwrap();
    let work = home.path().join("Work");
    let github = work.join("Github");
    let repo = github.join("project");
    std::fs::create_dir_all(&repo).unwrap();

    let config = format!(
        r#"{{
  "identities": [
    {{"name":"work","author_name":"Work User","email":"work@example.com","domain":"github.com","folders":["{}"]}},
    {{"name":"github-work","author_name":"Github User","email":"github@example.com","domain":"github.com","folders":["{}"]}}
  ]
}}"#,
        work.display(),
        github.display()
    );
    std::fs::write(config_dir.join("git-identities.json"), config).unwrap();

    let mut cmd = lum_with_home(&home);
    cmd.args(["git-id", "where"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicates::str::contains("github-work"))
        .stdout(predicates::str::contains("Work User").not());
}

#[test]
fn sync_creates_managed_artifacts_from_config() {
    let home = TempDir::new().unwrap();
    let bin = home.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let fake_ssh_keygen = bin.join("ssh-keygen");
    std::fs::write(
        &fake_ssh_keygen,
        "#!/bin/sh\nif [ \"$1\" = \"-h\" ]; then exit 0; fi\nkey=\"\"\ncomment=\"\"\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    -f) shift; key=\"$1\" ;;\n    -C) shift; comment=\"$1\" ;;\n  esac\n  shift\ndone\nprintf 'PRIVATE %s\n' \"$comment\" > \"$key\"\nprintf 'ssh-ed25519 TESTKEY %s\n' \"$comment\" > \"$key.pub\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&fake_ssh_keygen).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_ssh_keygen, perms).unwrap();
    }

    let config_dir = home.path().join(".config/lum");
    std::fs::create_dir_all(&config_dir).unwrap();
    let managed = home.path().join("Work/Github");
    let config = format!(
        r#"{{"identities":[{{"name":"github-work","author_name":"Jane Doe","email":"jane@company.com","domain":"github.com","folders":["{}"]}}]}}"#,
        managed.display()
    );
    std::fs::write(config_dir.join("git-identities.json"), config).unwrap();
    std::fs::create_dir_all(home.path().join(".ssh")).unwrap();
    std::fs::write(
        home.path().join(".ssh/config"),
        "Host existing\n  User git\n",
    )
    .unwrap();

    let mut cmd = lum_with_home(&home);
    cmd.env(
        "PATH",
        format!(
            "{}:{}",
            bin.display(),
            std::env::var("PATH").unwrap_or_default()
        ),
    )
    .args(["git-id", "sync"])
    .assert()
    .success()
    .stdout(predicates::str::contains("github-work"));

    assert!(managed.exists());
    let public_key =
        std::fs::read_to_string(home.path().join(".ssh/lum-git-id-github-work.pub")).unwrap();
    assert!(public_key.contains("[lum:git-id identity=github-work]"));
    let git_config =
        std::fs::read_to_string(home.path().join(".gitconfig-lum-git-id-github-work")).unwrap();
    assert!(git_config.contains("# lum:git-id:managed identity=github-work"));
    assert!(git_config.contains("signingkey"));
    assert!(git_config.contains("insteadOf = https://github.com/"));
    let global_git_config = std::fs::read_to_string(home.path().join(".gitconfig")).unwrap();
    assert!(global_git_config.contains("# lum:git-id:begin"));
    assert!(global_git_config.contains("includeIf"));
    assert!(!global_git_config.starts_with('\n'));
    let ssh_config = std::fs::read_to_string(home.path().join(".ssh/config")).unwrap();
    assert!(ssh_config.contains("Host existing\n  User git\n\n# lum:git-id:begin"));
    let allowed_signers =
        std::fs::read_to_string(home.path().join(".ssh/allowed_signers")).unwrap();
    assert!(!allowed_signers.starts_with('\n'));
}
