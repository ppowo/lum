pub(crate) const ALIASES: &[(&str, &str)] = &[
    ("exa", "EXA_API_KEY"),
    ("neuralwatt", "NEURALWATT_API_KEY"),
    ("openrouter", "OPENROUTER_API_KEY"),
    ("synthetic", "SYNTHETIC_API_KEY"),
];

pub(crate) const FORCED_ENV: &[(&str, &str)] = &[
    ("PI_HASHLINE_GREP_MAX_LINES", "150"),
    ("PI_HASHLINE_GREP_MAX_BYTES", "10000"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD", "1"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD_MAX_LINES", "400"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD_MAX_BYTES", "25000"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD_HEAD_LINES", "60"),
    ("PI_HASHLINE_BASH_CONTEXT_GUARD_TAIL_LINES", "150"),
    ("npm_config_ignore_scripts", "true"),
];

pub(crate) fn variable_for_alias(alias: &str) -> Option<&'static str> {
    ALIASES
        .iter()
        .find_map(|(key, variable)| (*key == alias).then_some(*variable))
}

pub(crate) fn mask_secret(value: &str) -> String {
    if value.len() <= 8 {
        "********".to_owned()
    } else {
        format!("{}...{}", &value[..4], &value[value.len() - 4..])
    }
}
