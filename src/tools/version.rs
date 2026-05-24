use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};

use super::catalog::ToolSpec;

pub(crate) fn probe_version(spec: &ToolSpec, path: &Path) -> Result<String> {
    if spec.version_args.is_empty() {
        bail!("no version args configured");
    }
    let output = Command::new(path).args(spec.version_args).output()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if text.is_empty() {
        bail!("version command produced no output");
    }
    Ok(extract_version(&text).unwrap_or(text))
}

fn extract_version(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|part| part.chars().any(|c| c.is_ascii_digit()))
        .map(|s| s.trim_start_matches('v').to_owned())
}

pub(crate) fn compare_versions(a: &str, b: &str) -> i32 {
    let parse = |s: &str| {
        s.trim_start_matches('v')
            .split(['.', '-'])
            .take(3)
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let av = parse(a);
    let bv = parse(b);
    for i in 0..3 {
        let aa = *av.get(i).unwrap_or(&0);
        let bb = *bv.get(i).unwrap_or(&0);
        if aa < bb {
            return -1;
        }
        if aa > bb {
            return 1;
        }
    }
    0
}
