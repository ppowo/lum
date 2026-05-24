use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;

use crate::cli::{EnvCommand, EnvShell};

const ALIASES: &[(&str, &str)] = &[
    ("exa", "EXA_API_KEY"),
    ("neuralwatt", "NEURALWATT_API_KEY"),
    ("openrouter", "OPENROUTER_API_KEY"),
    ("synthetic", "SYNTHETIC_API_KEY"),
];

const FORCED_ENV: &[(&str, &str)] = &[
    ("PI_HASHLINE_GREP_MAX_LINES", "150"),
    ("PI_HASHLINE_GREP_MAX_BYTES", "10000"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD", "1"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD_MAX_LINES", "400"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD_MAX_BYTES", "25000"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD_HEAD_LINES", "60"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD_TAIL_LINES", "150"),
    ("npm_config_ignore_scripts", "true"),
];

pub fn run(command: EnvCommand) -> Result<()> {
    match command {
        EnvCommand::Init { shell } => init(shell.unwrap_or_default()),
        EnvCommand::Set {
            shell,
            alias,
            value,
        } => set(&alias, &value, shell.unwrap_or_default()),
        EnvCommand::Unset { shell, alias } => unset(&alias, shell.unwrap_or_default()),
        EnvCommand::Path => {
            println!("{}", bin_dir()?.display());
            Ok(())
        }
        EnvCommand::List => list(),
        EnvCommand::Aliases => {
            for (alias, variable) in ALIASES {
                println!("{alias:<10} → {variable}");
            }
            Ok(())
        }
    }
}

impl Default for EnvShell {
    fn default() -> Self {
        if cfg!(windows) {
            Self::Powershell
        } else {
            Self::Posix
        }
    }
}

fn init(shell: EnvShell) -> Result<()> {
    let bin = bin_dir()?;
    fs::create_dir_all(&bin)
        .with_context(|| format!("failed to create bin directory {}", bin.display()))?;

    let state = read_state()?;
    match shell {
        EnvShell::Posix => emit_posix_init(&state, &bin),
        EnvShell::Powershell => emit_powershell_init(&state, &bin),
    }
    Ok(())
}

fn emit_posix_init(state: &BTreeMap<String, String>, bin: &std::path::Path) {
    for (alias, value) in state {
        if let Some(variable) = variable_for_alias(alias) {
            println!("export {variable}={}", shell_quote(value));
        }
    }
    for (name, value) in FORCED_ENV {
        println!("export {name}={}", shell_quote(value));
    }

    let quoted_bin = shell_quote(&bin.to_string_lossy());
    println!(
        r#"case ":$PATH:" in
  *":{bin}:"*) ;;
  *) export PATH={quoted_bin}:$PATH ;;
esac
lum() {{
  if [ "$1" = "env" ]; then
    case "$2" in
      set|unset)
        eval "$(command lum "$@")"
        ;;
      *)
        command lum "$@"
        ;;
    esac
  else
    command lum "$@"
  fi
}}"#,
        bin = bin.display(),
        quoted_bin = quoted_bin
    );
}

fn emit_powershell_init(state: &BTreeMap<String, String>, bin: &std::path::Path) {
    for (alias, value) in state {
        if let Some(variable) = variable_for_alias(alias) {
            println!("$env:{variable} = {}", powershell_quote(value));
        }
    }
    for (name, value) in FORCED_ENV {
        println!("$env:{name} = {}", powershell_quote(value));
    }
    let bin = powershell_quote(&bin.to_string_lossy());
    println!(
        r#"if (($env:PATH -split ';') -notcontains {bin}) {{
  $env:PATH = {bin} + ';' + $env:PATH
}}
function global:lum {{
  if (($args.Count -ge 2) -and ($args[0] -eq 'env') -and ($args[1] -eq 'set')) {{
    Invoke-Expression (& lum.exe env set --shell powershell @($args | Select-Object -Skip 2))
  }} elseif (($args.Count -ge 2) -and ($args[0] -eq 'env') -and ($args[1] -eq 'unset')) {{
    Invoke-Expression (& lum.exe env unset --shell powershell @($args | Select-Object -Skip 2))
  }} else {{
    & lum.exe @args
  }}
}}"#,
        bin = bin
    );
}

fn set(alias: &str, value: &str, shell: EnvShell) -> Result<()> {
    let variable =
        variable_for_alias(alias).with_context(|| format!("unknown environment alias: {alias}"))?;
    let mut state = read_state()?;
    state.insert(alias.to_owned(), value.to_owned());
    write_state(&state)?;
    match shell {
        EnvShell::Posix => println!("export {variable}={}", shell_quote(value)),
        EnvShell::Powershell => println!("$env:{variable} = {}", powershell_quote(value)),
    }
    eprintln!("[lum env] Set {alias} ({variable})");
    Ok(())
}

fn unset(alias: &str, shell: EnvShell) -> Result<()> {
    let variable =
        variable_for_alias(alias).with_context(|| format!("unknown environment alias: {alias}"))?;
    let mut state = read_state()?;
    state.remove(alias);
    write_state(&state)?;
    match shell {
        EnvShell::Posix => println!("unset {variable}"),
        EnvShell::Powershell => {
            println!("Remove-Item Env:{variable} -ErrorAction SilentlyContinue")
        }
    }
    eprintln!("[lum env] Unset {alias} ({variable})");
    Ok(())
}

fn list() -> Result<()> {
    let state = read_state()?;
    println!("Aliases (set with 'lum env set <alias> <value>'):");
    for (alias, variable) in ALIASES {
        if let Some(value) = state.get(*alias) {
            println!("  {alias:<10} {variable:<24} = {}", mask_secret(value));
        } else {
            println!("  {alias:<10} {variable:<24}   (not set)");
        }
    }
    println!();
    println!("Forced defaults (auto-set, not configurable):");
    for (name, value) in FORCED_ENV {
        println!("  {name:<45} {value}");
    }
    Ok(())
}

fn variable_for_alias(alias: &str) -> Option<&'static str> {
    ALIASES
        .iter()
        .find_map(|(key, variable)| (*key == alias).then_some(*variable))
}

fn dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("dev", "ppowo", "lum").context("failed to determine platform directories")
}

fn state_path() -> Result<PathBuf> {
    Ok(dirs()?.config_dir().join("env-state.json"))
}

fn bin_dir() -> Result<PathBuf> {
    Ok(dirs()?.data_dir().join("bin"))
}

fn read_state() -> Result<BTreeMap<String, String>> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let data =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&data).with_context(|| format!("failed to parse {}", path.display()))
}

fn write_state(state: &BTreeMap<String, String>) -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let data = serde_json::to_string_pretty(state)?;
    fs::write(&path, data).with_context(|| format!("failed to write {}", path.display()))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn mask_secret(value: &str) -> String {
    if value.len() <= 8 {
        "********".to_owned()
    } else {
        format!("{}...{}", &value[..4], &value[value.len() - 4..])
    }
}
