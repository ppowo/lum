use assert_cmd::Command;
use tempfile::TempDir;

fn lum_with_env(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("lum").unwrap();
    cmd.env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"));
    cmd
}

#[test]
fn env_set_persists_and_init_replays_export() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["env", "set", "openrouter", "sk-test"])
        .assert()
        .success()
        .stdout("export OPENROUTER_API_KEY='sk-test'\n");

    lum_with_env(&home)
        .args(["env", "init"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "export OPENROUTER_API_KEY='sk-test'",
        ));
}

#[test]
fn env_rejects_unknown_aliases() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["env", "set", "missing", "value"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "unknown environment alias: missing",
        ));
}

#[test]
fn env_aliases_lists_lilac_api_key() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["env", "aliases"])
        .assert()
        .success()
        .stdout(predicates::str::contains("lilac      → LILAC_API_KEY"));
}

#[test]
fn env_quotes_shell_values_safely() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["env", "set", "openrouter", "abc'def"])
        .assert()
        .success()
        .stdout("export OPENROUTER_API_KEY='abc'\\''def'\n");
}

#[test]
fn env_list_masks_secret_values_and_shows_forced_defaults() {
    use predicates::prelude::*;

    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["env", "set", "openrouter", "sk-or-v1-secret"])
        .assert()
        .success();

    lum_with_env(&home)
        .args(["env", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("OPENROUTER_API_KEY"))
        .stdout(predicates::str::contains("sk-o...cret"))
        .stdout(predicates::str::contains("npm_config_ignore_scripts"))
        .stdout(predicates::str::contains("true"))
        .stdout(predicates::str::contains("sk-or-v1-secret").not());
}

#[test]
fn env_init_can_emit_powershell_integration() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args(["env", "init", "--shell", "powershell"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "$env:npm_config_ignore_scripts = 'true'",
        ))
        .stdout(predicates::str::contains("function global:lum"))
        .stdout(predicates::str::contains(
            "lum.exe env set --shell powershell",
        ));
}

#[test]
fn env_set_and_unset_can_emit_powershell_statements() {
    let home = TempDir::new().unwrap();

    lum_with_env(&home)
        .args([
            "env",
            "set",
            "--shell",
            "powershell",
            "openrouter",
            "abc'def",
        ])
        .assert()
        .success()
        .stdout("$env:OPENROUTER_API_KEY = 'abc''def'\n");

    lum_with_env(&home)
        .args(["env", "unset", "--shell", "powershell", "openrouter"])
        .assert()
        .success()
        .stdout("Remove-Item Env:OPENROUTER_API_KEY -ErrorAction SilentlyContinue\n");
}
