use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
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

    pub(super) fn start_playlist(code: &str) -> Result<PlayerProcess> {
        let exe = std::env::current_exe().context("failed to resolve lum executable")?;
        let child = playlist_runner_command(exe, code)
            .spawn()
            .context("failed to start radio playlist runner")?;
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

    pub(super) fn is_alive_any(pid: u32, start_time: Option<u64>) -> bool {
        process_alive_any(pid, start_time)
    }

    pub(super) fn stop_any(pid: u32, start_time: Option<u64>) {
        kill_pid_any(pid, start_time);
    }

    pub(super) async fn play_until_exit(url: &str) -> Result<()> {
        let ffplay = ffmpeg::resolve_ffplay().await?;
        let status = ffplay_command(ffplay, url)
            .arg("-autoexit")
            .status()
            .context("failed to start ffplay")?;
        if !status.success() {
            bail!("ffplay exited with code {:?}", status.code());
        }
        Ok(())
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

fn playlist_runner_command(exe: impl AsRef<Path>, code: &str) -> Command {
    let mut command = Command::new(exe.as_ref());
    command
        .args(["__radio_playlist_runner", code])
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

fn process_alive_any(pid: u32, start_time: Option<u64>) -> bool {
    with_process(pid, |process| {
        process_start_time_matches(process.start_time(), start_time)
    })
    .unwrap_or(false)
}

fn kill_pid_any(pid: u32, start_time: Option<u64>) {
    let pid = Pid::from_u32(pid);
    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let Some(process) = system.process(pid) else {
        return;
    };
    if !process_start_time_matches(process.start_time(), start_time) {
        return;
    }

    kill_descendants(&system, pid);
    terminate_process(process);
}

fn kill_descendants(system: &System, parent: Pid) {
    let children: Vec<_> = system
        .processes()
        .iter()
        .filter_map(|(pid, process)| (process.parent() == Some(parent)).then_some(*pid))
        .collect();

    for child in children {
        kill_descendants(system, child);
        if let Some(process) = system.process(child) {
            terminate_process(process);
        }
    }
}

fn terminate_process(process: &sysinfo::Process) -> bool {
    match process.kill_with(Signal::Term) {
        Some(true) => true,
        _ => process.kill(),
    }
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
    fn playlist_runner_command_invokes_hidden_top_level_command() {
        let command = playlist_runner_command("/bin/lum", "aphx");
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();

        assert_eq!(args, ["__radio_playlist_runner", "aphx"]);
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
