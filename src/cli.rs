use clap::{Args, Parser, Subcommand};

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
}

#[derive(Debug, Args, Clone)]
pub struct RadioArgs {
    /// Station code to play. Omit to list stations.
    pub station: Option<String>,
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
        }
    }

    #[test]
    fn parses_radio_with_station() {
        let cli = Cli::parse_from(["lum", "radio", "atma"]);
        match cli.command {
            Commands::Radio(args) => assert_eq!(args.station.as_deref(), Some("atma")),
        }
    }
}
