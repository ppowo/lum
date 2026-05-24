pub mod args;
mod deps;

use std::path::Path;

use anyhow::Result;

use crate::cli::YtCommand;

pub async fn run(command: YtCommand) -> Result<()> {
    let yt_dlp = deps::resolve_yt_dlp().await?;

    match command {
        YtCommand::Aud { urls } => {
            let args = args::audio_args();
            let dest_dir = output_dirs::audio_dir();
            run_yt_dlp(&yt_dlp, &args, &dest_dir, &urls)
        }
        YtCommand::Vid { height, urls } => {
            check_ffmpeg()?;
            let args = args::video_args(height);
            let dest_dir = output_dirs::video_dir();
            run_yt_dlp(&yt_dlp, &args, &dest_dir, &urls)
        }
        YtCommand::Alb { urls } => {
            let args = args::album_args();
            let dest_dir = output_dirs::audio_dir();
            run_yt_dlp(&yt_dlp, &args, &dest_dir, &urls)
        }
    }
}

fn check_ffmpeg() -> Result<()> {
    if which::which("ffmpeg").is_err() {
        anyhow::bail!(
            "ffmpeg is not installed. Install it:\n  \
             macOS: brew install ffmpeg\n  \
             Linux: sudo apt install ffmpeg\n  \
             Windows: scoop install ffmpeg"
        );
    }
    Ok(())
}

fn run_yt_dlp(
    binary: &Path,
    extra_args: &[String],
    dest_dir: &Path,
    urls: &[String],
) -> Result<()> {
    std::fs::create_dir_all(dest_dir)?;

    let mut cmd = std::process::Command::new(binary);
    cmd.args(extra_args).arg("-P").arg(dest_dir).args(urls);

    // Pass through stdio so yt-dlp owns the terminal experience
    use std::process::Stdio;
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("yt-dlp exited with code {:?}", status.code());
    }
    Ok(())
}

mod output_dirs {
    use std::path::PathBuf;

    pub fn audio_dir() -> PathBuf {
        dirs::audio_dir().unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join("Music")
        })
    }

    pub fn video_dir() -> PathBuf {
        dirs::video_dir().unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join("Movies")
        })
    }
}
