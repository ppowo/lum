use anyhow::{Context, Result};

pub(crate) struct ToolSpec {
    pub name: &'static str,
    pub binary: &'static str,
    pub description: &'static str,
    pub version_args: &'static [&'static str],
    pub owner: &'static str,
    pub repo: &'static str,
}

pub(crate) const CATALOG: &[ToolSpec] = &[
    ToolSpec {
        name: "scc",
        binary: "scc",
        description: "Fast code counter with complexity",
        version_args: &["--version"],
        owner: "boyter",
        repo: "scc",
    },
    ToolSpec {
        name: "universal-ctags",
        binary: "ctags",
        description: "Maintained ctags implementation for source code indexing",
        version_args: &[],
        owner: "universal-ctags",
        repo: "ctags-nightly-build",
    },
];

pub(crate) fn lookup_tool(name: &str) -> Result<&'static ToolSpec> {
    CATALOG
        .iter()
        .find(|tool| tool.name == name)
        .with_context(|| {
            format!(
                "unknown managed tool {name:?} (available: {})",
                available_tools()
            )
        })
}

pub(crate) fn available_tools() -> String {
    CATALOG
        .iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn test_artifact_env(spec: &ToolSpec) -> String {
    format!(
        "LUM_TOOLS_TEST_ARTIFACT_{}",
        spec.name.replace('-', "_").to_ascii_uppercase()
    )
}
