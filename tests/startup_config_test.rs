use std::collections::HashMap;

use docmost_local_mcp::{
    startup_config::{normalize_base_url, parse_startup_config},
    types::AuthorityMode,
};

#[test]
fn reads_base_url_from_cli_arguments() {
    let argv = vec![
        "--base-url".to_string(),
        "https://docs.example.com/".to_string(),
    ];
    let config = parse_startup_config(&argv, &HashMap::new()).unwrap();
    assert_eq!(config.base_url.as_deref(), Some("https://docs.example.com"));
}

#[test]
fn supports_inline_cli_argument_syntax() {
    let argv = vec!["--base-url=https://docs.example.com/".to_string()];
    let config = parse_startup_config(&argv, &HashMap::new()).unwrap();
    assert_eq!(config.base_url.as_deref(), Some("https://docs.example.com"));
}

#[test]
fn falls_back_to_environment() {
    let env = HashMap::from([(
        "DOCMOST_BASE_URL".to_string(),
        "https://env.example.com/".to_string(),
    )]);
    let config = parse_startup_config(&[], &env).unwrap();
    assert_eq!(config.base_url.as_deref(), Some("https://env.example.com"));
}

#[test]
fn throws_when_base_url_flag_is_missing_value() {
    let argv = vec!["--base-url".to_string()];
    let error = parse_startup_config(&argv, &HashMap::new()).unwrap_err();
    assert_eq!(error.to_string(), "Missing value for --base-url.");
}

#[test]
fn removes_trailing_slashes() {
    assert_eq!(
        normalize_base_url("https://docs.example.com///"),
        "https://docs.example.com"
    );
}

#[test]
fn defaults_to_read_only_with_no_write_allowlist() {
    let config = parse_startup_config(&[], &HashMap::new()).unwrap();
    assert_eq!(config.authority_mode, AuthorityMode::ReadOnly);
    assert!(config.allowed_write_tools.is_empty());
}

#[test]
fn enables_only_explicitly_allowlisted_writes() {
    let argv = vec![
        "--authority-mode=write".to_string(),
        "--write-tools=create_page, update_comment".to_string(),
    ];
    let config = parse_startup_config(&argv, &HashMap::new()).unwrap();
    assert_eq!(config.authority_mode, AuthorityMode::Write);
    assert_eq!(
        config.allowed_write_tools.into_iter().collect::<Vec<_>>(),
        ["create_page", "update_comment"]
    );
}

#[test]
fn reads_authority_configuration_from_environment() {
    let env = HashMap::from([
        ("DOCMOST_AUTHORITY_MODE".to_string(), "write".to_string()),
        (
            "DOCMOST_WRITE_TOOLS".to_string(),
            "move_page_to_space".to_string(),
        ),
    ]);
    let config = parse_startup_config(&[], &env).unwrap();
    assert_eq!(config.authority_mode, AuthorityMode::Write);
    assert!(config.allowed_write_tools.contains("move_page_to_space"));
}

#[test]
fn cli_authority_configuration_overrides_environment() {
    let env = HashMap::from([
        ("DOCMOST_AUTHORITY_MODE".to_string(), "write".to_string()),
        ("DOCMOST_WRITE_TOOLS".to_string(), "create_page".to_string()),
    ]);
    let argv = vec![
        "--authority-mode=write".to_string(),
        "--write-tools=update_space".to_string(),
    ];
    let config = parse_startup_config(&argv, &env).unwrap();
    assert_eq!(
        config.allowed_write_tools.into_iter().collect::<Vec<_>>(),
        ["update_space"]
    );
}

#[test]
fn invalid_authority_configurations_fail_closed() {
    let cases = [
        (vec!["--authority-mode=admin"], "Invalid authority mode"),
        (
            vec!["--write-tools=create_page"],
            "authority mode is read-only",
        ),
        (
            vec!["--authority-mode=write"],
            "requires a nonempty --write-tools",
        ),
        (
            vec!["--authority-mode=write", "--write-tools=list_spaces"],
            "Unknown write tool 'list_spaces'",
        ),
        (
            vec![
                "--authority-mode=write",
                "--write-tools=create_page,create_page",
            ],
            "Duplicate write tool 'create_page'",
        ),
        (
            vec!["--authority-mode=write", "--write-tools=create_page,"],
            "empty tool name",
        ),
    ];

    for (argv, expected) in cases {
        let argv = argv.into_iter().map(str::to_string).collect::<Vec<_>>();
        let error = parse_startup_config(&argv, &HashMap::new()).unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?}, got {error}"
        );
    }
}

#[test]
fn reports_missing_authority_values() {
    for flag in ["--authority-mode", "--write-tools"] {
        let error = parse_startup_config(&[flag.to_string()], &HashMap::new()).unwrap_err();
        assert_eq!(error.to_string(), format!("Missing value for {flag}."));
    }
}
