use std::path::Path;

use anyhow::Result;

pub(crate) fn install_executable(source: &Path, target: &Path) -> Result<()> {
    let temp = target.with_extension("lum-tmp");
    std::fs::copy(source, &temp)?;
    make_executable(&temp)?;
    replace_file(&temp, target)?;
    Ok(())
}

pub(crate) fn write_executable(target: &Path, bytes: &[u8]) -> Result<()> {
    let temp = target.with_extension("lum-tmp");
    std::fs::write(&temp, bytes)?;
    make_executable(&temp)?;
    replace_file(&temp, target)?;
    Ok(())
}

fn replace_file(temp: &Path, target: &Path) -> Result<()> {
    if cfg!(windows) && target.exists() {
        std::fs::remove_file(target)?;
    }
    std::fs::rename(temp, target).or_else(|_| {
        std::fs::copy(temp, target)?;
        std::fs::remove_file(temp)?;
        Ok::<_, std::io::Error>(())
    })?;
    Ok(())
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}
