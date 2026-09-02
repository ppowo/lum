use anyhow::{Context, Result};

pub(crate) struct AppSpec {
    pub name: &'static str,
    pub app_bundle: &'static str,
    pub description: &'static str,
    pub owner: &'static str,
    pub repo: &'static str,
    pub asset_prefix: &'static str,
    pub needs_rosetta: bool,
    /// How to remove a foreign (unmanaged) copy, shown when installs fail fast.
    pub removal_hint: Option<&'static str>,
}

pub(crate) const CATALOG: &[AppSpec] = &[AppSpec {
    name: "openemu",
    app_bundle: "OpenEmu.app",
    description: "Open source video game emulation for macOS",
    owner: "OpenEmu",
    repo: "OpenEmu",
    asset_prefix: "OpenEmu",
    needs_rosetta: true,
    removal_hint: Some("brew uninstall --cask openemu"),
}];

pub(crate) fn test_artifact_env(spec: &AppSpec) -> String {
    format!(
        "LUM_APPS_TEST_ARTIFACT_{}",
        spec.name.replace('-', "_").to_ascii_uppercase()
    )
}

pub(crate) fn lookup_app(name: &str) -> Result<&'static AppSpec> {
    CATALOG
        .iter()
        .find(|app| app.name == name)
        .with_context(|| {
            format!(
                "unknown managed app {name:?} (available: {})",
                available_apps()
            )
        })
}

pub(crate) fn available_apps() -> String {
    CATALOG
        .iter()
        .map(|app| app.name)
        .collect::<Vec<_>>()
        .join(", ")
}
