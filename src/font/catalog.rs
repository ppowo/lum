use anyhow::{Context, Result};

pub(crate) struct FontSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub download_url: &'static str,
    pub dir_name: &'static str,
}

pub(crate) const CATALOG: &[FontSpec] = &[FontSpec {
    name: "dmca-sans-serif",
    description: "General-purpose sans serif font metric-compatible with Consolas",
    download_url: "https://typedesign.replit.app/DMCAsansserif9.0-20252.zip",
    dir_name: "dmca-sans-serif",
}];

pub(crate) fn lookup_font(name: &str) -> Result<&'static FontSpec> {
    CATALOG
        .iter()
        .find(|f| f.name == name)
        .with_context(|| format!("unknown font {name:?} (available: {})", available_fonts()))
}

pub(crate) fn available_fonts() -> String {
    CATALOG
        .iter()
        .map(|f| f.name)
        .collect::<Vec<_>>()
        .join(", ")
}
