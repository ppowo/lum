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
        .map(|s| {
            let s = s.trim_start_matches('v');
            // Strip any leading non-numeric prefix (e.g. "jq-" from "jq-1.8.1").
            if let Some(pos) = s.find(|c: char| c.is_ascii_digit()) {
                // Keep any delimiter right before the digit (e.g. keep the '-' in
                // "v1.8.1" is already stripped, but for "jq-1.8.1" we drop
                // "jq-" so compare_versions sees "1.8.1").
                let prefix_end = s[..pos].rfind(['-', '.']).map_or(pos, |d| d + 1);
                s[prefix_end..].to_owned()
            } else {
                s.to_owned()
            }
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_jq_style_version() {
        assert_eq!(extract_version("jq-1.8.1"), Some("1.8.1".to_owned()));
    }

    #[test]
    fn extract_plain_version() {
        assert_eq!(extract_version("1.8.1"), Some("1.8.1".to_owned()));
    }

    #[test]
    fn extract_v_prefixed_version() {
        assert_eq!(extract_version("v1.8.1"), Some("1.8.1".to_owned()));
    }

    #[test]
    fn extract_version_from_sentence() {
        assert_eq!(extract_version("ripgrep 15.1.0"), Some("15.1.0".to_owned()));
    }

    #[test]
    fn compare_jq_runtime_vs_release() {
        // Before the fix, compare_versions("jq-1.8.1", "1.8.1") returned -1
        assert_eq!(compare_versions("1.8.1", "1.8.1"), 0);
    }
}
