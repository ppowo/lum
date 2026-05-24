use std::{fs, path::{Path, PathBuf}, time::{Duration, SystemTime}};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

pub fn init() -> Result<WorkerGuard> {
    let log_dir = log_dir()?;
    fs::create_dir_all(&log_dir).with_context(|| format!("failed to create log directory {}", log_dir.display()))?;
    cleanup_old_logs(&log_dir, SystemTime::now())?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "lum.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    tracing::info!(log_dir = %log_dir.display(), "logging initialized");
    Ok(guard)
}

pub fn log_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("dev", "ppowo", "lum")
        .context("failed to determine platform log directory")?;
    let base = dirs.state_dir().unwrap_or_else(|| dirs.data_dir());
    Ok(base.join("logs"))
}

fn cleanup_old_logs(dir: &Path, now: SystemTime) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).with_context(|| format!("failed to read log directory {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        let modified = metadata.modified()?;
        if should_delete_log_file(&path, modified, now) {
            fs::remove_file(&path).with_context(|| format!("failed to remove old log {}", path.display()))?;
        }
    }

    Ok(())
}

fn should_delete_log_file(path: &Path, modified: SystemTime, now: SystemTime) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("log")
        && now.duration_since(modified).unwrap_or_default() > RETENTION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_dir_ends_with_logs() {
        assert_eq!(log_dir().unwrap().file_name().unwrap(), "logs");
    }

    #[test]
    fn retention_deletes_only_log_files_older_than_seven_days() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10 * 24 * 60 * 60);
        let old = SystemTime::UNIX_EPOCH;
        let fresh = now - Duration::from_secs(60);

        assert!(should_delete_log_file(Path::new("lum.log"), old, now));
        assert!(!should_delete_log_file(Path::new("lum.log"), fresh, now));
        assert!(!should_delete_log_file(Path::new("notes.txt"), old, now));
    }
}
