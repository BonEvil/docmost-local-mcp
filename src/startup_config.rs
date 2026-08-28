use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use url::{Host, Url};

use crate::types::StartupConfig;

pub fn parse_startup_config(
    argv: &[String],
    env: &HashMap<String, String>,
) -> Result<StartupConfig> {
    let mut base_url = read_base_url_from_env(env);
    let mut allow_insecure_loopback_http =
        read_bool_from_env(env, "DOCMOST_ALLOW_INSECURE_LOOPBACK_HTTP")?;
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
        }

        if argument == "--allow-insecure-loopback-http" {
            allow_insecure_loopback_http = true;
        }

        index += 1;
    }

    let mut config = StartupConfig {
        allow_insecure_loopback_http,
        ..StartupConfig::default()
    };
    if let Some(base_url) = base_url.filter(|value| !value.trim().is_empty()) {
        config.base_url = Some(
            CanonicalDocmostOrigin::parse(&base_url, allow_insecure_loopback_http)?.into_string(),
        );
    }

    Ok(config)
}

pub fn parse_runtime_startup_config(argv: &[String]) -> Result<StartupConfig> {
    let env = std::env::vars().collect::<HashMap<_, _>>();
    parse_startup_config(argv, &env)
}

pub fn normalize_base_url(base_url: &str) -> Result<String> {
    Ok(CanonicalDocmostOrigin::parse(base_url, false)?.into_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalDocmostOrigin {
    scheme: String,
    host: String,
    effective_port: u16,
    serialized: String,
}

impl CanonicalDocmostOrigin {
    pub fn parse(input: &str, allow_insecure_loopback_http: bool) -> Result<Self> {
        let input = input.trim();
        if input.is_empty() {
            bail!("The Docmost base URL cannot be empty.");
        }
        if input.contains('\\') || input.chars().any(char::is_control) {
            bail!("The Docmost base URL contains disallowed characters.");
        }

        let url = Url::parse(input).context("The Docmost base URL must be an absolute URL")?;
        let authority_and_suffix =
            input
                .split_once("://")
                .map(|(_, value)| value)
                .ok_or_else(|| {
                    anyhow::anyhow!("The Docmost base URL must use an absolute authority.")
                })?;
        let suffix_index = authority_and_suffix
            .find(['/', '?', '#'])
            .unwrap_or(authority_and_suffix.len());
        let raw_authority = &authority_and_suffix[..suffix_index];
        let raw_suffix = &authority_and_suffix[suffix_index..];
        if !url.username().is_empty() || url.password().is_some() {
            bail!("The Docmost base URL must not contain user information.");
        }
        if url.query().is_some() {
            bail!("The Docmost base URL must not contain a query string.");
        }
        if url.fragment().is_some() {
            bail!("The Docmost base URL must not contain a fragment.");
        }
        if url.path() != "/" || (!raw_suffix.is_empty() && raw_suffix != "/") {
            bail!("The Docmost base URL must be an origin without a path.");
        }

        let scheme = url.scheme();
        if scheme != "https" && scheme != "http" {
            bail!("The Docmost base URL scheme must be HTTPS.");
        }
        let host = url
            .host()
            .ok_or_else(|| anyhow::anyhow!("The Docmost base URL must contain a host."))?;

        if scheme == "http" {
            let literal_loopback = is_literal_loopback_authority(raw_authority);
            if !literal_loopback {
                bail!("Plain HTTP is allowed only for a literal loopback address.");
            }
            if !allow_insecure_loopback_http {
                bail!(
                    "Loopback HTTP is disabled. Deliberately enable --allow-insecure-loopback-http only for local development or tests."
                );
            }
        }

        let effective_port = url
            .port_or_known_default()
            .ok_or_else(|| anyhow::anyhow!("The Docmost base URL has no effective port."))?;
        let host = match host {
            Host::Domain(value) => value.to_ascii_lowercase(),
            Host::Ipv4(value) => value.to_string(),
            Host::Ipv6(value) => format!("[{value}]"),
        };
        let default_port = (scheme == "https" && effective_port == 443)
            || (scheme == "http" && effective_port == 80);
        let serialized = if default_port {
            format!("{scheme}://{host}")
        } else {
            format!("{scheme}://{host}:{effective_port}")
        };

        Ok(Self {
            scheme: scheme.to_string(),
            host,
            effective_port,
            serialized,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.serialized
    }

    pub fn into_string(self) -> String {
        self.serialized
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn effective_port(&self) -> u16 {
        self.effective_port
    }
}

fn is_literal_loopback_authority(authority: &str) -> bool {
    if let Some(bracketed) = authority.strip_prefix('[') {
        let Some((host, remainder)) = bracketed.split_once(']') else {
            return false;
        };
        if !remainder.is_empty() && !remainder.starts_with(':') {
            return false;
        }
        return host
            .parse::<std::net::Ipv6Addr>()
            .is_ok_and(|address| address.is_loopback());
    }

    let host = authority
        .rsplit_once(':')
        .map(|(host, _port)| host)
        .unwrap_or(authority);
    host.parse::<std::net::Ipv4Addr>()
        .is_ok_and(|address| address.is_loopback())
}

fn read_base_url_from_env(env: &HashMap<String, String>) -> Option<String> {
    let value = env.get("DOCMOST_BASE_URL")?.trim();
    if value.is_empty() {
        return None;
    }

    Some(value.to_string())
}

fn read_bool_from_env(env: &HashMap<String, String>, name: &str) -> Result<bool> {
    let Some(value) = env.get(name) else {
        return Ok(false);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" => Ok(true),
        "0" | "false" | "" => Ok(false),
        _ => bail!("{name} must be one of: true, false, 1, or 0."),
    }
}

pub fn ensure_base_url(config: &StartupConfig) -> Result<String> {
    if let Some(base_url) = &config.base_url {
        return Ok(base_url.clone());
    }

    bail!("A Docmost base URL is required. Pass --base-url or set DOCMOST_BASE_URL.")
}
