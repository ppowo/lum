use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
};

use anyhow::{Context, Result};

/// Resolve the ffmpeg binary to use.
///
/// Today this is PATH-only. Keep callers behind this seam so static ffmpeg
/// provisioning can be added here later without changing radio or yt code.
pub(crate) fn resolve() -> Result<PathBuf> {
    which::which("ffmpeg").context("ffmpeg is not installed or not on $PATH")
}

pub(crate) struct PcmStdout {
    child: Child,
    stdout: ChildStdout,
}

impl PcmStdout {
    pub(crate) fn spawn(ffmpeg: &Path, url: &str, sample_rate: u32) -> Result<Self> {
        let sample_rate = sample_rate.to_string();
        let mut child = Command::new(ffmpeg)
            .args([
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                url,
                "-vn",
                "-f",
                "f32le",
                "-acodec",
                "pcm_f32le",
                "-ac",
                "2",
                "-ar",
                &sample_rate,
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to run ffmpeg")?;

        let stdout = child.stdout.take().context("ffmpeg stdout was not piped")?;
        Ok(Self { child, stdout })
    }

    pub(crate) fn finish(mut self) -> Result<()> {
        let status = self.child.wait().context("failed to wait for ffmpeg")?;
        if !status.success() {
            anyhow::bail!("ffmpeg exited with status {status}");
        }
        Ok(())
    }

    pub(crate) fn kill(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Read for PcmStdout {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdout.read(buf)
    }
}
