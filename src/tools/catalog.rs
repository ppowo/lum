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
        name: "difftastic",
        binary: "difft",
        description: "A structural diff that understands syntax",
        version_args: &["--version"],
        owner: "Wilfred",
        repo: "difftastic",
    },
    ToolSpec {
        name: "fd",
        binary: "fd",
        description: "A simple, fast and user-friendly alternative to find",
        version_args: &["--version"],
        owner: "sharkdp",
        repo: "fd",
    },
    ToolSpec {
        name: "jq",
        binary: "jq",
        description: "A lightweight and flexible command-line JSON processor",
        version_args: &["--version"],
        owner: "jqlang",
        repo: "jq",
    },
    ToolSpec {
        name: "ripgrep",
        binary: "rg",
        description: "Recursively searches directories for a regex pattern",
        version_args: &["--version"],
        owner: "BurntSushi",
        repo: "ripgrep",
    },
    ToolSpec {
        name: "scc",
        binary: "scc",
        description: "Fast code counter with complexity",
        version_args: &["--version"],
        owner: "boyter",
        repo: "scc",
    },
    ToolSpec {
        name: "shellcheck",
        binary: "shellcheck",
        description: "Static analysis for shell scripts",
        version_args: &["--version"],
        owner: "koalaman",
        repo: "shellcheck",
    },
    ToolSpec {
        name: "universal-ctags",
        binary: "ctags",
        description: "Maintained ctags implementation for source code indexing",
        version_args: &[],
        owner: "universal-ctags",
        repo: "ctags-nightly-build",
    },
    ToolSpec {
        name: "yq",
        binary: "yq",
        description: "YAML, JSON, XML, CSV, TSV and properties processor",
        version_args: &["--version"],
        owner: "mikefarah",
        repo: "yq",
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
