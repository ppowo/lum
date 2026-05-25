use anyhow::{Context, Result};
mod archive;
mod restore;
mod target;
mod upload;

use crate::cli::BackupCommand;

pub async fn run(command: BackupCommand) -> Result<()> {
    match command {
        BackupCommand::Bio { code } => run_target(target::BIO, code).await,
        BackupCommand::Openemu { code } => run_target(target::OPENEMU, code).await,
    }
}

async fn run_target(target: target::BackupTarget, code: Option<String>) -> Result<()> {
    target.ensure_current_os_allowed()?;
    match code {
        Some(code) => restore::restore_backup(target, &code).await,
        None => upload_target(target).await,
    }
}

async fn upload_target(target: target::BackupTarget) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("failed to get home directory"))?;
    let target_path = target.target_path(&home);
    if !target_path.exists() {
        anyhow::bail!(
            "{} directory not found at {}",
            target.name,
            target_path.display()
        );
    }

    println!("{} - Starting {} backup", timestamp(), target.name);
    println!(
        "This will archive your ~/{} and upload it for backup/sharing",
        target.path
    );
    println!();
    println!(
        "{} - Found {} directory at {}",
        timestamp(),
        target.name,
        target_path.display()
    );

    let archive_file = tempfile::Builder::new()
        .prefix(&format!("{}-backup-", target.name))
        .suffix(".tar.gz")
        .tempfile()?;
    let archive_path = archive_file.path().to_owned();

    println!("{} - Creating compressed archive...", timestamp());
    let create_target = target;
    let create_home = home.clone();
    let create_archive_path = archive_path.clone();
    tokio::task::spawn_blocking(move || {
        archive::create_tar_gz(create_target, &create_home, &create_archive_path)
    })
    .await
    .context("archive creation task failed")??;
    let size_mb = archive_file.as_file().metadata()?.len() as f64 / (1024.0 * 1024.0);
    println!(
        "{} - Archive created successfully (size: {:.2} MB)",
        timestamp(),
        size_mb
    );

    let client = upload::client()?;
    println!("{} - Uploading...", timestamp());
    let url = upload::upload_archive(&client, &archive_path).await?;

    println!("{} - Verifying upload...", timestamp());
    let verify_file = tempfile::Builder::new()
        .prefix(&format!("{}-verify-", target.name))
        .suffix(".tar.gz")
        .tempfile()?;
    upload::download_archive(&client, &url, verify_file.path()).await?;
    let verify_path = verify_file.path().to_owned();
    tokio::task::spawn_blocking(move || archive::verify_tar_gz(&verify_path, target))
        .await
        .context("archive verification task failed")?
        .context(
            "upload verification failed; received file may be an error page instead of archive",
        )?;

    println!("{} - Upload verified successfully!", timestamp());
    println!(
        "{} - Your {} backup is available at:",
        timestamp(),
        target.name
    );
    println!("{url}");
    let code = upload::code_from_url(&url)?;
    println!(
        "{} - Restore with: lum backup {} {}",
        timestamp(),
        target.name,
        code
    );
    println!("{} - Temporary archive removed.", timestamp());
    Ok(())
}

fn timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}
