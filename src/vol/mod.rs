use anyhow::Result;

/// Returns the OS-specific default reset volume (0–100).
pub(crate) const fn default_volume() -> u8 {
    #[cfg(target_os = "macos")]
    {
        17
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        25
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        25
    }
}

pub struct VolArgs {
    pub volume: Option<u16>,
}

pub fn run(args: VolArgs) -> Result<()> {
    let target = match args.volume {
        Some(v) => {
            if v > 100 {
                anyhow::bail!("volume must be between 0 and 100");
            }
            v as u8
        }
        None => default_volume(),
    };
    let is_default = args.volume.is_none();

    let device = volumecontrol::AudioDevice::from_default()?;
    let previous = device.get_vol().ok();

    device.set_vol(target)?;

    match (is_default, previous) {
        (true, Some(prev)) => println!("Volume set to {} (default, was {})", target, prev),
        (true, None) => println!("Volume set to {} (default)", target),
        (false, Some(prev)) => println!("Volume set to {} (was {})", target, prev),
        (false, None) => println!("Volume set to {}", target),
    }

    Ok(())
}
