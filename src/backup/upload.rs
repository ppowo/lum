use anyhow::{Context, Result, bail};
use reqwest::{Client, multipart};
use std::path::Path;

// Public file host used by `lum backup`. This must work with the current
// upload/restore flow:
//  1. upload via multipart form field: file=@archive.tar.gz
//  2. return a direct URL to the uploaded file
//  3. allow restore by reconstructing: <download-base>/<code>.tar.gz
//
// Current default (verified 2026-03-29):
//
//     https://x0.at
//       - closest match to 0x0-style hosts
//       - worked
//       - 512 MiB max
//       - retention: 3–100 days
//
// Other candidates to keep in mind:
//
//     https://4d2.sh
//       - worked
//       - 1024 MiB max
//       - retention: 31–730 days
//       - NOTE: upload is https://4d2.sh but returned download URLs are on
//         https://dl.4d2.sh, so switching requires separate upload/download bases
//
//     https://curl-t.com
//       - worked
//       - curl-first UX
//       - site says files are stored forever
//       - NOTE: returned URLs include /<token>/<filename>, so switching requires
//         different code extraction + restore URL construction
//
// Works, but needs code changes:
//
//     https://catbox.moe/user/api.php
//       curl -F 'reqtype=fileupload' -F 'fileToUpload=@file.tar.gz' https://catbox.moe/user/api.php
//
//     https://litterbox.catbox.moe/resources/internals/api.php
//       - temporary only: 1h / 12h / 24h / 72h
//       curl -F 'reqtype=fileupload' -F 'time=24h' -F 'fileToUpload=@file.tar.gz' https://litterbox.catbox.moe/resources/internals/api.php
pub(crate) const BACKUP_SERVICE_URL: &str = "https://x0.at";

pub(crate) fn client() -> Result<Client> {
    Client::builder()
        .user_agent(format!("lum/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to build HTTP client")
}

pub(crate) async fn upload_archive(client: &Client, archive: &Path) -> Result<String> {
    let file_name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("archive.tar.gz")
        .to_owned();
    let bytes = tokio::fs::read(archive)
        .await
        .with_context(|| format!("failed to open archive {}", archive.display()))?;
    let part = multipart::Part::bytes(bytes).file_name(file_name);
    let form = multipart::Form::new().part("file", part);
    let response = client
        .post(BACKUP_SERVICE_URL)
        .multipart(form)
        .send()
        .await
        .context("failed to upload")?
        .error_for_status()
        .context("upload failed")?;

    let url = clean_url(
        &response
            .text()
            .await
            .context("failed to read upload response")?,
    );
    if url.is_empty() {
        bail!("upload failed: empty response");
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        bail!("upload failed: invalid URL response: {url}");
    }
    Ok(url)
}

pub(crate) async fn download_archive(client: &Client, url: &str, destination: &Path) -> Result<()> {
    let bytes = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to download {url}"))?
        .error_for_status()
        .with_context(|| format!("failed to download {url}"))?
        .bytes()
        .await
        .with_context(|| format!("failed to read downloaded archive from {url}"))?;
    tokio::fs::write(destination, bytes)
        .await
        .with_context(|| {
            format!(
                "failed to write downloaded archive to {}",
                destination.display()
            )
        })?;
    Ok(())
}

pub(crate) fn restore_url(code: &str) -> String {
    format!("{BACKUP_SERVICE_URL}/{code}.tar.gz")
}

pub(crate) fn code_from_url(url: &str) -> Result<String> {
    let path = reqwest::Url::parse(url)
        .with_context(|| format!("failed to parse upload URL {url}"))?
        .path()
        .to_owned();
    let file_name = Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("upload failed: URL has no file name: {url}"))?;
    file_name
        .strip_suffix(".tar.gz")
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("upload failed: URL did not end in .tar.gz: {url}"))
}

fn clean_url(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .collect::<String>()
        .trim()
        .to_owned()
}
