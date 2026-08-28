use chrono::Utc;
use serde::Serialize;
use serde_json::{Map, Value};

// Diagnostic fields are an allowlist, not a denylist: unknown future fields are omitted.
// In particular, origins, URLs, email addresses, payloads, errors, tokens, cookies,
// search strings, page bodies and comments can never reach stderr through `debug_log`.
const SAFE_FIELDS: &[&str] = &[
    "endpointClass",
    "requestId",
    "requestBytes",
    "responseBytes",
    "status",
    "ok",
    "retryOnUnauthorized",
    "hasConfig",
    "hasSession",
    "hasCredentials",
    "loopbackAuth",
    "mode",
    "contentType",
    "baseUrlReadonly",
    "version",
    "fieldNames",
];

pub fn debug_log<T>(scope: &str, message: &str, details: Option<&T>)
where
    T: Serialize + ?Sized,
{
    if !debug_enabled() {
        return;
    }

    let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let prefix = format!("[docmost-local-mcp][{timestamp}][{scope}]");

    match details {
        Some(details) => {
            let serialized = serde_json::to_value(details)
                .map(|value| sanitize_details(&value))
                .and_then(|value| serde_json::to_string(&value))
                .unwrap_or_else(|_| "{}".to_string());
            eprintln!("{prefix} {message} {serialized}");
        }
        None => eprintln!("{prefix} {message}"),
    }
}

pub fn sanitize_details(details: &Value) -> Value {
    let Some(object) = details.as_object() else {
        return Value::Object(Map::new());
    };
    let safe = object
        .iter()
        .filter(|(name, value)| {
            SAFE_FIELDS.contains(&name.as_str())
                && (value.is_boolean()
                    || value.is_number()
                    || value.is_string()
                    || (name.as_str() == "fieldNames"
                        && value
                            .as_array()
                            .is_some_and(|items| items.iter().all(Value::is_string))))
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    Value::Object(safe)
}

#[cfg(test)]
mod tests {
    use super::sanitize_details;

    #[test]
    fn diagnostics_allow_only_reviewed_metadata() {
        let private = serde_json::json!({
            "endpointClass": "pages/update",
            "requestId": 7,
            "requestBytes": 123,
            "fieldNames": ["content", "pageId"],
            "password": "private-password",
            "token": "private-token",
            "cookie": "authToken=private-cookie",
            "payload": {"content": "private-page-body"},
            "error": "private-error-excerpt",
            "email": "private@example.test",
            "baseUrl": "https://private.example.test",
            "query": "private-search-text",
            "markdown": "private-markdown",
            "comment": "private-comment"
        });
        let serialized = sanitize_details(&private).to_string();

        assert_eq!(
            serialized,
            r#"{"endpointClass":"pages/update","fieldNames":["content","pageId"],"requestBytes":123,"requestId":7}"#
        );
        for forbidden in [
            "private-password",
            "private-token",
            "private-cookie",
            "private-page-body",
            "private-error",
            "private@example",
            "private.example",
            "private-search",
            "private-markdown",
            "private-comment",
        ] {
            assert!(!serialized.contains(forbidden), "leaked {forbidden}");
        }
    }
}

pub fn debug_enabled() -> bool {
    matches!(
        std::env::var("DEBUG_DOCMOST_MCP").ok().as_deref(),
        Some("1") | Some("true")
    )
}
