use std::collections::{BTreeSet, HashMap};

use anyhow::{Result, bail};

use crate::types::{AuthorityMode, StartupConfig};

pub const WRITE_TOOL_NAMES: [&str; 10] = [
    "create_page",
    "update_page",
    "duplicate_page",
    "copy_page_to_space",
    "move_page",
    "move_page_to_space",
    "create_space",
    "update_space",
    "create_comment",
    "update_comment",
];

pub fn parse_startup_config(
    argv: &[String],
    env: &HashMap<String, String>,
) -> Result<StartupConfig> {
    let mut base_url = read_base_url_from_env(env);
    let mut authority_mode = env.get("DOCMOST_AUTHORITY_MODE").cloned();
    let mut write_tools = env.get("DOCMOST_WRITE_TOOLS").cloned();
    let mut index = 0usize;

    while index < argv.len() {
        let argument = &argv[index];

        if argument == "--base-url" {
            let value = argv
                .get(index + 1)
                .ok_or_else(|| anyhow::anyhow!("Missing value for --base-url."))?;
            base_url = Some(value.clone());
            index += 2;
            continue;
        }

        if let Some(value) = argument.strip_prefix("--base-url=") {
            base_url = Some(value.to_string());
            index += 1;
            continue;
        }

        if argument == "--authority-mode" {
            authority_mode = Some(required_value(argv, index, "--authority-mode")?);
            index += 2;
            continue;
        }

        if let Some(value) = argument.strip_prefix("--authority-mode=") {
            authority_mode = Some(value.to_string());
            index += 1;
            continue;
        }

        if argument == "--write-tools" {
            write_tools = Some(required_value(argv, index, "--write-tools")?);
            index += 2;
            continue;
        }

        if let Some(value) = argument.strip_prefix("--write-tools=") {
            write_tools = Some(value.to_string());
            index += 1;
            continue;
        }

        index += 1;
    }

    let mut config = StartupConfig::default();
    if let Some(base_url) = base_url.filter(|value| !value.trim().is_empty()) {
        config.base_url = Some(normalize_base_url(&base_url));
    }
    config.authority_mode = parse_authority_mode(authority_mode.as_deref())?;
    config.allowed_write_tools = parse_write_tools(write_tools.as_deref())?;
    validate_authority_config(&config)?;

    Ok(config)
}

fn required_value(argv: &[String], index: usize, flag: &str) -> Result<String> {
    argv.get(index + 1)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing value for {flag}."))
}

fn parse_authority_mode(value: Option<&str>) -> Result<AuthorityMode> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("read-only") => Ok(AuthorityMode::ReadOnly),
        Some("write") => Ok(AuthorityMode::Write),
        Some(value) => bail!("Invalid authority mode '{value}'. Expected 'read-only' or 'write'."),
    }
}

fn parse_write_tools(value: Option<&str>) -> Result<BTreeSet<String>> {
    let Some(value) = value else {
        return Ok(BTreeSet::new());
    };
    if value.trim().is_empty() {
        bail!("The write-tool allowlist must not be empty when configured.");
    }

    let mut tools = BTreeSet::new();
    for raw_name in value.split(',') {
        let name = raw_name.trim();
        if name.is_empty() {
            bail!("The write-tool allowlist contains an empty tool name.");
        }
        if !WRITE_TOOL_NAMES.contains(&name) {
            bail!(
                "Unknown write tool '{name}'. Allowed write tools: {}.",
                WRITE_TOOL_NAMES.join(", ")
            );
        }
        if !tools.insert(name.to_string()) {
            bail!("Duplicate write tool '{name}' in allowlist.");
        }
    }
    Ok(tools)
}

pub fn validate_authority_config(config: &StartupConfig) -> Result<()> {
    if let Some(name) = config
        .allowed_write_tools
        .iter()
        .find(|name| !WRITE_TOOL_NAMES.contains(&name.as_str()))
    {
        bail!("Unknown write tool '{name}' in programmatic configuration.");
    }
    match (config.authority_mode, config.allowed_write_tools.is_empty()) {
        (AuthorityMode::ReadOnly, false) => bail!(
            "Write tools were allowlisted while authority mode is read-only. Set --authority-mode=write explicitly."
        ),
        (AuthorityMode::Write, true) => {
            bail!("Write authority requires a nonempty --write-tools allowlist.")
        }
        _ => Ok(()),
    }
}

pub fn parse_runtime_startup_config(argv: &[String]) -> Result<StartupConfig> {
    let env = std::env::vars().collect::<HashMap<_, _>>();
    parse_startup_config(argv, &env)
}

pub fn normalize_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn read_base_url_from_env(env: &HashMap<String, String>) -> Option<String> {
    let value = env.get("DOCMOST_BASE_URL")?.trim();
    if value.is_empty() {
        return None;
    }

    Some(value.to_string())
}

pub fn ensure_base_url(config: &StartupConfig) -> Result<String> {
    if let Some(base_url) = &config.base_url {
        return Ok(base_url.clone());
    }

    bail!("A Docmost base URL is required. Pass --base-url or set DOCMOST_BASE_URL.")
}
