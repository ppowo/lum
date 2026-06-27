mod player;
pub mod stations;

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use rand::seq::SliceRandom;
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
    #[serde(default)]
    process_kind: RadioProcessKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
enum RadioProcessKind {
    #[default]
    Ffplay,
    PlaylistRunner,
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
    let (player, process_kind) = match station.kind {
        StationKind::YouTubePlaylist => (
            ExternalPlayer::start_playlist(station.code)?,
            RadioProcessKind::PlaylistRunner,
        ),
        _ => {
            let url = playable_url(station).await?;
            (ExternalPlayer::start(&url).await?, RadioProcessKind::Ffplay)
        }
    };
    write_state(&RadioState {
        pid: player.pid,
        start_time: player.start_time,
        code: station.code.to_string(),
        description: station.description.to_string(),
        process_kind,
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

    if process_is_alive(&state) {
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
        StationKind::YouTube => resolve_youtube_stream_url(station.url).await,
        StationKind::YouTubePlaylist => bail!("playlist stations are not playable yet"),
    }
}

fn stop_existing() -> Result<bool> {
    let Some(state) = read_state()? else {
        return Ok(false);
    };
    stop_process(&state);
    remove_state()?;
    Ok(true)
}

async fn resolve_youtube_stream_url(url: &str) -> Result<String> {
    let yt_dlp = resolve_yt_dlp().await?;
    let output = Command::new(yt_dlp)
        .args(youtube_stream_url_args(url))
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

fn youtube_stream_url_args(url: &str) -> [&str; 5] {
    ["-g", "--no-playlist", "-f", "bestaudio", url]
}

pub(crate) async fn run_playlist_runner(code: String) -> Result<()> {
    let urls = randomized_playlist_urls(&code)?;
    loop {
        for (index, url) in urls.iter().enumerate() {
            let item_number = index + 1;
            let stream_url = match resolve_youtube_stream_url(url).await {
                Ok(stream_url) => stream_url,
                Err(error) => {
                    let message =
                        playlist_failure_message(&code, item_number, url, &error.to_string());
                    tracing::error!(station = %code, item = item_number, url = %url, error = %error, "{message}");
                    bail!(message);
                }
            };

            if let Err(error) = ExternalPlayer::play_until_exit(&stream_url).await {
                let message = playlist_failure_message(&code, item_number, url, &error.to_string());
                tracing::error!(station = %code, item = item_number, url = %url, error = %error, "{message}");
                bail!(message);
            }
        }
    }
}

fn randomized_playlist_urls(code: &str) -> Result<Vec<&'static str>> {
    let mut urls = stations::playlist_urls(code)
        .with_context(|| format!("unknown radio playlist station '{code}'"))?
        .to_vec();
    urls.shuffle(&mut rand::rng());
    Ok(urls)
}

fn playlist_failure_message(code: &str, item_number: usize, url: &str, error: &str) -> String {
    format!("radio playlist '{code}' failed at item {item_number} ({url}): {error}")
}

fn process_is_alive(state: &RadioState) -> bool {
    match state.process_kind {
        RadioProcessKind::Ffplay => ExternalPlayer::is_alive(state.pid, state.start_time),
        RadioProcessKind::PlaylistRunner => {
            ExternalPlayer::is_alive_any(state.pid, state.start_time)
        }
    }
}

fn stop_process(state: &RadioState) {
    match state.process_kind {
        RadioProcessKind::Ffplay => ExternalPlayer::stop(state.pid, state.start_time),
        RadioProcessKind::PlaylistRunner => ExternalPlayer::stop_any(state.pid, state.start_time),
    }
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

    #[test]
    fn legacy_state_defaults_to_ffplay_process_kind() {
        let state: RadioState = serde_json::from_str(
            r#"{"pid":123,"start_time":456,"code":"atma","description":"atma.fm Channel 1"}"#,
        )
        .unwrap();

        assert_eq!(state.process_kind, RadioProcessKind::Ffplay);
    }

    #[test]
    fn playlist_failure_message_names_station_item_and_error() {
        let message =
            playlist_failure_message("aphx", 2, "https://example.test/watch", "yt-dlp failed");
        assert!(message.contains("aphx"));
        assert!(message.contains("item 2"));
        assert!(message.contains("https://example.test/watch"));
        assert!(message.contains("yt-dlp failed"));
    }

    #[test]
    fn playlist_runner_randomizes_a_copy_of_station_urls() {
        let mut randomized = randomized_playlist_urls("aphx").unwrap();
        randomized.sort_unstable();

        let mut expected = stations::playlist_urls("aphx").unwrap().to_vec();
        expected.sort_unstable();

        assert_eq!(randomized, expected);
    }

    #[test]
    fn youtube_resolution_requests_audio_only_for_radio() {
        assert_eq!(
            youtube_stream_url_args("https://www.youtube.com/watch?v=oR4gjzXs5EE"),
            [
                "-g",
                "--no-playlist",
                "-f",
                "bestaudio",
                "https://www.youtube.com/watch?v=oR4gjzXs5EE",
            ]
        );
    }
}
