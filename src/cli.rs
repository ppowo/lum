use crate::repos::scanner::ScanArgs;
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const GIT_COMMIT_HASH_SHORT: &str = match option_env!("LUM_GIT_COMMIT_HASH_SHORT") {
    Some(hash) => hash,
    None => "unknown",
};
const BUILD_TIME_UTC: &str = match option_env!("LUM_BUILD_TIME_UTC") {
    Some(time) => time,
    None => "unknown",
};

#[derive(Debug, Parser)]
#[command(
    name = "lum",
    long_version = long_version(),
    about = "Opinionated CLI toolbox"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

use std::sync::LazyLock;

/// Returns the long version string including commit hash and build timestamp.
pub static LONG_VERSION: LazyLock<String> = LazyLock::new(|| {
    format!(
        "{} ({}) built {}",
        PKG_VERSION, GIT_COMMIT_HASH_SHORT, BUILD_TIME_UTC
    )
});

pub fn long_version() -> &'static str {
    LONG_VERSION.as_str()
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Generate shell completions.
    #[command(name = "__completions", hide = true)]
    Completions { shell: Shell },
    /// Listen to internet radio stations.
    Radio(RadioArgs),
    /// Backup and restore directories.
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
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
    /// Manage folder-based Git identities.
    #[command(name = "git-id")]
    GitId {
        #[command(subcommand)]
        command: GitIdCommand,
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
#[command(
    after_help = "Commands:\n  lum radio                 List stations\n  lum radio <code>          Play a station (example: lum radio atma)\n  lum radio status          Show current playback state\n  lum radio stop            Stop playback and clear state"
)]
pub struct RadioArgs {
    /// Command (status|stop|list) or station code.
    ///
    /// Omit to list stations and common playback commands.
    pub arg: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum BackupCommand {
    /// Backup and restore ~/.bio.
    Bio { code: Option<String> },
    /// Backup and restore OpenEmu data.
    Openemu { code: Option<String> },
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
    #[command(visible_alias = "ls")]
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
    #[command(visible_alias = "ls")]
    List,
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
pub enum GitIdCommand {
    /// Print the path to the git identity config file.
    ConfigPath,
    /// Create a sample git identity config file if none exists.
    Init,
    /// Synchronize the machine with the git identity config.
    Sync,
    /// Show status for all configured git identities.
    Status,
    /// Show which git identity applies to the current directory.
    Where,
    /// Show detailed information about one git identity.
    Info { identity: String },
    /// Print an identity public key to stdout.
    Pubkey { identity: String },
    /// Show files and folders managed by git-id.
    Paths,
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
    #[command(visible_alias = "ls")]
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
    /// Watch mirror repos for upstream changes and send desktop notifications.
    Watch {
        /// Tag to filter repos by. Omit to see guidance.
        tag: Option<String>,
        /// Number of poll cycles to run. Test/support plumbing; omit for infinite (Ctrl+C to stop).
        #[arg(long, hide = true)]
        cycles: Option<usize>,
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
    #[command(visible_alias = "ls")]
    List,
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
            Commands::Radio(args) => assert_eq!(args.arg, None),
            Commands::Backup { .. }
            | Commands::Env { .. }
            | Commands::Completions { .. }
            | Commands::Tools { .. }
            | Commands::Repos { .. }
            | Commands::GitId { .. }
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
            Commands::Radio(args) => assert_eq!(args.arg.as_deref(), Some("atma")),
            Commands::Backup { .. }
            | Commands::Env { .. }
            | Commands::Completions { .. }
            | Commands::Tools { .. }
            | Commands::Repos { .. }
            | Commands::GitId { .. }
            | Commands::Yt { .. }
            | Commands::Font { .. }
            | Commands::Vol { .. } => {
                panic!("expected radio command")
            }
        }
    }
}
