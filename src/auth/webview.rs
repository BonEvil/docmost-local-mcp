use anyhow::{Context, Result};
use tokio::process::Child;

use crate::{debug::debug_log, types::AuthWindowSession};

const EXIT_SUCCESS: i32 = 0;
const EXIT_CANCELLED: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthWindowMode {
    Native,
    Browser,
}

pub struct AuthWindowHandle {
    pub mode: AuthWindowMode,
    child: Option<Child>,
}

impl AuthWindowHandle {
    pub async fn wait_for_exit(&mut self) -> Result<Option<i32>> {
        match self.child.as_mut() {
            Some(child) => Ok(child.wait().await?.code()),
            None => Ok(None),
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
        }
        self.child = None;
        Ok(())
    }
}

pub async fn launch_auth_window(session: &AuthWindowSession) -> Result<AuthWindowHandle> {
    debug_log(
        "auth-helper",
        "Opening system browser for authentication",
        Some(&serde_json::json!({ "fallbackUrl": session.fallback_url })),
    );
    open::that(&session.fallback_url).context("Failed to open fallback browser window")?;
    Ok(AuthWindowHandle {
        mode: AuthWindowMode::Browser,
        child: None,
    })
}

pub async fn run_auth_window(
    url: String,
    success_url: String,
    title: String,
    width: u32,
    height: u32,
) -> Result<()> {
    let _ = (url, success_url, title, width, height);
    Err(anyhow::anyhow!(
        "Native authentication windows are no longer supported; use the system browser flow."
    ))
}

pub fn helper_exit_success(code: Option<i32>) -> bool {
    code == Some(EXIT_SUCCESS)
}

pub fn helper_exit_cancelled(code: Option<i32>) -> bool {
    code == Some(EXIT_CANCELLED)
}

pub fn helper_exit_error_message(code: Option<i32>) -> String {
    if helper_exit_cancelled(code) {
        return "The Docmost sign-in window was closed before authentication completed."
            .to_string();
    }

    if let Some(code) = code {
        return format!("The Docmost sign-in window exited unexpectedly (code {code}).");
    }

    "The Docmost sign-in window exited unexpectedly.".to_string()
}

#[cfg(test)]
impl AuthWindowHandle {
    pub(crate) fn test_handle(mode: AuthWindowMode, child: Child) -> Self {
        Self {
            mode,
            child: Some(child),
        }
    }
}
