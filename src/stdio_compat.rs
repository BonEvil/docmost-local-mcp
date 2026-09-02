//! Bounded compatibility preflight for Atlas MCP 2.0's discovery probe.
//!
//! `rmcp` 0.6 implements the initialize-first MCP handshake. Current Atlas first
//! probes `server/discover` and falls back to that handshake when a legacy server
//! returns JSON-RPC `METHOD_NOT_FOUND`. Feeding the probe into `rmcp` closes the
//! transport before Atlas can fall back, so this module rejects exactly one valid
//! Atlas discovery probe at the byte-stream boundary and preserves the subsequent
//! `initialize` frame for `rmcp`.
//!
//! The preflight runs before `DocmostMcpServer` is constructed. It therefore cannot
//! open the credential store, authenticate, contact Docmost, register anything, or
//! dispatch a tool.

use std::{fmt, io::Cursor, time::Duration};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Chain},
    time::timeout,
};

pub const MAX_PREFLIGHT_MESSAGE_BYTES: usize = 64 * 1024;
pub const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(10);

const DISCOVER_METHOD: &str = "server/discover";
const INITIALIZE_METHOD: &str = "initialize";
const ATLAS_PROTOCOL_VERSION: &str = "2026-07-28";
const PROTOCOL_VERSION_META_KEY: &str = "io.modelcontextprotocol/protocolVersion";
const CLIENT_INFO_META_KEY: &str = "io.modelcontextprotocol/clientInfo";
const CLIENT_CAPABILITIES_META_KEY: &str = "io.modelcontextprotocol/clientCapabilities";

pub type PreflightReader<R> = Chain<Cursor<Vec<u8>>, R>;

#[derive(Debug, PartialEq, Eq)]
pub enum PreflightError {
    Timeout,
    Oversized,
    Malformed,
    Reordered,
    Replayed,
    Unsupported,
    Io,
}

impl fmt::Display for PreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Timeout => "pre-handshake input timed out",
            Self::Oversized => "pre-handshake message exceeded the size limit",
            Self::Malformed => "pre-handshake message was malformed",
            Self::Reordered => "pre-handshake message order was rejected",
            Self::Replayed => "pre-handshake request was replayed",
            Self::Unsupported => "pre-handshake method was unsupported",
            Self::Io => "pre-handshake I/O failed",
        })
    }
}

impl std::error::Error for PreflightError {}

/// Consume only the compatibility preflight and return a reader whose first
/// frame is the untouched standards-compliant `initialize` request.
pub async fn negotiate<R, W>(
    reader: R,
    writer: &mut W,
) -> Result<PreflightReader<R>, PreflightError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    negotiate_with_timeout(reader, writer, PREFLIGHT_TIMEOUT).await
}

async fn negotiate_with_timeout<R, W>(
    mut reader: R,
    writer: &mut W,
    deadline: Duration,
) -> Result<PreflightReader<R>, PreflightError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let first = read_frame(&mut reader, deadline).await?;
    let first_message = parse_request(&first)?;

    match method(&first_message)? {
        INITIALIZE_METHOD => Ok(Cursor::new(first).chain(reader)),
        DISCOVER_METHOD => {
            validate_discover(&first_message)?;
            write_method_not_found(writer, request_id(&first_message)?).await?;

            let initialize = read_frame(&mut reader, deadline).await?;
            let initialize_message = parse_request(&initialize)?;
            let initialize_id = request_id(&initialize_message)?;
            if initialize_id == request_id(&first_message)? {
                return Err(PreflightError::Replayed);
            }
            match method(&initialize_message)? {
                INITIALIZE_METHOD => Ok(Cursor::new(initialize).chain(reader)),
                DISCOVER_METHOD => Err(PreflightError::Replayed),
                _ => Err(PreflightError::Reordered),
            }
        }
        _ => Err(PreflightError::Unsupported),
    }
}

async fn read_frame<R>(reader: &mut R, deadline: Duration) -> Result<Vec<u8>, PreflightError>
where
    R: AsyncRead + Unpin,
{
    timeout(deadline, async {
        let mut frame = Vec::with_capacity(1024);
        loop {
            let byte = reader.read_u8().await.map_err(|_| PreflightError::Io)?;
            frame.push(byte);
            if frame.len() > MAX_PREFLIGHT_MESSAGE_BYTES {
                return Err(PreflightError::Oversized);
            }
            if byte == b'\n' {
                return Ok(frame);
            }
        }
    })
    .await
    .map_err(|_| PreflightError::Timeout)?
}

fn parse_request(frame: &[u8]) -> Result<Value, PreflightError> {
    let message: Value = serde_json::from_slice(frame).map_err(|_| PreflightError::Malformed)?;
    let object = message.as_object().ok_or(PreflightError::Malformed)?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || !object.contains_key("id")
        || !object.contains_key("method")
    {
        return Err(PreflightError::Malformed);
    }
    request_id(&message)?;
    Ok(message)
}

fn request_id(message: &Value) -> Result<&Value, PreflightError> {
    let id = message.get("id").ok_or(PreflightError::Malformed)?;
    if id.is_string() || id.is_number() {
        Ok(id)
    } else {
        Err(PreflightError::Malformed)
    }
}

fn method(message: &Value) -> Result<&str, PreflightError> {
    message
        .get("method")
        .and_then(Value::as_str)
        .ok_or(PreflightError::Malformed)
}

fn validate_discover(message: &Value) -> Result<(), PreflightError> {
    let params = message
        .get("params")
        .and_then(Value::as_object)
        .ok_or(PreflightError::Malformed)?;
    if params.len() != 1 {
        return Err(PreflightError::Malformed);
    }
    let metadata = params
        .get("_meta")
        .and_then(Value::as_object)
        .ok_or(PreflightError::Malformed)?;
    if metadata
        .get(PROTOCOL_VERSION_META_KEY)
        .and_then(Value::as_str)
        != Some(ATLAS_PROTOCOL_VERSION)
        || !metadata
            .get(CLIENT_INFO_META_KEY)
            .is_some_and(Value::is_object)
        || !metadata
            .get(CLIENT_CAPABILITIES_META_KEY)
            .is_some_and(Value::is_object)
    {
        return Err(PreflightError::Malformed);
    }
    Ok(())
}

async fn write_method_not_found<W>(writer: &mut W, id: &Value) -> Result<(), PreflightError>
where
    W: AsyncWrite + Unpin,
{
    let response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": -32601, "message": "Method not found"}
    });
    let mut bytes = serde_json::to_vec(&response).map_err(|_| PreflightError::Io)?;
    bytes.push(b'\n');
    writer
        .write_all(&bytes)
        .await
        .map_err(|_| PreflightError::Io)?;
    writer.flush().await.map_err(|_| PreflightError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    fn discover(id: u64) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": DISCOVER_METHOD,
            "params": {"_meta": {
                PROTOCOL_VERSION_META_KEY: ATLAS_PROTOCOL_VERSION,
                CLIENT_INFO_META_KEY: {"name": "Atlas", "version": "test"},
                CLIENT_CAPABILITIES_META_KEY: {}
            }}
        })
        .to_string()
            + "\n"
    }

    fn initialize(id: u64) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": INITIALIZE_METHOD,
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "ordinary-client", "version": "1"}
            }
        })
        .to_string()
            + "\n"
    }

    async fn run(input: Vec<u8>) -> (Result<Vec<u8>, PreflightError>, Vec<u8>) {
        let (mut input_writer, input_reader) = tokio::io::duplex(input.len().max(1) + 1);
        input_writer.write_all(&input).await.unwrap();
        drop(input_writer);
        let mut output = Vec::new();
        let result =
            negotiate_with_timeout(input_reader, &mut output, Duration::from_millis(50)).await;
        let result = match result {
            Ok(mut reader) => {
                let mut retained = Vec::new();
                reader.read_to_end(&mut retained).await.unwrap();
                Ok(retained)
            }
            Err(error) => Err(error),
        };
        (result, output)
    }

    #[tokio::test]
    async fn initialize_first_is_preserved_byte_for_byte() {
        let input = initialize(1).into_bytes();
        let (result, output) = run(input.clone()).await;
        assert_eq!(result.unwrap(), input);
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn atlas_discovery_gets_legacy_fallback_without_reaching_rmcp() {
        let initialization = initialize(2);
        let input = (discover(1) + &initialization).into_bytes();
        let (result, output) = run(input).await;
        assert_eq!(result.unwrap(), initialization.as_bytes());
        let response: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(response["id"], 1);
        assert_eq!(response["error"]["code"], -32601);
        assert_eq!(response["error"]["message"], "Method not found");
        assert!(response.get("result").is_none());
    }

    #[tokio::test]
    async fn repeated_discovery_is_rejected_as_replay() {
        let (result, _) = run((discover(1) + &discover(2)).into_bytes()).await;
        assert_eq!(result.unwrap_err(), PreflightError::Replayed);
    }

    #[tokio::test]
    async fn non_initialize_after_discovery_is_rejected_as_reordered() {
        let unsupported =
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}\n";
        let (result, _) = run((discover(1) + unsupported).into_bytes()).await;
        assert_eq!(result.unwrap_err(), PreflightError::Reordered);
    }

    #[tokio::test]
    async fn reused_request_id_is_rejected_as_replay() {
        let (result, _) = run((discover(7) + &initialize(7)).into_bytes()).await;
        assert_eq!(result.unwrap_err(), PreflightError::Replayed);
    }

    #[tokio::test]
    async fn reordered_notification_is_rejected() {
        let message = b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n".to_vec();
        let (result, _) = run(message).await;
        assert_eq!(result.unwrap_err(), PreflightError::Malformed);
    }

    #[tokio::test]
    async fn unsupported_pre_handshake_tool_call_is_rejected() {
        let message =
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{}}\n".to_vec();
        let (result, _) = run(message).await;
        assert_eq!(result.unwrap_err(), PreflightError::Unsupported);
    }

    #[tokio::test]
    async fn malformed_discovery_metadata_is_rejected() {
        let message = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{\"_meta\":{}}}\n".to_vec();
        let (result, output) = run(message).await;
        assert_eq!(result.unwrap_err(), PreflightError::Malformed);
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn oversized_pre_handshake_message_is_rejected() {
        let mut message = vec![b' '; MAX_PREFLIGHT_MESSAGE_BYTES + 1];
        message.push(b'\n');
        let (result, output) = run(message).await;
        assert_eq!(result.unwrap_err(), PreflightError::Oversized);
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn incomplete_pre_handshake_message_times_out() {
        let (writer, reader) = tokio::io::duplex(64);
        let _keep_open = writer;
        let mut output = Vec::new();
        let result = negotiate_with_timeout(reader, &mut output, Duration::from_millis(10)).await;
        assert_eq!(result.unwrap_err(), PreflightError::Timeout);
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn missing_initialize_after_discovery_times_out_after_bounded_response() {
        let (mut writer, reader) = tokio::io::duplex(2048);
        writer.write_all(discover(1).as_bytes()).await.unwrap();
        let _keep_open = writer;
        let mut output = Vec::new();
        let result = negotiate_with_timeout(reader, &mut output, Duration::from_millis(10)).await;
        assert_eq!(result.unwrap_err(), PreflightError::Timeout);
        let response: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(response["error"]["code"], -32601);
    }
}
