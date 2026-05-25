use std::path::Path;

pub fn quote_path(path: &Path) -> String {
    let raw = path.display().to_string();
    if raw.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | ':' | '+' | '=' | ',')
    }) {
        return raw;
    }

    format!("'{}'", raw.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::quote_path;
    use std::path::Path;

    #[test]
    fn quote_path_quotes_spaces() {
        assert_eq!(
            quote_path(Path::new(
                "/Users/me/Library/Application Support/lum/git-identities.json"
            )),
            "'/Users/me/Library/Application Support/lum/git-identities.json'"
        );
    }

    #[test]
    fn quote_path_escapes_single_quotes() {
        assert_eq!(
            quote_path(Path::new("/tmp/it's here/config.json")),
            "'/tmp/it'\\''s here/config.json'"
        );
    }
}
