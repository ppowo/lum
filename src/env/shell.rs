use std::collections::BTreeMap;

use crate::cli::EnvShell;

use super::catalog::{FORCED_ENV, variable_for_alias};

pub(crate) fn emit_posix_init(state: &BTreeMap<String, String>, bin: &std::path::Path) {
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
}}

# --- lum shell completion ---
if [ -n "$BASH_VERSION" ]; then
  eval "$(command lum __completions bash)"
elif [ -n "$ZSH_VERSION" ]; then
  if ! command -v compdef >/dev/null 2>&1; then
    autoload -Uz compinit && compinit -i
  fi
  eval "$(command lum __completions zsh)"
fi
# --- end lum shell completion ---"#,
        bin = bin.display(),
        quoted_bin = quoted_bin
    );
}

pub(crate) fn emit_powershell_init(state: &BTreeMap<String, String>, bin: &std::path::Path) {
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
}}

# --- lum shell completion ---
Invoke-Expression (& lum.exe __completions powershell | Out-String)
# --- end lum shell completion ---"#,
        bin = bin
    );
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

pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

pub(crate) fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
