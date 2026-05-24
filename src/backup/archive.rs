use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};

use super::target::{BackupTarget, GLOBAL_EXCLUDE_GLOBS};

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

pub(crate) fn create_tar_gz(target: BackupTarget, home: &Path, output: &Path) -> Result<()> {
    let exclude_set = exclude_set(target)?;
    let target_path = target.target_path(home);
    let tar_gz =
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?;
    let encoder = GzEncoder::new(tar_gz, Compression::default());
    let mut builder = Builder::new(encoder);

    let mut walker = WalkBuilder::new(&target_path);
    walker
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false);

    for entry in walker.build() {
        let entry = entry.with_context(|| format!("failed to walk {}", target_path.display()))?;
        let path = entry.path();
        let archive_path = path
            .strip_prefix(home)
            .with_context(|| format!("failed to make archive path for {}", path.display()))?;

        if should_exclude(path, archive_path, &exclude_set) {
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }
            continue;
        }

        builder
            .append_path_with_name(path, archive_path)
            .with_context(|| format!("failed to add {} to archive", archive_path.display()))?;
    }

    let encoder = builder
        .into_inner()
        .context("failed to finish tar archive")?;
    encoder.finish().context("failed to finish gzip archive")?;
    Ok(())
}

pub(crate) fn verify_tar_gz(path: &Path, target: BackupTarget) -> Result<()> {
    verify_gzip_magic(path)?;
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let mut found_target = false;

    for entry in archive.entries().context("failed to read tar entries")? {
        let entry = entry.context("failed to read tar entry")?;
        let entry_path = entry.path().context("failed to read tar entry path")?;
        if entry_path == Path::new(target.path) || entry_path.starts_with(target.path) {
            found_target = true;
        }
    }

    if !found_target {
        bail!("archive does not contain a {} directory", target.path);
    }

    Ok(())
}

pub(crate) fn extract_tar_gz(path: &Path, destination: &Path) -> Result<()> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive
        .unpack(destination)
        .with_context(|| format!("failed to extract archive to {}", destination.display()))?;
    Ok(())
}

fn verify_gzip_magic(path: &Path) -> Result<()> {
    use std::io::Read;

    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut magic = [0_u8; 2];
    file.read_exact(&mut magic)
        .context("file too small to be a valid tar.gz")?;
    if magic != GZIP_MAGIC {
        bail!("file is not a valid gzip archive (wrong magic bytes)");
    }
    Ok(())
}

fn exclude_set(target: BackupTarget) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in GLOBAL_EXCLUDE_GLOBS.iter().chain(target.excludes.iter()) {
        add_pattern(&mut builder, pattern)?;
    }
    builder.build().context("failed to build exclude glob set")
}

fn add_pattern(builder: &mut GlobSetBuilder, pattern: &str) -> Result<()> {
    add_glob(builder, pattern)?;
    add_glob(builder, &format!("{pattern}/**"))?;
    if !pattern.contains('/') {
        add_glob(builder, &format!("**/{pattern}"))?;
        add_glob(builder, &format!("**/{pattern}/**"))?;
    }
    Ok(())
}

fn add_glob(builder: &mut GlobSetBuilder, pattern: &str) -> Result<()> {
    builder.add(Glob::new(pattern).with_context(|| format!("invalid exclude glob {pattern}"))?);
    Ok(())
}

fn should_exclude(path: &Path, archive_path: &Path, exclude_set: &GlobSet) -> bool {
    exclude_set.is_match(archive_path)
        || path
            .file_name()
            .map(PathBuf::from)
            .is_some_and(|name| exclude_set.is_match(name))
}
