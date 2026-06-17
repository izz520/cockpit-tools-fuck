use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::infra::{atomic_write, paths};
use crate::models::account::{CodexAccount, CodexAccountView, CodexAuthMode, TokenBundle};
use crate::models::auth::{CodexAuthFile, CodexAuthTokens};
use crate::models::error::{AppError, AppResult};
use crate::services::{account_service, auth_file_service};

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTH_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
const SCOPES: &str =
    "openid profile email offline_access api.connectors.read api.connectors.invoke";
const ORIGINATOR: &str = "codex_vscode";
const CALLBACK_PORT: u16 = 1455;
const OAUTH_TIMEOUT_SECONDS: i64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStartResult {
    pub login_id: String,
    pub auth_url: String,
    pub redirect_uri: String,
    pub expires_at: i64,
    pub listener_started: bool,
    pub listener_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStatusResult {
    pub step: OAuthStatusStep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OAuthStatusStep {
    Started,
    CallbackSubmitted,
    Expired,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingOAuth {
    login_id: String,
    auth_url: String,
    redirect_uri: String,
    code_verifier: String,
    state: String,
    expires_at: i64,
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    id_token: String,
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

const ACCESS_TOKEN_REFRESH_SKEW_SECONDS: i64 = 120;

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn pending_oauth_path() -> AppResult<std::path::PathBuf> {
    Ok(paths::app_data_dir()?.join("pending-oauth.json"))
}

fn save_pending(pending: &PendingOAuth) -> AppResult<()> {
    let content = serde_json::to_vec_pretty(pending).map_err(|err| {
        AppError::new(
            "CODEX_OAUTH_STATE_SERIALIZE_FAILED",
            format!("Failed to serialize OAuth state: {}", err),
            "Start OAuth again.",
        )
    })?;
    atomic_write::write_atomic(&pending_oauth_path()?, &content)
}

fn clear_pending() -> AppResult<()> {
    let path = pending_oauth_path()?;
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path).map_err(|err| {
        AppError::new(
            "CODEX_OAUTH_STATE_CLEAR_FAILED",
            format!("Failed to clear {}: {}", path.display(), err),
            "Delete the pending OAuth file manually, then retry.",
        )
    })
}

fn load_pending() -> AppResult<PendingOAuth> {
    let path = pending_oauth_path()?;
    let content = fs::read_to_string(&path).map_err(|err| {
        AppError::new(
            "CODEX_OAUTH_STATE_NOT_FOUND",
            format!("Failed to read {}: {}", path.display(), err),
            "Start OAuth login again.",
        )
    })?;
    let pending: PendingOAuth = serde_json::from_str(&content).map_err(|err| {
        AppError::new(
            "CODEX_OAUTH_STATE_INVALID",
            format!("Failed to parse pending OAuth state: {}", err),
            "Start OAuth login again.",
        )
    })?;
    if pending.expires_at <= now_timestamp() {
        let _ = clear_pending();
        return Err(AppError::new(
            "CODEX_OAUTH_EXPIRED",
            "OAuth login has expired.",
            "Start OAuth login again.",
        ));
    }
    Ok(pending)
}

fn generate_token() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    URL_SAFE_NO_PAD.encode(bytes)
}

fn generate_code_challenge(code_verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect::<Vec<_>>(),
        })
        .collect()
}

fn build_auth_url(redirect_uri: &str, code_challenge: &str, state: &str) -> String {
    format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&id_token_add_organizations=true&codex_cli_simplified_flow=true&state={}&originator={}",
        AUTH_ENDPOINT,
        CLIENT_ID,
        percent_encode(redirect_uri),
        percent_encode(SCOPES),
        code_challenge,
        state,
        percent_encode(ORIGINATOR)
    )
}

fn decode_query_component(value: &str) -> String {
    let mut bytes = Vec::new();
    let raw = value.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == b'%' && index + 2 < raw.len() {
            if let Ok(hex) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                bytes.push(hex);
                index += 3;
                continue;
            }
        }
        bytes.push(if raw[index] == b'+' { b' ' } else { raw[index] });
        index += 1;
    }
    String::from_utf8(bytes).unwrap_or_else(|_| value.to_string())
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.trim();
            if key.is_empty() {
                return None;
            }
            Some((
                key.to_string(),
                decode_query_component(parts.next().unwrap_or_default()),
            ))
        })
        .collect()
}

fn callback_query(callback_url: &str) -> AppResult<String> {
    let trimmed = callback_url.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "CODEX_OAUTH_CALLBACK_EMPTY",
            "OAuth callback URL is empty.",
            "Paste the full callback URL from the browser.",
        ));
    }
    let query = trimmed
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or(trimmed);
    Ok(query.trim_start_matches('?').to_string())
}

fn http_response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn callback_url_from_http_request(request: &str) -> AppResult<String> {
    let request_line = request.lines().next().ok_or_else(|| {
        AppError::new(
            "CODEX_OAUTH_CALLBACK_REQUEST_EMPTY",
            "OAuth callback request is empty.",
            "Paste the full callback URL manually.",
        )
    })?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();

    if method != "GET" {
        return Err(AppError::new(
            "CODEX_OAUTH_CALLBACK_METHOD_UNSUPPORTED",
            "OAuth callback request method is not supported.",
            "Paste the full callback URL manually.",
        ));
    }

    if target.starts_with("http://") || target.starts_with("https://") {
        return Ok(target.to_string());
    }

    if !target.starts_with("/auth/callback") {
        return Err(AppError::new(
            "CODEX_OAUTH_CALLBACK_PATH_UNSUPPORTED",
            "OAuth callback request path is not supported.",
            "Paste the full callback URL manually.",
        ));
    }

    Ok(format!("http://localhost:{}{}", CALLBACK_PORT, target))
}

fn handle_callback_stream(login_id: &str, stream: &mut TcpStream) -> AppResult<bool> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buffer = [0_u8; 4096];
    let bytes_read = stream.read(&mut buffer).map_err(|err| {
        AppError::new(
            "CODEX_OAUTH_CALLBACK_READ_FAILED",
            format!("Failed to read OAuth callback request: {}", err),
            "Paste the full callback URL manually.",
        )
    })?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let callback_url = match callback_url_from_http_request(&request) {
        Ok(callback_url) => callback_url,
        Err(error) if error.code == "CODEX_OAUTH_CALLBACK_PATH_UNSUPPORTED" => {
            let response = http_response(
                "404 Not Found",
                "Codex Lite OAuth callback path was not found.",
            );
            let _ = stream.write_all(response.as_bytes());
            return Ok(false);
        }
        Err(error) => return Err(error),
    };

    submit_callback_url(login_id.to_string(), callback_url)?;
    let response = http_response(
        "200 OK",
        "Codex Lite received the OAuth callback. Return to the app and the account will be added automatically.",
    );
    let _ = stream.write_all(response.as_bytes());
    Ok(true)
}

fn should_keep_listening(login_id: &str, expires_at: i64) -> bool {
    if expires_at <= now_timestamp() {
        return false;
    }

    match load_pending() {
        Ok(pending) => pending.login_id == login_id && pending.code.is_none(),
        Err(_) => false,
    }
}

fn spawn_callback_listener(login_id: String, expires_at: i64) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", CALLBACK_PORT)).map_err(|err| {
        format!(
            "Failed to start OAuth callback listener on 127.0.0.1:{}: {}",
            CALLBACK_PORT, err
        )
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("Failed to configure OAuth callback listener: {}", err))?;

    thread::spawn(move || {
        while should_keep_listening(&login_id, expires_at) {
            match listener.accept() {
                Ok((mut stream, _)) => match handle_callback_stream(&login_id, &mut stream) {
                    Ok(true) => break,
                    Ok(false) => {}
                    Err(error) => {
                        let response = http_response("400 Bad Request", &error.message);
                        let _ = stream.write_all(response.as_bytes());
                    }
                },
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                }
                Err(_) => break,
            }
        }
    });

    Ok(())
}

pub fn start_login() -> AppResult<OAuthStartResult> {
    let code_verifier = generate_token();
    let code_challenge = generate_code_challenge(&code_verifier);
    let state = generate_token();
    let login_id = generate_token();
    let redirect_uri = format!("http://localhost:{}/auth/callback", CALLBACK_PORT);
    let auth_url = build_auth_url(&redirect_uri, &code_challenge, &state);
    let expires_at = now_timestamp() + OAUTH_TIMEOUT_SECONDS;

    let pending = PendingOAuth {
        login_id: login_id.clone(),
        auth_url: auth_url.clone(),
        redirect_uri: redirect_uri.clone(),
        code_verifier,
        state,
        expires_at,
        code: None,
    };
    save_pending(&pending)?;
    let listener_result = spawn_callback_listener(login_id.clone(), expires_at);
    let listener_error = listener_result.err();

    Ok(OAuthStartResult {
        login_id,
        auth_url,
        redirect_uri,
        expires_at,
        listener_started: listener_error.is_none(),
        listener_error,
    })
}

pub fn submit_callback_url(login_id: String, callback_url: String) -> AppResult<()> {
    let mut pending = load_pending()?;
    if pending.login_id != login_id {
        return Err(AppError::new(
            "CODEX_OAUTH_LOGIN_ID_MISMATCH",
            "OAuth login id does not match the active login.",
            "Use the latest OAuth login window or start again.",
        ));
    }

    let params = parse_query(&callback_query(&callback_url)?);
    let code = params
        .get("code")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::new(
                "CODEX_OAUTH_CALLBACK_MISSING_CODE",
                "OAuth callback does not contain a code parameter.",
                "Paste the full callback URL from the browser.",
            )
        })?;
    let state = params
        .get("state")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::new(
                "CODEX_OAUTH_CALLBACK_MISSING_STATE",
                "OAuth callback does not contain a state parameter.",
                "Paste the full callback URL from the browser.",
            )
        })?;

    if state != pending.state {
        return Err(AppError::new(
            "CODEX_OAUTH_STATE_MISMATCH",
            "OAuth callback state does not match the active login.",
            "Start OAuth login again.",
        ));
    }

    pending.code = Some(code);
    save_pending(&pending)
}

pub fn login_status(login_id: String) -> AppResult<OAuthStatusResult> {
    let pending = match load_pending() {
        Ok(pending) => pending,
        Err(error) if error.code == "CODEX_OAUTH_STATE_NOT_FOUND" => {
            return Ok(OAuthStatusResult {
                step: OAuthStatusStep::Missing,
            });
        }
        Err(error) if error.code == "CODEX_OAUTH_EXPIRED" => {
            return Ok(OAuthStatusResult {
                step: OAuthStatusStep::Expired,
            });
        }
        Err(error) => return Err(error),
    };

    if pending.login_id != login_id {
        return Err(AppError::new(
            "CODEX_OAUTH_LOGIN_ID_MISMATCH",
            "OAuth login id does not match the active login.",
            "Use the latest OAuth login window or start again.",
        ));
    }

    Ok(OAuthStatusResult {
        step: if pending.code.is_some() {
            OAuthStatusStep::CallbackSubmitted
        } else {
            OAuthStatusStep::Started
        },
    })
}

async fn exchange_code(pending: &PendingOAuth) -> AppResult<TokenResponse> {
    let code = pending.code.as_deref().ok_or_else(|| {
        AppError::new(
            "CODEX_OAUTH_CODE_NOT_READY",
            "OAuth authorization code is not ready.",
            "Complete browser authorization and submit the callback URL first.",
        )
    })?;

    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", pending.redirect_uri.as_str()),
        ("client_id", CLIENT_ID),
        ("code_verifier", pending.code_verifier.as_str()),
    ];
    let response = reqwest::Client::new()
        .post(TOKEN_ENDPOINT)
        .form(&params)
        .send()
        .await
        .map_err(|err| {
            AppError::new(
                "CODEX_OAUTH_TOKEN_REQUEST_FAILED",
                format!("OAuth token request failed: {}", err),
                "Check network connectivity and try completing OAuth again.",
            )
            .retryable()
        })?;
    let status = response.status();
    let body = response.text().await.map_err(|err| {
        AppError::new(
            "CODEX_OAUTH_TOKEN_RESPONSE_READ_FAILED",
            format!("Failed to read OAuth token response: {}", err),
            "Try completing OAuth again.",
        )
        .retryable()
    })?;

    if !status.is_success() {
        let token_error = parse_token_error(&body);
        let mut error = AppError::new(
            "CODEX_OAUTH_TOKEN_EXCHANGE_FAILED",
            token_exchange_error_message(status.as_u16(), body.len(), token_error.as_ref()),
            "Start OAuth login again.",
        );
        error.details = token_error;
        return Err(error);
    }

    serde_json::from_str(&body).map_err(|err| {
        AppError::new(
            "CODEX_OAUTH_TOKEN_RESPONSE_INVALID",
            format!("OAuth token response is invalid: {}", err),
            "Start OAuth login again.",
        )
    })
}

fn parse_token_error(body: &str) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let mut details = serde_json::Map::new();

    if let Some(error) = value.get("error") {
        if let Some(error_code) = error.as_str() {
            details.insert(
                "error".to_string(),
                serde_json::Value::String(error_code.to_string()),
            );
        } else if let Some(error_code) = error.get("code").and_then(serde_json::Value::as_str) {
            details.insert(
                "error".to_string(),
                serde_json::Value::String(error_code.to_string()),
            );
        }
    }

    if let Some(description) = value
        .get("error_description")
        .and_then(serde_json::Value::as_str)
        .or_else(|| value.get("message").and_then(serde_json::Value::as_str))
    {
        details.insert(
            "errorDescription".to_string(),
            serde_json::Value::String(description.to_string()),
        );
    }

    if details.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(details))
    }
}

fn token_exchange_error_message(
    status_code: u16,
    body_len: usize,
    token_error: Option<&serde_json::Value>,
) -> String {
    let mut message = format!(
        "OAuth token exchange failed with HTTP {} and body length {}.",
        status_code, body_len
    );

    if let Some(error_code) = token_error
        .and_then(|value| value.get("error"))
        .and_then(serde_json::Value::as_str)
    {
        message.push_str(&format!(" error={}.", error_code));
    }

    message
}

fn token_refresh_error_message(
    status_code: u16,
    body_len: usize,
    token_error: Option<&serde_json::Value>,
) -> String {
    let mut message = format!(
        "OAuth token refresh failed with HTTP {} and body length {}.",
        status_code, body_len
    );

    if let Some(error_code) = token_error
        .and_then(|value| value.get("error"))
        .and_then(serde_json::Value::as_str)
    {
        message.push_str(&format!(" error={}.", error_code));
    }

    message
}

fn access_token_needs_refresh(access_token: &str) -> bool {
    auth_file_service::decode_jwt_payload(access_token)
        .ok()
        .and_then(|payload| payload.exp)
        .is_none_or(|expires_at| expires_at <= now_timestamp() + ACCESS_TOKEN_REFRESH_SKEW_SECONDS)
}

fn refreshed_account_from_token_response(
    account: &CodexAccount,
    response: TokenResponse,
) -> CodexAccount {
    let mut refreshed = account.clone();
    let refresh_token = response
        .refresh_token
        .or_else(|| account.token_bundle.as_ref()?.refresh_token.clone());
    refreshed.token_bundle = Some(TokenBundle {
        id_token: response.id_token,
        access_token: response.access_token,
        refresh_token,
    });
    refreshed.quota_error = None;
    refreshed.updated_at = now_timestamp();

    let auth = auth_file_service::auth_file_from_account(&refreshed);
    if let Ok(auth) = auth {
        if let Ok(parsed) = auth_file_service::account_from_auth(auth) {
            refreshed.email = parsed.email;
            refreshed.account_id = parsed.account_id;
            refreshed.user_id = parsed.user_id;
            refreshed.plan_type = parsed.plan_type;
            refreshed.subscription_active_until = parsed.subscription_active_until;
            if refreshed.display_name == "Codex OAuth Account" {
                refreshed.display_name = parsed.display_name;
            }
        }
    }

    refreshed
}

async fn refresh_token_bundle(account: &CodexAccount) -> AppResult<CodexAccount> {
    if account.auth_mode != CodexAuthMode::OAuth {
        return Err(AppError::new(
            "CODEX_ACCOUNT_NOT_OAUTH",
            "Only OAuth accounts can refresh OAuth tokens.",
            "Choose an OAuth account and try again.",
        ));
    }
    let tokens = account.token_bundle.as_ref().ok_or_else(|| {
        AppError::new(
            "CODEX_ACCOUNT_MISSING_CREDENTIALS",
            "Selected OAuth account has no token bundle.",
            "Re-import this account before switching.",
        )
    })?;
    let refresh_token = tokens
        .refresh_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::new(
                "CODEX_OAUTH_REFRESH_TOKEN_MISSING",
                "OAuth account has no refresh token.",
                "Re-authenticate this account before switching.",
            )
        })?;

    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", CLIENT_ID),
    ];
    let response = reqwest::Client::new()
        .post(TOKEN_ENDPOINT)
        .form(&params)
        .send()
        .await
        .map_err(|err| {
            AppError::new(
                "CODEX_OAUTH_REFRESH_REQUEST_FAILED",
                format!("OAuth token refresh request failed: {}", err),
                "Check network connectivity and try again.",
            )
            .retryable()
        })?;
    let status = response.status();
    let body = response.text().await.map_err(|err| {
        AppError::new(
            "CODEX_OAUTH_REFRESH_RESPONSE_READ_FAILED",
            format!("Failed to read OAuth refresh response: {}", err),
            "Try switching again.",
        )
        .retryable()
    })?;

    if !status.is_success() {
        let token_error = parse_token_error(&body);
        let mut error = AppError::new(
            "CODEX_OAUTH_REFRESH_FAILED",
            token_refresh_error_message(status.as_u16(), body.len(), token_error.as_ref()),
            "Re-authenticate this account, then switch again.",
        );
        error.details = token_error;
        return Err(error);
    }

    let token_response = serde_json::from_str(&body).map_err(|err| {
        AppError::new(
            "CODEX_OAUTH_REFRESH_RESPONSE_INVALID",
            format!("OAuth refresh response is invalid: {}", err),
            "Re-authenticate this account, then switch again.",
        )
    })?;
    Ok(refreshed_account_from_token_response(
        account,
        token_response,
    ))
}

pub async fn refresh_account_if_needed(
    account: &CodexAccount,
    force: bool,
) -> AppResult<CodexAccount> {
    if account.auth_mode != CodexAuthMode::OAuth {
        return Ok(account.clone());
    }
    let access_token = account
        .token_bundle
        .as_ref()
        .map(|bundle| bundle.access_token.as_str())
        .ok_or_else(|| {
            AppError::new(
                "CODEX_ACCOUNT_MISSING_CREDENTIALS",
                "Selected OAuth account has no token bundle.",
                "Re-import this account before switching.",
            )
        })?;

    #[cfg(test)]
    if access_token.starts_with("fixture-") {
        return Ok(account.clone());
    }

    if !force && !access_token_needs_refresh(access_token) {
        return Ok(account.clone());
    }

    let refreshed = refresh_token_bundle(account).await?;
    account_service::upsert_account(refreshed.clone())?;
    Ok(refreshed)
}

pub async fn complete_login(login_id: String) -> AppResult<CodexAccountView> {
    let pending = load_pending()?;
    if pending.login_id != login_id {
        return Err(AppError::new(
            "CODEX_OAUTH_LOGIN_ID_MISMATCH",
            "OAuth login id does not match the active login.",
            "Use the latest OAuth login window or start again.",
        ));
    }
    let token = exchange_code(&pending).await?;
    clear_pending()?;

    let auth = CodexAuthFile {
        auth_mode: Some("oauth".to_string()),
        openai_api_key: None,
        base_url: None,
        tokens: Some(CodexAuthTokens {
            id_token: token.id_token,
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            account_id: None,
        }),
        last_refresh: Some(serde_json::json!(now_timestamp())),
    };
    let account = auth_file_service::account_from_auth(auth)?;
    account_service::upsert_account(account)
}

pub fn cancel_login(login_id: Option<String>) -> AppResult<()> {
    match load_pending() {
        Ok(pending) => {
            if let Some(expected_login_id) = login_id {
                if pending.login_id != expected_login_id {
                    return Err(AppError::new(
                        "CODEX_OAUTH_LOGIN_ID_MISMATCH",
                        "OAuth login id does not match the active login.",
                        "Use the latest OAuth login window or start again.",
                    ));
                }
            }
            clear_pending()
        }
        Err(error) if error.code == "CODEX_OAUTH_STATE_NOT_FOUND" => Ok(()),
        Err(error) => Err(error),
    }
}

pub fn is_callback_port_in_use() -> AppResult<bool> {
    match std::net::TcpListener::bind(("127.0.0.1", CALLBACK_PORT)) {
        Ok(listener) => {
            drop(listener);
            Ok(false)
        }
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => Ok(true),
        Err(err) => Err(AppError::new(
            "CODEX_OAUTH_PORT_CHECK_FAILED",
            format!(
                "Failed to check OAuth callback port {}: {}",
                CALLBACK_PORT, err
            ),
            "Check local firewall or port permissions.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_auth_url, callback_query, callback_url_from_http_request, cancel_login,
        decode_query_component, generate_token, load_pending, login_status, parse_query,
        parse_token_error, percent_encode, save_pending, submit_callback_url,
        token_exchange_error_message, OAuthStatusStep, PendingOAuth,
    };
    use crate::infra::{paths, storage};
    use crate::test_support::TestEnv;

    #[test]
    fn percent_encodes_oauth_url_components() {
        assert_eq!(
            percent_encode("openid profile email"),
            "openid%20profile%20email"
        );
        assert_eq!(
            percent_encode("http://localhost:1455/auth/callback"),
            "http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback"
        );
    }

    #[test]
    fn builds_auth_url_with_pkce_state_and_originator() {
        let auth_url = build_auth_url(
            "http://localhost:1455/auth/callback",
            "fixture-code-challenge",
            "fixture-state",
        );

        assert!(auth_url.starts_with("https://auth.openai.com/oauth/authorize?"));
        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains("code_challenge=fixture-code-challenge"));
        assert!(auth_url.contains("state=fixture-state"));
        assert!(auth_url.contains("originator=codex_vscode"));
    }

    #[test]
    fn generated_oauth_tokens_meet_pkce_length_requirements() {
        let token = generate_token();

        assert_eq!(token.len(), 43);
        assert!(token
            .chars()
            .all(|item| item.is_ascii_alphanumeric() || item == '-' || item == '_'));
    }

    #[test]
    fn decodes_query_components() {
        assert_eq!(
            decode_query_component("fixture+code%2Fvalue"),
            "fixture code/value"
        );
    }

    #[test]
    fn parses_callback_url_query() {
        let query = callback_query(
            "http://localhost:1455/auth/callback?code=fixture-code&state=fixture-state",
        )
        .expect("Callback URL should produce query");
        let params = parse_query(&query);

        assert_eq!(params.get("code").map(String::as_str), Some("fixture-code"));
        assert_eq!(
            params.get("state").map(String::as_str),
            Some("fixture-state")
        );
    }

    #[test]
    fn rejects_empty_callback_url() {
        let error = callback_query(" ").expect_err("Empty callback should fail");

        assert_eq!(error.code, "CODEX_OAUTH_CALLBACK_EMPTY");
    }

    #[test]
    fn parse_query_omits_empty_keys() {
        let params = parse_query("=missing&code=fixture-code&&state=fixture-state");

        assert_eq!(params.get("code").map(String::as_str), Some("fixture-code"));
        assert_eq!(
            params.get("state").map(String::as_str),
            Some("fixture-state")
        );
        assert!(!params.contains_key(""));
    }

    #[test]
    fn token_exchange_errors_include_safe_error_code() {
        let body = r#"{"error":"invalid_grant","error_description":"Code verifier is invalid"}"#;
        let details = parse_token_error(body).expect("token error should parse");
        let message = token_exchange_error_message(400, body.len(), Some(&details));

        assert_eq!(
            details.get("error").and_then(serde_json::Value::as_str),
            Some("invalid_grant")
        );
        assert!(message.contains("HTTP 400"));
        assert!(message.contains("error=invalid_grant"));
    }

    #[test]
    fn parses_callback_http_request_target() {
        let callback_url = callback_url_from_http_request(
            "GET /auth/callback?code=fixture-code&state=fixture-state HTTP/1.1\r\nHost: localhost:1455\r\n\r\n",
        )
        .expect("Callback request should parse");

        assert_eq!(
            callback_url,
            "http://localhost:1455/auth/callback?code=fixture-code&state=fixture-state"
        );
    }

    #[test]
    fn rejects_non_callback_http_request_path() {
        let error = callback_url_from_http_request(
            "GET /favicon.ico HTTP/1.1\r\nHost: localhost:1455\r\n\r\n",
        )
        .expect_err("Non-callback path should fail");

        assert_eq!(error.code, "CODEX_OAUTH_CALLBACK_PATH_UNSUPPORTED");
    }

    fn fixture_pending(expires_at: i64) -> PendingOAuth {
        PendingOAuth {
            login_id: "login-fixture".to_string(),
            auth_url: "https://auth.openai.com/oauth/authorize?state=state-fixture".to_string(),
            redirect_uri: "http://localhost:1455/auth/callback".to_string(),
            code_verifier: "verifier-fixture".to_string(),
            state: "state-fixture".to_string(),
            expires_at,
            code: None,
        }
    }

    #[test]
    fn submit_callback_url_rejects_state_mismatch() {
        let _env = TestEnv::new("oauth-state-mismatch");
        save_pending(&fixture_pending(chrono::Utc::now().timestamp() + 60))
            .expect("pending OAuth state should save");

        let error = submit_callback_url(
            "login-fixture".to_string(),
            "http://localhost:1455/auth/callback?code=fixture-code&state=wrong-state".to_string(),
        )
        .expect_err("state mismatch should fail");

        assert_eq!(error.code, "CODEX_OAUTH_STATE_MISMATCH");
    }

    #[test]
    fn login_status_reports_callback_submitted() {
        let _env = TestEnv::new("oauth-status-callback-submitted");
        let mut pending = fixture_pending(chrono::Utc::now().timestamp() + 60);
        pending.code = Some("fixture-code".to_string());
        save_pending(&pending).expect("pending OAuth state should save");

        let status = login_status("login-fixture".to_string()).expect("status should load");

        assert!(matches!(status.step, OAuthStatusStep::CallbackSubmitted));
    }

    #[test]
    fn load_pending_clears_expired_state() {
        let _env = TestEnv::new("oauth-expired");
        save_pending(&fixture_pending(chrono::Utc::now().timestamp() - 1))
            .expect("expired pending OAuth state should save");

        let error = load_pending().expect_err("expired state should fail");

        assert_eq!(error.code, "CODEX_OAUTH_EXPIRED");
        assert!(!paths::app_data_dir()
            .expect("data dir should resolve")
            .join("pending-oauth.json")
            .exists());
    }

    #[test]
    fn cancel_login_clears_pending_state_without_creating_account() {
        let _env = TestEnv::new("oauth-cancel");
        save_pending(&fixture_pending(chrono::Utc::now().timestamp() + 60))
            .expect("pending OAuth state should save");

        cancel_login(Some("login-fixture".to_string())).expect("cancel should succeed");
        let error = load_pending().expect_err("pending state should be cleared");
        let accounts = storage::load_accounts_file().expect("accounts file should load");

        assert_eq!(error.code, "CODEX_OAUTH_STATE_NOT_FOUND");
        assert!(accounts.accounts.is_empty());
    }
}
