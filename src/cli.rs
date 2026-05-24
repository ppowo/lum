use crate::repos::scanner::ScanArgs;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "lum", version, about = "Opinionated CLI toolbox")]
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
    /// Scan directory trees for Git repositories and report status.
    Repos {
        #[command(subcommand)]
        command: ReposCommand,
    },
    /// Manage curated developer tools installed into lum's bin path.
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },
    /// Download audio, video, or albums from YouTube using yt-dlp.
    Yt {
        #[command(subcommand)]
        command: YtCommand,
    },
    /// Install and manage fonts.
    Font {
        #[command(subcommand)]
        command: FontCommand,
    },
    /// Set system volume to default or specified level.
    Vol {
        /// Volume level (0–100). Omit to reset to OS default.
        volume: Option<u16>,
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

#[derive(Debug, Subcommand)]
pub enum ReposCommand {
    /// Scan a directory tree for Git repositories and report branch and sync status.
    Scan(ScanArgs),
    /// Clone, update, and inspect configured mirror repositories.
    Mirror {
        #[command(subcommand)]
        command: MirrorCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum MirrorCommand {
    /// Print the path to the mirror config file.
    ConfigPath,
    /// Print the path to the mirror directory.
    Dir,
    /// Create a sample mirror config file if none exists.
    Init,
    /// List configured mirror repositories.
    List,
    /// Clone or update all configured mirror repositories.
    Sync {
        /// Maximum concurrent git operations.
        #[arg(short = 'j', default_value = "4")]
        jobs: usize,
    },
    /// Check if local mirrors are up to date with their remotes.
    Status {
        /// Maximum concurrent git operations.
        #[arg(short = 'j', default_value = "4")]
        jobs: usize,
        /// Compare against cached remote refs instead of contacting remotes.
        #[arg(long)]
        offline: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum YtCommand {
    /// Download audio from YouTube URL(s).
    Aud {
        /// YouTube URL(s) to download.
        #[arg(required = true)]
        urls: Vec<String>,
    },
    /// Download video from YouTube URL(s).
    Vid {
        /// Maximum video height (default: 1080).
        #[arg(long)]
        height: Option<u32>,
        /// YouTube URL(s) to download.
        #[arg(required = true)]
        urls: Vec<String>,
    },
    /// Download an album or playlist from YouTube URL(s).
    Alb {
        /// YouTube URL(s) to download.
        #[arg(required = true)]
        urls: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum FontCommand {
    /// List managed fonts and local state.
    Ls,
    /// Install a managed font.
    Install {
        font: String,
        #[arg(long)]
        force: bool,
    },
    /// Uninstall a managed font.
    Uninstall { font: String },
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
            Commands::Env { .. }
            | Commands::Tools { .. }
            | Commands::Repos { .. }
            | Commands::Yt { .. }
            | Commands::Font { .. }
            | Commands::Vol { .. } => {
                panic!("expected radio command")
            }
        }
    }

    #[test]
    fn parses_radio_with_station() {
        let cli = Cli::parse_from(["lum", "radio", "atma"]);
        match cli.command {
            Commands::Radio(args) => assert_eq!(args.station.as_deref(), Some("atma")),
            Commands::Env { .. }
            | Commands::Tools { .. }
            | Commands::Repos { .. }
            | Commands::Yt { .. }
            | Commands::Font { .. }
            | Commands::Vol { .. } => {
                panic!("expected radio command")
            }
        }
    }
}
