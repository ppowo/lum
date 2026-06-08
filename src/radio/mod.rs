mod player;
pub mod stations;

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::cli::RadioArgs;
use crate::paths;
use crate::yt::resolve_yt_dlp;
use player::ExternalPlayer;
use stations::{Station, StationKind};

const COMMAND_HELP: &str = "Commands:\n  lum radio <code>  play a station (example: lum radio atma)\n  lum radio status  show playback state\n  lum radio stop    stop playback";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioCommand {
    List,
    Status,
    Stop,
    Play { code: String },
}

pub fn parse_command(args: &RadioArgs) -> RadioCommand {
    match args.arg.as_deref() {
        None | Some("list") => RadioCommand::List,
        Some("status") => RadioCommand::Status,
        Some("stop") => RadioCommand::Stop,
        Some(code) => RadioCommand::Play {
            code: code.to_string(),
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RadioState {
    pid: u32,
    start_time: Option<u64>,
    code: String,
    description: String,
}

pub async fn run(args: RadioArgs) -> Result<()> {
    match parse_command(&args) {
        RadioCommand::List => {
            println!("{}\n\n{}", stations::format_listing(), COMMAND_HELP);
            Ok(())
        }
        RadioCommand::Status => {
            print_status()?;
            Ok(())
        }
        RadioCommand::Stop => stop(),
        RadioCommand::Play { code } => {
            let station = stations::find(&code).with_context(|| {
                format!(
                    "station not found: {code}\n\n{}\n\n{}",
                    stations::format_listing(),
                    COMMAND_HELP
                )
            })?;
            play(*station).await
        }
    }
}

async fn play(station: Station) -> Result<()> {
    let _ = stop_existing();
    let url = playable_url(station).await?;
    let player = ExternalPlayer::start(&url).await?;
    write_state(&RadioState {
        pid: player.pid,
        start_time: player.start_time,
        code: station.code.to_string(),
        description: station.description.to_string(),
    })?;
    println!("playing {} {}", station.code, station.description);
    Ok(())
}

fn stop() -> Result<()> {
    let _ = stop_existing()?;
    println!("stopped");
    Ok(())
}

fn print_status() -> Result<()> {
    let Some(state) = read_state()? else {
        println!("stopped");
        return Ok(());
    };

    if ExternalPlayer::is_alive(state.pid, state.start_time) {
        println!("playing {} {}", state.code, state.description);
    } else {
        let _ = remove_state();
        println!("stopped");
    }

    Ok(())
}

async fn playable_url(station: Station) -> Result<String> {
    match station.kind {
        StationKind::Direct => Ok(station.url.to_string()),
        StationKind::YouTube => {
            let yt_dlp = resolve_yt_dlp().await?;
            let output = Command::new(yt_dlp)
                .args(["-g", "--no-playlist", station.url])
                .output()
                .context("failed to run yt-dlp")?;
            if !output.status.success() {
                bail!("yt-dlp failed to resolve YouTube station");
            }
            Ok(String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .context("yt-dlp produced no stream URL")?
                .to_string())
        }
    }
}

fn stop_existing() -> Result<bool> {
    let Some(state) = read_state()? else {
        return Ok(false);
    };
    ExternalPlayer::stop(state.pid, state.start_time);
    remove_state()?;
    Ok(true)
}

fn state_file() -> Result<PathBuf> {
    paths::state_dir("radio-player.json")
}

fn read_state() -> Result<Option<RadioState>> {
    let path = state_file()?;
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read radio state {}", path.display()))?;
    Ok(Some(serde_json::from_str(&data)?))
}

fn write_state(state: &RadioState) -> Result<()> {
    let path = state_file()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(state)?)
        .with_context(|| format!("failed to write radio state {}", path.display()))
}

fn remove_state() -> Result<()> {
    let path = state_file()?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::RadioArgs;

    #[test]
    fn routes_list_when_no_arg() {
        assert_eq!(parse_command(&RadioArgs { arg: None }), RadioCommand::List);
    }

    #[test]
    fn routes_status_as_command() {
        assert_eq!(
            parse_command(&RadioArgs {
                arg: Some("status".into())
            }),
            RadioCommand::Status
        );
    }

    #[test]
    fn routes_code_as_play_command() {
        assert_eq!(
            parse_command(&RadioArgs {
                arg: Some("atma".into())
            }),
            RadioCommand::Play {
                code: "atma".into()
            }
        );
    }

    #[test]
    fn pause_routes_as_play_command() {
        assert_eq!(
            parse_command(&RadioArgs {
                arg: Some("pause".into())
            }),
            RadioCommand::Play {
                code: "pause".into()
            }
        );
    }

    #[test]
    fn resume_routes_as_play_command() {
        assert_eq!(
            parse_command(&RadioArgs {
                arg: Some("resume".into())
            }),
            RadioCommand::Play {
                code: "resume".into()
            }
        );
    }
}
