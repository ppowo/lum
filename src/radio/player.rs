use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::ffmpeg;

pub(super) struct ExternalPlayer;

impl ExternalPlayer {
    pub(super) async fn start(url: &str) -> Result<u32> {
        let ffplay = ffmpeg::resolve_ffplay().await?;
        let child = ffplay_command(ffplay, url)
            .spawn()
            .context("failed to start ffplay")?;
        Ok(child.id())
    }

    pub(super) fn is_alive(pid: u32) -> bool {
        process_alive(pid)
    }

    pub(super) fn stop(pid: u32) {
        kill_pid(pid);
    }
}

fn ffplay_command(ffplay: impl AsRef<std::ffi::OsStr>, url: &str) -> Command {
    let mut command = Command::new(ffplay);
    command
        .args(["-nodisp", "-hide_banner", "-loglevel", "error", url])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}")])
        .stdin(Stdio::null())
        .output()
        .is_ok_and(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
        })
}

#[cfg(unix)]
fn kill_pid(pid: u32) {
    let _ = Command::new("kill")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(windows)]
fn kill_pid(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffplay_command_uses_quiet_detached_audio_args() {
        let command = ffplay_command("ffplay", "https://example.test/stream");
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();

        assert_eq!(
            args,
            [
                "-nodisp",
                "-hide_banner",
                "-loglevel",
                "error",
                "https://example.test/stream"
            ]
        );
    }
}
