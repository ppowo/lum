mod catalog;
mod install;
mod state;

use anyhow::Result;

use crate::cli::AppsCommand;

pub fn run(command: AppsCommand) -> Result<()> {
    if !cfg!(target_os = "macos") {
        anyhow::bail!("lum apps is only supported on macOS");
    }
    match command {
        AppsCommand::List => list(),
        AppsCommand::Install { app, force } => install_cmd(&app, force),
        AppsCommand::Status { app } => status_cmd(&app),
        AppsCommand::Update { app, force } => update_cmd(&app, force),
        AppsCommand::Sync { dry_run } => sync_cmd(dry_run),
        AppsCommand::Version { app } => version_cmd(&app),
    }
}

fn version_cmd(app: &str) -> Result<()> {
    let spec = catalog::lookup_app(app)?;
    let status = state::local_status(spec)?;
    let artifact = install::resolve_latest(spec)?;
    println!(
        "Installed: {}",
        status.effective_version().unwrap_or("unknown")
    );
    println!("Latest:    {}", artifact.version);
    Ok(())
}

fn sync_cmd(dry_run: bool) -> Result<()> {
    let mut installed = 0;
    let mut updated = 0;
    let mut up_to_date = 0;
    let mut skipped = 0;
    for spec in catalog::CATALOG {
        let status = state::local_status(spec)?;
        if status.exists && !status.managed {
            println!(
                "\u{2022} {}: unmanaged at {}; rerun 'lum apps install {} --force' to take over",
                spec.name,
                status.path.display(),
                spec.name
            );
            skipped += 1;
            continue;
        }
        ensure_no_foreign_copies(spec)?;
        let artifact = install::resolve_latest(spec)?;
        match status.effective_version() {
            None if dry_run => {
                println!("\u{2022} {}: would install {}", spec.name, artifact.version)
            }
            None => {
                print!(
                    "\u{2022} {}: installing {}... ",
                    spec.name, artifact.version
                );
                std::io::Write::flush(&mut std::io::stdout())?;
                install::install_app(spec, &artifact, None)?;
                println!("done");
                installed += 1;
            }
            Some(current) if current != artifact.version.as_str() && dry_run => {
                println!(
                    "\u{2022} {}: would update {} -> {}",
                    spec.name, current, artifact.version
                )
            }
            Some(current) if current != artifact.version.as_str() => {
                print!(
                    "\u{2022} {}: updating {} -> {}... ",
                    spec.name, current, artifact.version
                );
                std::io::Write::flush(&mut std::io::stdout())?;
                install::install_app(spec, &artifact, state::installed_at(spec)?)?;
                println!("done");
                updated += 1;
            }
            Some(current) => {
                println!("\u{2022} {}: up to date ({})", spec.name, current);
                up_to_date += 1;
            }
        }
    }
    println!(
        "\nSummary: {installed} installed, {updated} updated, {up_to_date} up to date, {skipped} skipped"
    );
    Ok(())
}

fn update_cmd(app: &str, force: bool) -> Result<()> {
    let spec = catalog::lookup_app(app)?;
    let status = state::local_status(spec)?;
    if !status.managed {
        anyhow::bail!(
            "{} is not installed; use 'lum apps install {}' first",
            spec.name,
            spec.name
        );
    }
    ensure_no_foreign_copies(spec)?;
    let artifact = install::resolve_latest(spec)?;
    let previous = status.effective_version().unwrap_or("unknown").to_owned();
    if !force && previous != "unknown" && previous == artifact.version {
        println!("{} is already up to date ({})", spec.name, previous);
        return Ok(());
    }
    let updated = install::install_app(spec, &artifact, state::installed_at(spec)?)?;
    println!(
        "\u{2713} Updated {} {} -> {} at {}",
        spec.name,
        previous,
        updated.installed_version,
        updated.path.display()
    );
    Ok(())
}

fn ensure_no_foreign_copies(spec: &catalog::AppSpec) -> Result<()> {
    let foreign = state::foreign_copies(spec)?;
    if foreign.is_empty() {
        return Ok(());
    }
    let paths = foreign
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let hint = spec
        .removal_hint
        .map(|hint| format!(" (e.g. '{hint}')"))
        .unwrap_or_default();
    anyhow::bail!(
        "{} exists at {} and is not managed by lum; remove it{hint} first — lum never installs alongside a foreign copy",
        spec.app_bundle,
        paths
    );
}

fn install_cmd(app: &str, force: bool) -> Result<()> {
    let spec = catalog::lookup_app(app)?;
    ensure_no_foreign_copies(spec)?;
    let status = state::local_status(spec)?;
    if status.exists && !status.managed && !force {
        anyhow::bail!(
            "{} already exists at {} but is not managed by lum; rerun with --force to overwrite it",
            spec.name,
            status.path.display()
        );
    }
    if status.exists && status.managed && !force {
        anyhow::bail!(
            "{} is already installed at {}; use 'lum apps update {}' or rerun with --force",
            spec.name,
            status.path.display(),
            spec.name
        );
    }
    let artifact = install::resolve_latest(spec)?;
    let installed = install::install_app(spec, &artifact, None)?;
    println!(
        "\u{2713} Installed {} {} to {}",
        spec.name,
        installed.installed_version,
        installed.path.display()
    );
    post_install_notes(spec);
    Ok(())
}

fn post_install_notes(spec: &catalog::AppSpec) {
    if spec.needs_rosetta && cfg!(target_arch = "aarch64") {
        println!(
            "Note: {} is an Intel-only build and requires Rosetta 2.",
            spec.app_bundle
        );
        println!("      Install it once with: softwareupdate --install-rosetta --agree-to-license");
    }
}

fn status_cmd(app: &str) -> Result<()> {
    let spec = catalog::lookup_app(app)?;
    let status = state::local_status(spec)?;
    println!("App:                {}", spec.name);
    println!("Bundle:             {}", spec.app_bundle);
    println!("Managed:           {}", yes_no(status.managed));
    println!("Path:              {}", status.path.display());
    println!("Exists:            {}", yes_no(status.exists));
    let foreign = state::foreign_copies(spec)?;
    if !foreign.is_empty() {
        let paths = foreign
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        println!("Foreign copies:     {paths}");
    }
    println!(
        "Installed version: {}",
        status.effective_version().unwrap_or("unknown")
    );
    Ok(())
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn list() -> Result<()> {
    println!("{:<18} {:<14} {:<12} DESCRIPTION", "APP", "BUNDLE", "STATE");
    for app in catalog::CATALOG {
        let status = state::local_status(app)?;
        let label = if status.managed && status.exists {
            "installed"
        } else if status.exists {
            "unmanaged"
        } else {
            "not installed"
        };
        println!(
            "{:<18} {:<14} {:<12} {}",
            app.name, app.app_bundle, label, app.description
        );
    }
    for app in catalog::CATALOG {
        for path in state::foreign_copies(app)? {
            println!("foreign: {} ({})", path.display(), app.name);
        }
    }
    Ok(())
}
