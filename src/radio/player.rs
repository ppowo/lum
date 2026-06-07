use std::{
    ffi::OsStr,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};

use crate::ffmpeg;

pub(super) struct ExternalPlayer;

#[derive(Debug, Clone, Copy)]
pub(super) struct PlayerProcess {
    pub(super) pid: u32,
    pub(super) start_time: Option<u64>,
}

impl ExternalPlayer {
    pub(super) async fn start(url: &str) -> Result<PlayerProcess> {
        let ffplay = ffmpeg::resolve_ffplay().await?;
        let child = ffplay_command(ffplay, url)
            .spawn()
            .context("failed to start ffplay")?;
        let pid = child.id();
        Ok(PlayerProcess {
            pid,
            start_time: process_start_time(pid),
        })
    }

    pub(super) fn is_alive(pid: u32, start_time: Option<u64>) -> bool {
        process_alive(pid, start_time)
    }

    pub(super) fn stop(pid: u32, start_time: Option<u64>) {
        kill_pid(pid, start_time);
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

fn process_alive(pid: u32, start_time: Option<u64>) -> bool {
    with_ffplay_process(pid, start_time, |_| ()).is_some()
}

fn kill_pid(pid: u32, start_time: Option<u64>) {
    let _ = with_ffplay_process(pid, start_time, |process| {
        match process.kill_with(Signal::Term) {
            Some(true) => true,
            _ => process.kill(),
        }
    });
}

fn process_start_time(pid: u32) -> Option<u64> {
    with_process(pid, |process| process.start_time())
}

fn with_ffplay_process<R>(
    pid: u32,
    start_time: Option<u64>,
    f: impl FnOnce(&sysinfo::Process) -> R,
) -> Option<R> {
    with_process(pid, |process| {
        (is_ffplay_process(process) && process_start_time_matches(process.start_time(), start_time))
            .then(|| f(process))
    })
    .flatten()
}

fn with_process<R>(pid: u32, f: impl FnOnce(&sysinfo::Process) -> R) -> Option<R> {
    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid).map(f)
}

fn is_ffplay_process(process: &sysinfo::Process) -> bool {
    process
        .exe()
        .and_then(|path| path.file_name())
        .is_some_and(is_ffplay_name)
        || is_ffplay_name(process.name())
}

fn is_ffplay_name(name: &OsStr) -> bool {
    let name = name.to_string_lossy();
    name.eq_ignore_ascii_case("ffplay") || name.eq_ignore_ascii_case("ffplay.exe")
}

fn process_start_time_matches(actual: u64, expected: Option<u64>) -> bool {
    expected.is_none_or(|expected| actual == expected)
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

    #[test]
    fn non_ffplay_pid_is_not_considered_alive() {
        assert!(!ExternalPlayer::is_alive(std::process::id(), None));
    }

    #[test]
    fn expected_process_start_time_must_match_when_known() {
        assert!(process_start_time_matches(42, None));
        assert!(process_start_time_matches(42, Some(42)));
        assert!(!process_start_time_matches(42, Some(7)));
    }
}
