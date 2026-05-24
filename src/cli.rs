use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "lum", version, about = "Personal CLI toolbox")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Listen to internet radio stations.
    Radio(RadioArgs),
    /// Manage shell environment variables and lum's bin path.
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },
    /// Manage curated developer tools installed into lum's bin path.
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },
}

#[derive(Debug, Args, Clone)]
pub struct RadioArgs {
    /// Station code to play. Omit to list stations.
    pub station: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum EnvShell {
    Posix,
    Powershell,
}

#[derive(Debug, Subcommand)]
pub enum EnvCommand {
    /// Print shell integration code for eval in shell startup.
    Init {
        #[arg(long, value_enum)]
        shell: Option<EnvShell>,
    },
    /// Set a managed environment variable alias.
    Set {
        #[arg(long, value_enum)]
        shell: Option<EnvShell>,
        alias: String,
        value: String,
    },
    /// Unset a managed environment variable alias.
    Unset {
        #[arg(long, value_enum)]
        shell: Option<EnvShell>,
        alias: String,
    },
    /// Show managed aliases and forced defaults.
    List,
    /// Show alias to environment variable mappings.
    Aliases,
    /// Print lum's environment bin directory.
    Path,
}

#[derive(Debug, Subcommand)]
pub enum ToolsCommand {
    /// Install a managed tool.
    Install {
        tool: String,
        #[arg(long)]
        force: bool,
    },
    /// List managed tools and local state.
    Ls,
    /// Show detailed status for one tool.
    Status { tool: String },
    /// Install missing tools and update outdated tools.
    Sync {
        #[arg(long)]
        dry_run: bool,
    },
    /// Update one managed tool.
    Update {
        tool: String,
        #[arg(long)]
        force: bool,
    },
    /// Show installed and latest version for one tool.
    Version { tool: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_radio_without_station() {
        let cli = Cli::parse_from(["lum", "radio"]);
        match cli.command {
            Commands::Radio(args) => assert_eq!(args.station, None),
            Commands::Env { .. } => panic!("expected radio command"),
            Commands::Tools { .. } => panic!("expected radio command"),
        }
    }

    #[test]
    fn parses_radio_with_station() {
        let cli = Cli::parse_from(["lum", "radio", "atma"]);
        match cli.command {
            Commands::Radio(args) => assert_eq!(args.station.as_deref(), Some("atma")),
            Commands::Env { .. } => panic!("expected radio command"),
            Commands::Tools { .. } => panic!("expected radio command"),
        }
    }
}
