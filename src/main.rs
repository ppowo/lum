mod artifact;
mod backup;
mod cli;
mod env;
mod ffmpeg;
mod font;
mod git_id;
mod logging;
mod paths;
mod radio;
mod repos;
mod tools;
mod vol;
mod yt;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Commands::GitCredential {
        route_id,
        operation,
    } = &cli.command
    {
        if let Err(error) = git_id::run_credential_helper(route_id, operation) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    if let Err(error) = run(cli).await {
        eprintln!("{error:#}");
        tracing::error!(error = ?error, "command failed");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let _log_guard = logging::init()?;
    match cli.command {
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_owned();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }
        Commands::RadioPlaylistRunner { code } => radio::run_playlist_runner(code).await,
        Commands::GitCredential { .. } => unreachable!("credential helper handled before logging"),
        Commands::Backup { command } => backup::run(command).await,
        Commands::Radio(args) => radio::run(args).await,
        Commands::Repos { command } => repos::run(command),
        Commands::Env { command } => env::run(command),
        Commands::GitId { command } => git_id::run(command),
        Commands::Tools { command } => {
            tokio::task::spawn_blocking(move || tools::run(command)).await?
        }
        Commands::Yt { command } => yt::run(command).await,
        Commands::Font { command } => font::run(command),
        Commands::Vol { volume } => vol::run(vol::VolArgs { volume }),
    }
}
