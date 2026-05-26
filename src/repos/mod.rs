pub mod mirror;
pub mod scanner;

use anyhow::{Context, Result};

use crate::cli::ReposCommand;

pub fn run(command: ReposCommand) -> Result<()> {
    match command {
        ReposCommand::Scan(args) => scanner::run(&args),
        ReposCommand::Mirror { command } => mirror::run(command),
    }
}

pub(crate) fn ensure_git_on_path() -> Result<()> {
    which::which("git").context("git executable not found on PATH")?;
    Ok(())
}
