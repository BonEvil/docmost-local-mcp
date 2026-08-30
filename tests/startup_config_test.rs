use std::collections::HashMap;

use docmost_local_mcp::startup_config::{
    CanonicalDocmostOrigin, normalize_base_url, parse_startup_config,
};
use docmost_local_mcp::types::AuthorityMode;

#[test]
fn reads_and_canonicalizes_base_url_from_cli_arguments() {
    let argv = vec![
        "--base-url".to_string(),
        "HTTPS://Docs.Example.COM:443/".to_string(),
    ];
    let config = parse_startup_config(&argv, &HashMap::new()).unwrap();
    assert_eq!(config.base_url.as_deref(), Some("https://docs.example.com"));
}

#[test]
fn supports_inline_cli_argument_syntax() {
    let argv = vec!["--base-url=https://docs.example.com:8443/".to_string()];
    let config = parse_startup_config(&argv, &HashMap::new()).unwrap();
    assert_eq!(
        config.base_url.as_deref(),
        Some("https://docs.example.com:8443")
    );
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
fn canonical_origin_exposes_scheme_host_and_effective_port() {
    let origin = CanonicalDocmostOrigin::parse("https://DOCS.example.com/", false).unwrap();
    assert_eq!(origin.as_str(), "https://docs.example.com");
    assert_eq!(origin.scheme(), "https");
    assert_eq!(origin.host(), "docs.example.com");
    assert_eq!(origin.effective_port(), 443);
    assert_eq!(
        normalize_base_url("https://docs.example.com:443/").unwrap(),
        "https://docs.example.com"
    );
}

#[test]
fn loopback_http_requires_deliberate_enablement_and_literal_address() {
    let disabled = CanonicalDocmostOrigin::parse("http://127.0.0.1:3000", false).unwrap_err();
    assert!(disabled.to_string().contains("Deliberately enable"));

    assert_eq!(
        CanonicalDocmostOrigin::parse("http://127.0.0.1:3000/", true)
            .unwrap()
            .as_str(),
        "http://127.0.0.1:3000"
    );
    assert_eq!(
        CanonicalDocmostOrigin::parse("http://[::1]:3000/", true)
            .unwrap()
            .as_str(),
        "http://[::1]:3000"
    );
    assert!(CanonicalDocmostOrigin::parse("http://localhost:3000", true).is_err());
    assert!(CanonicalDocmostOrigin::parse("http://2130706433:3000", true).is_err());
    assert!(CanonicalDocmostOrigin::parse("http://0x7f000001:3000", true).is_err());
    assert!(CanonicalDocmostOrigin::parse("http://126.0.0.1:3000", true).is_err());
    assert!(CanonicalDocmostOrigin::parse("http://192.0.2.1:3000", true).is_err());
}

#[test]
fn rejects_unsupported_or_misleading_url_components() {
    for rejected in [
        "ftp://docs.example.com",
        "//docs.example.com",
        "https://user@docs.example.com",
        "https://user:password@docs.example.com",
        "https://docs.example.com/path",
        "https://docs.example.com/.",
        "https://docs.example.com/path/..",
        "https://docs.example.com/?query=value",
        "https://docs.example.com/#fragment",
        "https://docs.example.com\\@evil.example",
        "https://docs.example.com///",
        "https://",
        "not a url",
    ] {
        assert!(
            CanonicalDocmostOrigin::parse(rejected, false).is_err(),
            "expected rejection for {rejected:?}"
        );
    }
}

#[test]
fn startup_flag_enables_only_loopback_http() {
    let argv = vec![
        "--base-url=http://127.0.0.1:3000".to_string(),
        "--allow-insecure-loopback-http".to_string(),
    ];
    let config = parse_startup_config(&argv, &HashMap::new()).unwrap();
    assert!(config.allow_insecure_loopback_http);
    assert_eq!(config.base_url.as_deref(), Some("http://127.0.0.1:3000"));

    let remote = vec![
        "--base-url=http://example.com".to_string(),
        "--allow-insecure-loopback-http".to_string(),
    ];
    assert!(parse_startup_config(&remote, &HashMap::new()).is_err());
}

#[test]
fn insecure_credential_file_requires_explicit_operator_acknowledgement() {
    let defaults = parse_startup_config(&[], &HashMap::new()).unwrap();
    assert!(!defaults.allow_insecure_credential_file);

    let cli = parse_startup_config(
        &["--allow-insecure-credential-file".to_string()],
        &HashMap::new(),
    )
    .unwrap();
    assert!(cli.allow_insecure_credential_file);

    let env = HashMap::from([(
        "DOCMOST_ALLOW_INSECURE_CREDENTIAL_FILE".to_string(),
        "true".to_string(),
    )]);
    assert!(
        parse_startup_config(&[], &env)
            .unwrap()
            .allow_insecure_credential_file
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
