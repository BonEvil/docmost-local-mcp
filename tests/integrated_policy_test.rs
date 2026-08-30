use std::{fs, path::PathBuf};

use anyhow::Result;
use docmost_local_mcp::startup_config::parse_startup_config;
use docmost_local_mcp::types::AuthorityMode;

#[test]
fn production_atlas_example_is_absolute_https_and_read_only() -> Result<()> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/atlas-mcp.production.example.json");
    let config: serde_json::Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let server = &config["mcpServers"]["docmost"];
    let command = server["command"].as_str().expect("string command");
    let args = server["args"]
        .as_array()
        .expect("array args")
        .iter()
        .map(|value| value.as_str().expect("string arg").to_string())
        .collect::<Vec<_>>();

    assert!(command.starts_with('/'), "Atlas command must be absolute");
    assert!(!command.contains("npx"));
    assert!(
        args.iter()
            .any(|arg| arg.starts_with("--base-url=https://"))
    );
    assert!(!args.iter().any(|arg| {
        arg == "--allow-insecure-loopback-http"
            || arg == "--allow-insecure-credential-file"
            || arg.starts_with("--write-tools")
    }));

    let parsed = parse_startup_config(&args, &Default::default())?;
    assert_eq!(parsed.authority_mode, AuthorityMode::ReadOnly);
    assert!(parsed.allowed_write_tools.is_empty());
    assert!(!parsed.allow_insecure_loopback_http);
    assert!(!parsed.allow_insecure_credential_file);
    Ok(())
}
