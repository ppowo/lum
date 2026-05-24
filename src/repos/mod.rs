pub mod mirror;
pub mod scanner;

use anyhow::Result;

use crate::cli::ReposCommand;

pub fn run(command: ReposCommand) -> Result<()> {
    match command {
        ReposCommand::Scan(args) => scanner::run(&args),
        ReposCommand::Mirror { command } => mirror::run(command),
    }
}
