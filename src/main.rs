mod cli;
mod env;
mod font;
mod logging;
mod radio;
mod repos;
mod tools;
mod yt;

use anyhow::Result;
use clap::Parser;
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
        Commands::Radio(args) => radio::run(args).await,
        Commands::Repos { command } => repos::run(command),
        Commands::Env { command } => env::run(command),
        Commands::Tools { command } => tools::run(command),
        Commands::Yt { command } => yt::run(command).await,
        Commands::Font { command } => font::run(command),
    }
}
