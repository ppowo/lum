mod backup;
mod cli;
mod env;
mod font;
mod git_id;
mod logging;
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
    if let Err(error) = run().await {
        eprintln!("{error:#}");
        tracing::error!(error = ?error, "command failed");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let _log_guard = logging::init()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_owned();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }
        Commands::Backup { command } => backup::run(command),
        Commands::Radio(args) => radio::run(args).await,
        Commands::Repos { command } => repos::run(command),
        Commands::Env { command } => env::run(command),
        Commands::GitId { command } => git_id::run(command),
        Commands::Tools { command } => tools::run(command),
        Commands::Yt { command } => yt::run(command).await,
        Commands::Font { command } => font::run(command),
        Commands::Vol { volume } => vol::run(vol::VolArgs { volume }),
    }
}
