use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow};
use base64::{
    Engine as _,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{Client, Response, header::SET_COOKIE};

use crate::{
    auth::{
        local_server::{LocalAuthDefaults, LocalAuthServer},
        webview::{
            AuthWindowHandle, helper_exit_cancelled, helper_exit_error_message,
            helper_exit_success, launch_auth_window,
        },
    },
    debug::debug_log,
    network_policy::{NetworkPolicy, read_bounded_body, safe_transport_error},
    startup_config::CanonicalDocmostOrigin,
    storage::state_store::StateStore,
    types::{
        AuthenticatedSession, LoginInput, StartupConfig, StoredConfig, StoredCredentials,
        StoredSession,
    },
};

const REFRESH_WINDOW_MS: i64 = 2 * 60 * 1000;
static AUTH_TOKEN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:^|,\s*)authToken=([^;]+)").expect("valid auth token regex"));

#[derive(Debug, Clone)]
pub struct AuthManager {
    store: Arc<StateStore>,
    configured_origin: Option<CanonicalDocmostOrigin>,
    pinned_origin: Arc<Mutex<Option<CanonicalDocmostOrigin>>>,
    allow_insecure_loopback_http: bool,
    http: Client,
    network_policy: NetworkPolicy,
}

impl AuthManager {
    pub fn new(options: StartupConfig, base_dir: Option<PathBuf>) -> Result<Self> {
        Self::new_with_network_policy(options, base_dir, NetworkPolicy::default())
    }

    /// Testable constructor; production always uses the reviewed defaults through `new`.
    pub fn new_with_network_policy(
        options: StartupConfig,
        base_dir: Option<PathBuf>,
        network_policy: NetworkPolicy,
    ) -> Result<Self> {
        let configured_origin = options
            .base_url
            .as_deref()
            .map(|value| CanonicalDocmostOrigin::parse(value, options.allow_insecure_loopback_http))
            .transpose()?;
        Ok(Self {
            pinned_origin: Arc::new(Mutex::new(configured_origin.clone())),
            configured_origin,
            allow_insecure_loopback_http: options.allow_insecure_loopback_http,
            store: Arc::new(StateStore::new(
                base_dir,
                options.allow_insecure_credential_file,
            )?),
            http: network_policy.build_http_client()?,
            network_policy,
        })
    }

    pub async fn get_authenticated_session(&self) -> Result<AuthenticatedSession> {
        let config = self.store.read_config().await?;
        let preferred_origin = self.get_preferred_origin(config.as_ref())?;
        let session = match preferred_origin.as_ref() {
            Some(origin) => self.store.read_session(origin.as_str()).await?,
            None => None,
        };
        let has_config = config.is_some();
        let has_session = session.is_some();

        if let (Some(config), Some(session)) = (config.as_ref(), session.as_ref()) {
            let session_matches = preferred_origin.as_ref().is_some_and(|origin| {
                saved_session_matches_config(
                    config,
                    session,
                    origin,
                    self.allow_insecure_loopback_http,
                )
            });
            if session_matches && !is_session_expiring(session) {
                debug_log(
                    "auth",
                    "Using saved session",
                    Some(&serde_json::json!({ "hasSession": true })),
                );
                return Ok(to_authenticated_session(config.clone(), session.clone()));
            }
        }

        debug_log(
            "auth",
            "Saved session missing or expiring; reauthenticating",
            Some(&serde_json::json!({
                "hasConfig": has_config,
                "hasSession": has_session,
            })),
        );
        self.reauthenticate().await
    }

    pub async fn reauthenticate(&self) -> Result<AuthenticatedSession> {
        let config = self.store.read_config().await?;
        let preferred_origin = self.get_preferred_origin(config.as_ref())?;
        let credentials = match preferred_origin.as_ref() {
            Some(origin) => self.store.read_credentials(origin.as_str()).await?,
            None => None,
        };
        let has_config = config.is_some();
        let has_credentials = credentials.is_some();

        if let (Some(origin), Some(credentials), Some(config)) = (
            preferred_origin.as_ref(),
            credentials.clone(),
            config.as_ref(),
        ) {
            if credentials.email != config.email {
                self.store.clear_credentials(origin.as_str()).await?;
                return Err(anyhow!(
                    "Remembered credentials do not match the active Docmost identity. The stale credentials were cleared; interactive authentication is required."
                ));
            }
            debug_log(
                "auth",
                "Reauthenticating with saved credentials",
                Some(&serde_json::json!({ "hasCredentials": true })),
            );
            return self
                .login(LoginInput {
                    base_url: origin.as_str().to_string(),
                    email: credentials.email,
                    password: credentials.password,
                    remember_password: true,
                })
                .await;
        }

        debug_log(
            "auth",
            "No reusable credentials available; starting interactive authentication",
            Some(&serde_json::json!({
                "hasConfig": has_config,
                "hasCredentials": has_credentials,
            })),
        );
        self.prompt_for_login(config.as_ref()).await
    }

    pub async fn login(&self, input: LoginInput) -> Result<AuthenticatedSession> {
        let origin =
            CanonicalDocmostOrigin::parse(&input.base_url, self.allow_insecure_loopback_http)?;
        self.pin_origin(&origin)?;
        let base_url = origin.as_str().to_string();
        debug_log(
            "auth",
            "Starting Docmost login",
            Some(&serde_json::json!({
                "endpointClass": "auth/login",
                "fieldNames": ["email", "password"]
            })),
        );

        let response = self
            .http
            .post(format!("{base_url}/api/auth/login"))
            .json(&serde_json::json!({
                "email": input.email,
                "password": input.password
            }))
            .send()
            .await
            .map_err(safe_transport_error)
            .context("Failed to call the Docmost login endpoint")?;

        debug_log(
            "auth",
            "Docmost login response received",
            Some(&serde_json::json!({
                "status": response.status().as_u16(),
                "ok": response.status().is_success()
            })),
        );

        if !response.status().is_success() {
            let status = response.status();
            let details =
                safe_read_response_text(response, self.network_policy.max_error_body_bytes).await?;
            return Err(anyhow!(
                format!("Docmost login failed ({}). {}", status, details)
                    .trim()
                    .to_string()
            ));
        }

        let token = read_auth_token_from_headers(response.headers()).ok_or_else(|| {
            anyhow!("Docmost login succeeded but no authToken cookie was returned.")
        })?;
        // Authentication is header-based, but still consume the success body through a
        // hard cap so a hostile login endpoint cannot stream indefinitely or oversized.
        read_bounded_body(
            response,
            self.network_policy.max_error_body_bytes,
            "authentication response body",
        )
        .await?;
        let expires_at = get_jwt_expiry_iso(&token);
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.persist_authenticated_state(
            LoginInput {
                base_url,
                email: input.email,
                password: input.password,
                remember_password: input.remember_password,
            },
            token,
            expires_at,
            now,
        )
        .await
    }

    async fn persist_authenticated_state(
        &self,
        input: LoginInput,
        token: String,
        expires_at: Option<String>,
        now: String,
    ) -> Result<AuthenticatedSession> {
        let LoginInput {
            base_url,
            email,
            password,
            remember_password,
        } = input;
        if remember_password {
            self.store
                .write_credentials(&StoredCredentials {
                    origin: Some(base_url.clone()),
                    email: email.clone(),
                    password,
                })
                .await?;
        } else {
            // Clear every remembered representation before updating config/session. If clearing
            // fails, the new session-only identity is not committed. If a later persistence step
            // fails, the older identity can no longer be silently restored.
            self.store.clear_credentials(&base_url).await?;
        }
        // Invalidate the previous session before changing the identity-bearing config. If either
        // following write fails, restart cannot combine the new config with an old token.
        self.store.clear_session(&base_url).await?;
        self.store
            .write_config(&StoredConfig {
                base_url: base_url.clone(),
                email: email.clone(),
                last_authenticated_at: now.clone(),
            })
            .await?;
        self.store
            .write_session(&StoredSession {
                origin: Some(base_url.clone()),
                email: Some(email.clone()),
                token: token.clone(),
                expires_at: expires_at.clone(),
                saved_at: now.clone(),
            })
            .await?;
        Ok(AuthenticatedSession {
            base_url,
            email,
            token,
            expires_at,
        })
    }

    pub async fn forget(&self, origin: &str) -> Result<()> {
        let origin = CanonicalDocmostOrigin::parse(origin, self.allow_insecure_loopback_http)?;
        self.store.forget_origin(origin.as_str()).await
    }

    fn get_preferred_origin(
        &self,
        config: Option<&StoredConfig>,
    ) -> Result<Option<CanonicalDocmostOrigin>> {
        if let Some(origin) = &self.configured_origin {
            return Ok(Some(origin.clone()));
        }
        let origin = config
            .map(|config| {
                CanonicalDocmostOrigin::parse(&config.base_url, self.allow_insecure_loopback_http)
            })
            .transpose()?;
        if let Some(origin) = &origin {
            self.pin_origin(origin)?;
        }
        Ok(origin)
    }

    fn pin_origin(&self, origin: &CanonicalDocmostOrigin) -> Result<()> {
        let mut pinned = self
            .pinned_origin
            .lock()
            .map_err(|_| anyhow!("The process origin lock was poisoned."))?;
        if let Some(existing) = pinned.as_ref() {
            if existing != origin {
                return Err(anyhow!(
                    "This process is pinned to {} and cannot authenticate to {}. Start a new explicit authentication flow for the new origin.",
                    existing.as_str(),
                    origin.as_str()
                ));
            }
        } else {
            *pinned = Some(origin.clone());
        }
        Ok(())
    }

    async fn prompt_for_login(
        &self,
        config: Option<&StoredConfig>,
    ) -> Result<AuthenticatedSession> {
        let preferred_base_url = self.get_preferred_origin(config)?;
        let defaults = LocalAuthDefaults {
            base_url: preferred_base_url.map(CanonicalDocmostOrigin::into_string),
            email: config.map(|config| config.email.clone()),
            base_url_readonly: self.configured_origin.is_some(),
        };

        let auth_manager = self.clone();
        let mut auth_server = LocalAuthServer::new(
            defaults,
            move |input| {
                let auth_manager = auth_manager.clone();
                async move {
                    auth_manager.login(input).await?;
                    Ok(())
                }
            },
            None,
        );

        let auth_session = auth_server.start().await?;
        let mut auth_window = launch_auth_window(&auth_session).await?;

        debug_log(
            "auth",
            "Waiting for interactive authentication",
            Some(&serde_json::json!({
                "mode": format!("{:?}", auth_window.mode),
                "loopbackAuth": true
            })),
        );

        let completion =
            wait_for_authentication_completion(&mut auth_server, &mut auth_window).await;

        let result = async {
            completion?;
            let refreshed_config = self
                .store
                .read_config()
                .await?
                .ok_or_else(|| anyhow!("Authentication completed, but no config was saved."))?;
            let refreshed_origin = CanonicalDocmostOrigin::parse(
                &refreshed_config.base_url,
                self.allow_insecure_loopback_http,
            )?;
            let refreshed_session = self
                .store
                .read_session(refreshed_origin.as_str())
                .await?
                .ok_or_else(|| anyhow!("Authentication completed, but no session was saved."))?;
            if !saved_session_matches_config(
                &refreshed_config,
                &refreshed_session,
                &refreshed_origin,
                self.allow_insecure_loopback_http,
            ) {
                return Err(anyhow!(
                    "Authentication completed, but the saved session identity did not match the active config."
                ));
            }
            Ok(to_authenticated_session(
                refreshed_config,
                refreshed_session,
            ))
        }
        .await;

        auth_window.close().await?;
        auth_server.stop().await?;

        result
    }
}

fn to_authenticated_session(config: StoredConfig, session: StoredSession) -> AuthenticatedSession {
    AuthenticatedSession {
        base_url: session.origin.clone().unwrap_or(config.base_url),
        email: config.email,
        token: session.token,
        expires_at: session.expires_at,
    }
}

fn saved_session_matches_config(
    config: &StoredConfig,
    session: &StoredSession,
    preferred_origin: &CanonicalDocmostOrigin,
    allow_insecure_loopback_http: bool,
) -> bool {
    CanonicalDocmostOrigin::parse(&config.base_url, allow_insecure_loopback_http)
        .is_ok_and(|configured| configured == *preferred_origin)
        && session.origin.as_deref() == Some(preferred_origin.as_str())
        && session.email.as_deref() == Some(config.email.as_str())
}

fn is_session_expiring(session: &StoredSession) -> bool {
    let Some(expires_at) = &session.expires_at else {
        return false;
    };

    let Ok(expires_at) = DateTime::parse_from_rfc3339(expires_at) else {
        return false;
    };

    expires_at.timestamp_millis() - Utc::now().timestamp_millis() <= REFRESH_WINDOW_MS
}

pub fn read_auth_token_from_headers(headers: &reqwest::header::HeaderMap) -> Option<String> {
    for header in headers.get_all(SET_COOKIE) {
        let Ok(cookie) = header.to_str() else {
            continue;
        };

        let token = AUTH_TOKEN_RE.captures(cookie).and_then(|c| c.get(1));
        if let Some(token) = token {
            let decoded = urlencoding::decode(token.as_str()).ok()?;
            return Some(decoded.into_owned());
        }
    }

    None
}

pub fn get_jwt_expiry_iso(token: &str) -> Option<String> {
    let payload_part = token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(payload_part)
        .or_else(|_| URL_SAFE.decode(payload_part))
        .ok()?;
    let payload = serde_json::from_slice::<serde_json::Value>(&decoded).ok()?;
    let exp = payload.get("exp")?.as_i64()?;
    DateTime::<Utc>::from_timestamp(exp, 0)
        .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

pub async fn safe_read_response_text(response: Response, limit: usize) -> Result<String> {
    let body = read_bounded_body(response, limit, "error response body").await?;
    if body.is_empty() {
        Ok(String::new())
    } else {
        // Error documents can echo requests, credentials, comments, or page content.
        // Their bytes are bounded and consumed, but never projected into diagnostics.
        Ok(format!("Response body omitted ({} bytes).", body.len()))
    }
}

async fn wait_for_authentication_completion(
    auth_server: &mut LocalAuthServer,
    auth_window: &mut AuthWindowHandle,
) -> Result<()> {
    if auth_window.mode == crate::auth::webview::AuthWindowMode::Browser {
        return auth_server.wait_for_completion().await;
    }

    let completion = auth_server.wait_for_completion();
    tokio::pin!(completion);

    tokio::select! {
        result = &mut completion => result,
        exit = auth_window.wait_for_exit() => {
            let code = exit?;
            if helper_exit_success(code) {
                completion.await
            } else if helper_exit_cancelled(code) {
                Err(anyhow!("The Docmost sign-in window was closed before authentication completed."))
            } else {
                Err(anyhow!(helper_exit_error_message(code)))
            }
        }
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use tempfile::TempDir;

    const ORIGIN: &str = "https://docs.example.com";

    #[tokio::test]
    async fn failed_session_replacement_cannot_restore_the_previous_identity() -> Result<()> {
        unsafe { std::env::set_var("DOCMOST_DISABLE_KEYRING", "1") };
        let temp = TempDir::new()?;
        let manager = AuthManager::new(
            StartupConfig {
                base_url: Some(ORIGIN.to_string()),
                allow_insecure_credential_file: true,
                ..StartupConfig::default()
            },
            Some(temp.path().to_path_buf()),
        )?;
        manager
            .store
            .write_config(&StoredConfig {
                base_url: ORIGIN.to_string(),
                email: "identity-a@example.com".to_string(),
                last_authenticated_at: "2026-08-30T00:00:00.000Z".to_string(),
            })
            .await?;
        manager
            .store
            .write_session(&StoredSession {
                origin: Some(ORIGIN.to_string()),
                email: Some("identity-a@example.com".to_string()),
                token: "identity-a-token".to_string(),
                expires_at: None,
                saved_at: "2026-08-30T00:00:00.000Z".to_string(),
            })
            .await?;

        manager.store.fail_next_session_write();
        let error = manager
            .persist_authenticated_state(
                LoginInput {
                    base_url: ORIGIN.to_string(),
                    email: "identity-b@example.com".to_string(),
                    password: "synthetic-b-password".to_string(),
                    remember_password: false,
                },
                "identity-b-token".to_string(),
                None,
                "2026-08-30T01:00:00.000Z".to_string(),
            )
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("Injected session persistence failure")
        );
        assert_eq!(
            manager.store.read_config().await?.unwrap().email,
            "identity-b@example.com"
        );
        assert_eq!(manager.store.read_session(ORIGIN).await?, None);
        assert_eq!(manager.store.read_credentials(ORIGIN).await?, None);
        Ok(())
    }

    #[test]
    fn saved_session_requires_exact_identity_binding() {
        let origin = CanonicalDocmostOrigin::parse(ORIGIN, false).unwrap();
        let config = StoredConfig {
            base_url: ORIGIN.to_string(),
            email: "identity-b@example.com".to_string(),
            last_authenticated_at: "2026-08-30T01:00:00.000Z".to_string(),
        };
        let mut session = StoredSession {
            origin: Some(ORIGIN.to_string()),
            email: Some("identity-a@example.com".to_string()),
            token: "identity-a-token".to_string(),
            expires_at: None,
            saved_at: "2026-08-30T00:00:00.000Z".to_string(),
        };

        assert!(!saved_session_matches_config(
            &config, &session, &origin, false
        ));
        session.email = None;
        assert!(!saved_session_matches_config(
            &config, &session, &origin, false
        ));
        session.email = Some(config.email.clone());
        assert!(saved_session_matches_config(
            &config, &session, &origin, false
        ));
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tokio::{
        process::Command,
        time::{Duration, sleep, timeout},
    };

    use super::wait_for_authentication_completion;
    use crate::auth::{
        local_server::{LocalAuthDefaults, LocalAuthServer},
        webview::{AuthWindowHandle, AuthWindowMode},
    };

    #[tokio::test]
    async fn native_success_exit_still_waits_for_auth_completion() -> Result<()> {
        let mut auth_server = LocalAuthServer::new(
            LocalAuthDefaults::default(),
            |_input| async move { Ok(()) },
            Some(5_000),
        );
        let auth_session = auth_server.start().await?;

        let login_url = reqwest::Url::parse(&auth_session.login_url)?;
        let flow = login_url
            .query_pairs()
            .find(|(name, _)| name == "flow")
            .expect("flow query")
            .1
            .into_owned();
        let origin = format!(
            "{}://{}:{}",
            login_url.scheme(),
            login_url.host_str().expect("host"),
            login_url.port().expect("port")
        );
        let auth_url = format!("{origin}/auth");
        tokio::spawn(async move {
            sleep(Duration::from_millis(50)).await;
            let _ = reqwest::Client::new()
                .post(&auth_url)
                .header(reqwest::header::ORIGIN, origin)
                .header("x-docmost-auth-flow", flow)
                .json(&serde_json::json!({
                    "baseUrl": "https://docs.example.com",
                    "email": "jane@example.com",
                    "password": "synthetic-test-value",
                    "rememberPassword": false
                }))
                .send()
                .await;
        });

        let child = Command::new(std::env::current_exe()?)
            .arg("--help")
            .spawn()?;
        let mut auth_window = AuthWindowHandle::test_handle(AuthWindowMode::Native, child);

        let result = timeout(
            Duration::from_secs(2),
            wait_for_authentication_completion(&mut auth_server, &mut auth_window),
        )
        .await?;

        auth_window.close().await?;
        auth_server.stop().await?;

        result
    }
}
