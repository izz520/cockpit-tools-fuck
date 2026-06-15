use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::infra::{atomic_write, paths, storage};
use crate::models::account::{CodexAccount, CodexAuthMode};
use crate::models::error::{AppError, AppResult};
use crate::services::codex_config_service;
use futures_util::StreamExt;
use reqwest::header::{HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Method, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

pub const GATEWAY_PROVIDER_BASE_URL: &str = "http://127.0.0.1:14567/v1";

const GATEWAY_HOST: &str = "127.0.0.1";
const GATEWAY_PORT: u16 = 14567;
const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const GATEWAY_STATE_FILE: &str = "codex_lite_gateway_state.json";
const MAX_REQUEST_BYTES: usize = 32 * 1024 * 1024;
const RESPONSE_PATH: &str = "/v1/responses";

static GATEWAY_RUNTIME: OnceLock<Mutex<Option<GatewayRuntime>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GatewayState {
    account_id: String,
    base_url: String,
}

#[derive(Debug)]
struct GatewayRuntime {
    account_id: String,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

#[derive(Debug)]
struct ParsedHttpRequest {
    method: Method,
    target: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn runtime_store() -> &'static Mutex<Option<GatewayRuntime>> {
    GATEWAY_RUNTIME.get_or_init(|| Mutex::new(None))
}

fn state_file_path() -> AppResult<PathBuf> {
    Ok(paths::app_data_dir()?.join(GATEWAY_STATE_FILE))
}

fn gateway_addr() -> SocketAddr {
    format!("{}:{}", GATEWAY_HOST, GATEWAY_PORT)
        .parse()
        .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], GATEWAY_PORT)))
}

fn save_gateway_state(account: &CodexAccount) -> AppResult<()> {
    let state = GatewayState {
        account_id: account.id.clone(),
        base_url: account
            .api_base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.to_string()),
    };
    let content = serde_json::to_vec_pretty(&state).map_err(|err| {
        AppError::new(
            "CODEX_GATEWAY_STATE_SERIALIZE_FAILED",
            format!("Failed to serialize local gateway state: {}", err),
            "Switch the account again.",
        )
    })?;
    atomic_write::write_atomic(&state_file_path()?, &content)
}

fn remove_gateway_state_file() {
    if let Ok(path) = state_file_path() {
        let _ = std::fs::remove_file(path);
    }
}

fn is_api_key_bound_to_oauth(account: &CodexAccount) -> bool {
    account.auth_mode == CodexAuthMode::ApiKey
        && account
            .bound_oauth_account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
}

fn current_api_key_bound_oauth_account() -> AppResult<Option<CodexAccount>> {
    let file = storage::load_accounts_file()?;
    let Some(current_id) = file.current_account_id.as_deref() else {
        return Ok(None);
    };
    Ok(file
        .accounts
        .into_iter()
        .find(|account| account.id == current_id && is_api_key_bound_to_oauth(account)))
}

fn load_current_gateway_account() -> AppResult<CodexAccount> {
    current_api_key_bound_oauth_account()?.ok_or_else(|| {
        AppError::new(
            "CODEX_GATEWAY_ACCOUNT_UNAVAILABLE",
            "Current account is not an API+OAuth account.",
            "Switch to the API+OAuth account again.",
        )
    })
}

fn upstream_base_url(account: &CodexAccount) -> AppResult<Url> {
    let base_url = account
        .api_base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_OPENAI_BASE_URL);
    let mut normalized = base_url.trim_end_matches('/').to_string();
    if !normalized.ends_with("/v1") {
        normalized.push_str("/v1");
    }
    Url::parse(&(normalized + "/")).map_err(|err| {
        AppError::new(
            "CODEX_GATEWAY_INVALID_BASE_URL",
            format!("Invalid API base URL: {}", err),
            "Edit the API account and switch again.",
        )
    })
}

fn strip_query(target: &str) -> &str {
    target
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(target)
}

fn target_is_responses(target: &str) -> bool {
    matches!(strip_query(target), RESPONSE_PATH | "/responses")
}

fn upstream_relative_target(target: &str) -> &str {
    let target = target.trim_start_matches('/');
    target.strip_prefix("v1/").unwrap_or(target)
}

fn remove_image_generation_tool(body: &mut Value) -> bool {
    let Some(object) = body.as_object_mut() else {
        return false;
    };
    let mut changed = false;

    if let Some(Value::Array(tools)) = object.get_mut("tools") {
        let before = tools.len();
        tools.retain(|tool| tool.get("type").and_then(Value::as_str) != Some("image_generation"));
        changed |= before != tools.len();
        if tools.is_empty() {
            object.remove("tools");
        }
    }

    if object.get("tool_choice").is_some_and(|choice| {
        choice.as_str() == Some("image_generation")
            || choice.get("type").and_then(Value::as_str) == Some("image_generation")
            || (choice.get("type").and_then(Value::as_str) == Some("tool")
                && choice.get("name").and_then(Value::as_str) == Some("image_generation"))
    }) {
        object.remove("tool_choice");
        changed = true;
    }

    changed
}

pub fn sanitize_responses_body(body: &[u8]) -> Vec<u8> {
    let Ok(mut value) = serde_json::from_slice::<Value>(body) else {
        return body.to_vec();
    };
    if !remove_image_generation_tool(&mut value) {
        return body.to_vec();
    }
    serde_json::to_vec(&value).unwrap_or_else(|_| body.to_vec())
}

fn content_length(headers: &HashMap<String, String>) -> Option<usize> {
    headers
        .get("content-length")
        .and_then(|value| value.trim().parse::<usize>().ok())
}

fn parse_head(head: &[u8]) -> Option<(Method, String, HashMap<String, String>)> {
    let head_text = String::from_utf8_lossy(head);
    let mut lines = head_text.lines();
    let request_line = lines.next()?;
    let mut request_parts = request_line.split_whitespace();
    let method = Method::from_bytes(request_parts.next()?.as_bytes()).ok()?;
    let target = request_parts.next()?.to_string();
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect();
    Some((method, target, headers))
}

async fn read_http_request(stream: &mut TcpStream) -> Option<ParsedHttpRequest> {
    let mut received = Vec::with_capacity(8192);
    let header_end;
    loop {
        let mut chunk = [0u8; 8192];
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        received.extend_from_slice(&chunk[..read]);
        if received.len() > MAX_REQUEST_BYTES {
            return None;
        }
        if let Some(index) = received.windows(4).position(|window| window == b"\r\n\r\n") {
            header_end = index + 4;
            break;
        }
    }

    let (method, target, headers) = parse_head(&received[..header_end])?;
    let expected_body_len = content_length(&headers).unwrap_or(0);
    let mut body = received[header_end..].to_vec();
    while body.len() < expected_body_len {
        let mut chunk = vec![0u8; expected_body_len - body.len()];
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        body.extend_from_slice(&chunk[..read]);
        if body.len() > MAX_REQUEST_BYTES {
            return None;
        }
    }
    body.truncate(expected_body_len);

    Some(ParsedHttpRequest {
        method,
        target,
        headers,
        body,
    })
}

fn error_json(status: u16, message: &str) -> Vec<u8> {
    let body = serde_json::json!({ "error": { "message": message } });
    let body = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
    let status_text = match status {
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        _ => "Error",
    };
    let mut response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        status_text,
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    response
}

async fn forward_request(request: ParsedHttpRequest) -> AppResult<reqwest::Response> {
    let account = load_current_gateway_account()?;
    let base_url = upstream_base_url(&account)?;
    let upstream_url = base_url
        .join(upstream_relative_target(&request.target))
        .map_err(|err| {
            AppError::new(
                "CODEX_GATEWAY_UPSTREAM_URL_FAILED",
                format!("Failed to build upstream URL: {}", err),
                "Check the API account base URL.",
            )
        })?;
    let api_key = account
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::new(
                "CODEX_GATEWAY_MISSING_API_KEY",
                "Selected API account has no API key.",
                "Edit the API account and switch again.",
            )
        })?;
    let client = Client::builder().no_proxy().build().map_err(|err| {
        AppError::new(
            "CODEX_GATEWAY_CLIENT_FAILED",
            format!("Failed to create local gateway client: {}", err),
            "Try switching the account again.",
        )
    })?;
    let mut upstream = client
        .request(request.method, upstream_url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key));

    for (name, value) in &request.headers {
        if matches!(
            name.as_str(),
            "authorization"
                | "host"
                | "content-length"
                | "connection"
                | "accept-encoding"
                | "proxy-connection"
                | "x-api-key"
        ) {
            continue;
        }
        let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) else {
            continue;
        };
        let Ok(header_value) = HeaderValue::from_str(value) else {
            continue;
        };
        upstream = upstream.header(header_name, header_value);
    }

    if !request.headers.contains_key("content-type") && !request.body.is_empty() {
        upstream = upstream.header(CONTENT_TYPE, "application/json");
    }
    let body = if target_is_responses(&request.target) {
        sanitize_responses_body(&request.body)
    } else {
        request.body
    };
    if !body.is_empty() {
        upstream = upstream.body(body);
    }

    upstream.send().await.map_err(|err| {
        AppError::new(
            "CODEX_GATEWAY_UPSTREAM_FAILED",
            format!("Local gateway upstream request failed: {}", err),
            "Check the API base URL and network connectivity.",
        )
    })
}

async fn write_upstream_response(
    stream: &mut TcpStream,
    response: reqwest::Response,
) -> std::io::Result<()> {
    let status = response.status();
    let status_text = status.canonical_reason().unwrap_or("OK");
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json");
    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        status.as_u16(),
        status_text,
        content_type
    );
    stream.write_all(headers.as_bytes()).await?;
    let mut body = response.bytes_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        if chunk.is_empty() {
            continue;
        }
        stream
            .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
            .await?;
        stream.write_all(&chunk).await?;
        stream.write_all(b"\r\n").await?;
    }
    stream.write_all(b"0\r\n\r\n").await
}

async fn handle_connection(mut stream: TcpStream) {
    let Some(request) = read_http_request(&mut stream).await else {
        let _ = stream
            .write_all(&error_json(400, "Invalid gateway request."))
            .await;
        let _ = stream.shutdown().await;
        return;
    };
    match forward_request(request).await {
        Ok(response) => {
            let _ = write_upstream_response(&mut stream, response).await;
        }
        Err(error) => {
            let _ = stream
                .write_all(&error_json(
                    502,
                    &format!("{}: {}", error.code, error.message),
                ))
                .await;
        }
    }
    let _ = stream.shutdown().await;
}

async fn serve(listener: TcpListener, mut shutdown: oneshot::Receiver<()>) {
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _)) => {
                        tokio::spawn(handle_connection(stream));
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

async fn stop_runtime_locked(runtime: GatewayRuntime) {
    let _ = runtime.shutdown.send(());
    let _ = runtime.task.await;
}

pub async fn ensure_for_account(account: &CodexAccount) -> AppResult<Option<String>> {
    if !is_api_key_bound_to_oauth(account) {
        stop().await;
        remove_gateway_state_file();
        return Ok(None);
    }

    let mut guard = runtime_store().lock().await;
    if let Some(runtime) = guard.as_ref() {
        if runtime.account_id == account.id {
            save_gateway_state(account)?;
            return Ok(Some(GATEWAY_PROVIDER_BASE_URL.to_string()));
        }
    }
    if let Some(runtime) = guard.take() {
        stop_runtime_locked(runtime).await;
    }

    let listener = TcpListener::bind(gateway_addr()).await.map_err(|err| {
        AppError::new(
            "CODEX_GATEWAY_BIND_FAILED",
            format!(
                "Failed to bind local Codex gateway on {}: {}",
                GATEWAY_PROVIDER_BASE_URL, err
            ),
            "Close the process using port 14567 and switch again.",
        )
    })?;
    let (shutdown, shutdown_rx) = oneshot::channel();
    let task = tokio::spawn(serve(listener, shutdown_rx));
    save_gateway_state(account)?;
    *guard = Some(GatewayRuntime {
        account_id: account.id.clone(),
        shutdown,
        task,
    });
    Ok(Some(GATEWAY_PROVIDER_BASE_URL.to_string()))
}

pub async fn restore_for_current_account() -> AppResult<()> {
    match current_api_key_bound_oauth_account()? {
        Some(account) => {
            let provider_base_url = ensure_for_account(&account).await?;
            codex_config_service::apply_account_config_with_provider_base_url(
                &account,
                &paths::default_codex_config_file()?,
                provider_base_url.as_deref(),
            )?;
        }
        None => {
            stop().await;
            remove_gateway_state_file();
        }
    }
    Ok(())
}

pub async fn stop() {
    let mut guard = runtime_store().lock().await;
    if let Some(runtime) = guard.take() {
        stop_runtime_locked(runtime).await;
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_responses_body;
    use serde_json::Value;

    fn has_image_generation_tool(value: &Value) -> bool {
        value
            .get("tools")
            .and_then(Value::as_array)
            .map(|tools| {
                tools.iter().any(|tool| {
                    tool.get("type").and_then(Value::as_str) == Some("image_generation")
                })
            })
            .unwrap_or(false)
    }

    #[test]
    fn sanitize_responses_body_removes_image_generation_tool_and_choice() {
        let body = br#"{
            "model": "gpt-5.4",
            "input": "hello",
            "tool_choice": { "type": "image_generation" },
            "tools": [
                { "type": "web_search_preview" },
                { "type": "image_generation", "output_format": "png" }
            ]
        }"#;

        let sanitized = sanitize_responses_body(body);
        let value: Value = serde_json::from_slice(&sanitized).expect("sanitized json");

        assert!(!has_image_generation_tool(&value));
        assert!(value.get("tool_choice").is_none());
        assert_eq!(
            value.pointer("/tools/0/type").and_then(Value::as_str),
            Some("web_search_preview")
        );
    }

    #[test]
    fn sanitize_responses_body_keeps_body_without_image_generation() {
        let body = br#"{"model":"gpt-5.4","input":"hello"}"#;

        assert_eq!(sanitize_responses_body(body), body);
    }
}
