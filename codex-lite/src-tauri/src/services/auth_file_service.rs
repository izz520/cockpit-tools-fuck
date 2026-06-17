use std::fs;
use std::path::Path;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

use crate::models::account::{CodexAccount, CodexAuthMode, TokenBundle};
use crate::models::auth::{CodexAuthFile, JwtPayload};
use crate::models::error::{AppError, AppResult};

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Codex writes `last_refresh` as an RFC3339 timestamp with microsecond
/// precision (e.g. `2025-02-20T14:30:45.123456Z`), not a Unix integer.
fn now_last_refresh() -> serde_json::Value {
    serde_json::Value::String(
        chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.6fZ")
            .to_string(),
    )
}

pub fn read_auth_file(path: &Path) -> AppResult<CodexAuthFile> {
    let content = fs::read_to_string(path).map_err(|err| {
        AppError::new(
            "CODEX_AUTH_READ_FAILED",
            format!("Failed to read {}: {}", path.display(), err),
            "Check that Codex is installed and the auth path is readable.",
        )
    })?;
    parse_auth_json(&content)
}

pub fn parse_auth_json(content: &str) -> AppResult<CodexAuthFile> {
    serde_json::from_str(content).map_err(|err| {
        AppError::new(
            "CODEX_AUTH_INVALID_FORMAT",
            format!("Codex auth JSON is invalid: {}", err),
            "Choose a valid Codex auth.json file.",
        )
    })
}

fn parse_auth_value(value: &serde_json::Value) -> AppResult<CodexAuthFile> {
    serde_json::from_value(value.clone()).map_err(|err| {
        AppError::new(
            "CODEX_AUTH_INVALID_FORMAT",
            format!("Codex auth JSON is invalid: {}", err),
            "Choose a valid Codex auth JSON file.",
        )
    })
}

fn read_string_path(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
}

fn auth_from_portable_value(value: &serde_json::Value) -> Option<CodexAuthFile> {
    let id_token = read_string_path(value, &["id_token"])
        .or_else(|| read_string_path(value, &["idToken"]))
        .or_else(|| read_string_path(value, &["credentials", "id_token"]))
        .or_else(|| read_string_path(value, &["credentials", "idToken"]))?;
    let access_token = read_string_path(value, &["access_token"])
        .or_else(|| read_string_path(value, &["accessToken"]))
        .or_else(|| read_string_path(value, &["credentials", "access_token"]))
        .or_else(|| read_string_path(value, &["credentials", "accessToken"]))?;
    let refresh_token = read_string_path(value, &["refresh_token"])
        .or_else(|| read_string_path(value, &["refreshToken"]))
        .or_else(|| read_string_path(value, &["session_token"]))
        .or_else(|| read_string_path(value, &["sessionToken"]))
        .or_else(|| read_string_path(value, &["credentials", "refresh_token"]))
        .or_else(|| read_string_path(value, &["credentials", "refreshToken"]))
        .or_else(|| read_string_path(value, &["credentials", "session_token"]))
        .or_else(|| read_string_path(value, &["credentials", "sessionToken"]));
    let account_id = read_string_path(value, &["account_id"])
        .or_else(|| read_string_path(value, &["accountId"]))
        .or_else(|| read_string_path(value, &["credentials", "account_id"]))
        .or_else(|| read_string_path(value, &["credentials", "accountId"]))
        .or_else(|| read_string_path(value, &["credentials", "chatgpt_account_id"]));

    Some(CodexAuthFile {
        auth_mode: Some("oauth".to_string()),
        openai_api_key: None,
        base_url: read_string_path(value, &["api_base_url"])
            .or_else(|| read_string_path(value, &["apiBaseUrl"]))
            .or_else(|| read_string_path(value, &["base_url"]))
            .or_else(|| read_string_path(value, &["baseUrl"]))
            .or_else(|| read_string_path(value, &["credentials", "api_base_url"]))
            .or_else(|| read_string_path(value, &["credentials", "apiBaseUrl"]))
            .or_else(|| read_string_path(value, &["credentials", "base_url"]))
            .or_else(|| read_string_path(value, &["credentials", "baseUrl"])),
        tokens: Some(crate::models::auth::CodexAuthTokens {
            id_token,
            access_token,
            refresh_token,
            account_id,
        }),
        last_refresh: value.get("last_refresh").cloned().or_else(|| {
            value
                .get("lastRefresh")
                .cloned()
                .or_else(|| value.get("credentials")?.get("last_refresh").cloned())
        }),
    })
}

fn account_from_import_value(value: &serde_json::Value) -> AppResult<Option<CodexAccount>> {
    let auth = match parse_auth_value(value) {
        Ok(auth) if auth.tokens.is_some() || auth.openai_api_key.is_some() => Some(auth),
        _ => auth_from_portable_value(value),
    };
    let Some(auth) = auth else {
        return Ok(None);
    };

    let mut account = account_from_auth(auth)?;
    if let Some(display_name) = read_string_path(value, &["display_name"])
        .or_else(|| read_string_path(value, &["displayName"]))
        .or_else(|| read_string_path(value, &["name"]))
    {
        account.display_name = display_name;
    }
    if let Some(email) = read_string_path(value, &["email"])
        .or_else(|| read_string_path(value, &["credentials", "email"]))
    {
        account.email = Some(email.clone());
        if account.display_name == "Codex OAuth Account" {
            account.display_name = email;
        }
    }
    if let Some(plan_type) = read_string_path(value, &["plan_type"])
        .or_else(|| read_string_path(value, &["planType"]))
        .or_else(|| read_string_path(value, &["credentials", "plan_type"]))
        .or_else(|| read_string_path(value, &["credentials", "planType"]))
    {
        account.plan_type = Some(plan_type);
    }
    if let Some(subscription_active_until) = read_string_path(value, &["subscription_active_until"])
        .or_else(|| read_string_path(value, &["subscriptionActiveUntil"]))
        .or_else(|| read_string_path(value, &["credentials", "subscription_active_until"]))
        .or_else(|| read_string_path(value, &["credentials", "subscriptionActiveUntil"]))
    {
        account.subscription_active_until = Some(subscription_active_until);
    }
    if let Some(account_id) = read_string_path(value, &["account_id"])
        .or_else(|| read_string_path(value, &["accountId"]))
        .or_else(|| read_string_path(value, &["credentials", "account_id"]))
        .or_else(|| read_string_path(value, &["credentials", "accountId"]))
        .or_else(|| read_string_path(value, &["credentials", "chatgpt_account_id"]))
    {
        account.account_id = Some(account_id);
    }
    Ok(Some(account))
}

fn collect_import_candidate_values(value: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(items) = value.as_array() {
        return items.clone();
    }
    if let Some(accounts) = value.get("accounts").and_then(|item| item.as_array()) {
        return accounts.clone();
    }
    vec![value.clone()]
}

pub fn accounts_from_auth_json(content: &str) -> AppResult<Vec<CodexAccount>> {
    let value = serde_json::from_str::<serde_json::Value>(content).map_err(|err| {
        AppError::new(
            "CODEX_AUTH_INVALID_FORMAT",
            format!("Codex auth JSON is invalid: {}", err),
            "Choose a valid Codex auth JSON file.",
        )
    })?;
    let mut accounts = Vec::new();
    for candidate in collect_import_candidate_values(&value) {
        if let Some(account) = account_from_import_value(&candidate)? {
            accounts.push(account);
        }
    }
    if accounts.is_empty() {
        return Err(AppError::new(
            "CODEX_AUTH_INVALID_FORMAT",
            "Codex auth file does not contain OAuth tokens or an API key.",
            "Import a valid Codex auth JSON file, CPA token export, sub2api export, or API Key JSON.",
        ));
    }
    Ok(accounts)
}

pub fn account_from_auth(auth: CodexAuthFile) -> AppResult<CodexAccount> {
    if let Some(api_key_value) = auth
        .openai_api_key
        .as_ref()
        .and_then(|value| value.as_str())
    {
        let api_key = api_key_value.trim();
        if !api_key.is_empty() {
            return Ok(api_key_account(api_key.to_string(), auth.base_url));
        }
    }

    let tokens = auth.tokens.ok_or_else(|| {
        AppError::new(
            "CODEX_AUTH_INVALID_FORMAT",
            "Codex auth file does not contain OAuth tokens or an API key.",
            "Import a valid Codex auth.json file or use Token/API Key import.",
        )
    })?;
    let id_payload = decode_jwt_payload(&tokens.id_token)?;
    let access_payload = decode_jwt_payload(&tokens.access_token).ok();
    let email = trim_optional_ref(id_payload.email.as_deref())
        .or_else(|| {
            id_payload
                .profile_data
                .as_ref()
                .and_then(|value| trim_optional_ref(value.email.as_deref()))
        })
        .or_else(|| {
            access_payload
                .as_ref()
                .and_then(|value| trim_optional_ref(value.email.as_deref()))
        });
    let user_id = id_payload
        .auth_data
        .as_ref()
        .and_then(|value| trim_optional_ref(value.chatgpt_user_id.as_deref()))
        .or_else(|| trim_optional_ref(id_payload.sub.as_deref()))
        .or_else(|| {
            access_payload.as_ref().and_then(|value| {
                value
                    .auth_data
                    .as_ref()
                    .and_then(|auth_data| trim_optional_ref(auth_data.chatgpt_user_id.as_deref()))
                    .or_else(|| trim_optional_ref(value.sub.as_deref()))
            })
        });
    let account_id = trim_optional_ref(tokens.account_id.as_deref())
        .or_else(|| extract_account_id_from_payload(access_payload.as_ref()))
        .or_else(|| extract_account_id_from_payload(Some(&id_payload)));
    let plan_type = extract_plan_type_from_payload(access_payload.as_ref())
        .or_else(|| extract_plan_type_from_payload(Some(&id_payload)));
    let subscription_active_until =
        extract_subscription_active_until_from_payload(access_payload.as_ref())
            .or_else(|| extract_subscription_active_until_from_payload(Some(&id_payload)));
    let id_seed = user_id
        .clone()
        .or_else(|| account_id.clone())
        .or_else(|| email.clone())
        .unwrap_or_else(|| tokens.id_token.clone());
    let id = stable_id(&id_seed);
    let now = now_timestamp();

    Ok(CodexAccount {
        id,
        display_name: email
            .clone()
            .unwrap_or_else(|| "Codex OAuth Account".to_string()),
        email,
        auth_mode: CodexAuthMode::OAuth,
        bound_oauth_account_id: None,
        account_id,
        user_id,
        plan_type,
        subscription_active_until,
        token_bundle: Some(TokenBundle {
            id_token: tokens.id_token,
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
        }),
        api_key: None,
        api_base_url: auth.base_url,
        quota: None,
        quota_error: None,
        tags: Vec::new(),
        note: None,
        created_at: now,
        updated_at: now,
        last_used_at: None,
    })
}

fn trim_optional_ref(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
}

fn extract_account_id_from_payload(payload: Option<&JwtPayload>) -> Option<String> {
    let auth_data = payload?.auth_data.as_ref()?;
    trim_optional_ref(auth_data.chatgpt_account_id.as_deref())
        .or_else(|| trim_optional_ref(auth_data.account_id.as_deref()))
}

fn extract_plan_type_from_payload(payload: Option<&JwtPayload>) -> Option<String> {
    trim_optional_ref(payload?.auth_data.as_ref()?.chatgpt_plan_type.as_deref())
}

fn extract_subscription_active_until_from_payload(payload: Option<&JwtPayload>) -> Option<String> {
    trim_optional_ref(
        payload?
            .auth_data
            .as_ref()?
            .chatgpt_subscription_active_until
            .as_deref(),
    )
}

/// Recovers the ChatGPT account id from an OAuth id_token's claims. Older builds
/// stored accounts without `account_id`, but Codex needs it inside `tokens` for
/// the auth file to be accepted, so we re-derive it from the JWT on switch.
fn account_id_from_id_token(id_token: &str) -> Option<String> {
    let payload = decode_jwt_payload(id_token).ok()?;
    extract_account_id_from_payload(Some(&payload))
}

pub fn auth_file_from_account(account: &CodexAccount) -> AppResult<CodexAuthFile> {
    match account.auth_mode {
        CodexAuthMode::OAuth => {
            let tokens = account.token_bundle.as_ref().ok_or_else(|| {
                AppError::new(
                    "CODEX_ACCOUNT_MISSING_CREDENTIALS",
                    "Selected OAuth account has no token bundle.",
                    "Re-import this account before switching.",
                )
            })?;
            let account_id = account
                .account_id
                .clone()
                .or_else(|| account_id_from_id_token(&tokens.id_token));
            Ok(CodexAuthFile {
                // Codex treats an absent auth_mode as ChatGPT/OAuth mode. Writing
                // "oauth" here is not a value Codex recognizes and breaks login.
                auth_mode: None,
                // OAuth auth files carry OPENAI_API_KEY as an explicit null.
                openai_api_key: Some(serde_json::Value::Null),
                base_url: account.api_base_url.clone(),
                tokens: Some(crate::models::auth::CodexAuthTokens {
                    id_token: tokens.id_token.clone(),
                    access_token: tokens.access_token.clone(),
                    refresh_token: tokens.refresh_token.clone(),
                    account_id,
                }),
                last_refresh: Some(now_last_refresh()),
            })
        }
        CodexAuthMode::ApiKey => {
            let api_key = account.api_key.as_ref().ok_or_else(|| {
                AppError::new(
                    "CODEX_ACCOUNT_MISSING_CREDENTIALS",
                    "Selected API Key account has no API key.",
                    "Re-import this account before switching.",
                )
            })?;
            Ok(CodexAuthFile {
                auth_mode: Some("apikey".to_string()),
                openai_api_key: Some(serde_json::Value::String(api_key.clone())),
                base_url: account.api_base_url.clone(),
                tokens: None,
                last_refresh: Some(now_last_refresh()),
            })
        }
    }
}

pub fn stable_id(seed: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    format!("codex_{}", hex_prefix(&digest, 16))
}

fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>()[..chars]
        .to_string()
}

fn api_key_account(api_key: String, base_url: Option<String>) -> CodexAccount {
    let id = stable_id(&api_key);
    let now = now_timestamp();
    CodexAccount {
        id,
        display_name: "Codex API Key".to_string(),
        email: None,
        auth_mode: CodexAuthMode::ApiKey,
        bound_oauth_account_id: None,
        account_id: None,
        user_id: None,
        plan_type: Some("API_KEY".to_string()),
        subscription_active_until: None,
        token_bundle: None,
        api_key: Some(api_key),
        api_base_url: base_url,
        quota: None,
        quota_error: None,
        tags: Vec::new(),
        note: None,
        created_at: now,
        updated_at: now,
        last_used_at: None,
    }
}

pub fn decode_jwt_payload(id_token: &str) -> AppResult<JwtPayload> {
    let payload = id_token.split('.').nth(1).ok_or_else(|| {
        AppError::new(
            "CODEX_TOKEN_INVALID",
            "ID token does not look like a JWT.",
            "Check the token and try again.",
        )
    })?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).map_err(|err| {
        AppError::new(
            "CODEX_TOKEN_INVALID",
            format!("Failed to decode ID token: {}", err),
            "Check the token and try again.",
        )
    })?;
    serde_json::from_slice(&decoded).map_err(|err| {
        AppError::new(
            "CODEX_TOKEN_INVALID",
            format!("Failed to parse ID token payload: {}", err),
            "Check the token and try again.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        account_from_auth, accounts_from_auth_json, auth_file_from_account, parse_auth_json,
    };
    use crate::models::account::CodexAuthMode;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    const OAUTH_FIXTURE: &str = include_str!("../../../fixtures/redacted-auth/oauth.json");
    const API_KEY_FIXTURE: &str = include_str!("../../../fixtures/redacted-auth/api-key.json");
    const INVALID_FIXTURE: &str =
        include_str!("../../../fixtures/redacted-auth/invalid-empty.json");

    fn make_jwt(payload: serde_json::Value) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
        let body = URL_SAFE_NO_PAD.encode(payload.to_string());
        format!("{header}.{body}.signature")
    }

    #[test]
    fn parses_oauth_fixture_into_account() {
        let auth = parse_auth_json(OAUTH_FIXTURE).expect("OAuth fixture should parse");
        let account = account_from_auth(auth).expect("OAuth fixture should project to account");

        assert_eq!(account.auth_mode, CodexAuthMode::OAuth);
        assert_eq!(account.email.as_deref(), Some("fixture.oauth@example.test"));
        assert_eq!(account.user_id.as_deref(), Some("user_fixture_123"));
        assert_eq!(account.account_id.as_deref(), Some("acct_fixture_redacted"));
        assert!(account.token_bundle.is_some());
        assert!(account.api_key.is_none());
    }

    #[test]
    fn parses_account_id_and_plan_from_access_token_auth_claims() {
        let id_token = make_jwt(serde_json::json!({
            "email": "fixture.oauth@example.test",
            "sub": "user_fixture_123"
        }));
        let access_token = make_jwt(serde_json::json!({
            "https://api.openai.com/auth": {
                "chatgpt_user_id": "user_fixture_123",
                "chatgpt_account_id": "acct_from_access",
                "chatgpt_plan_type": "plus"
            }
        }));
        let auth = parse_auth_json(
            &serde_json::json!({
                "auth_mode": "oauth",
                "tokens": {
                    "id_token": id_token,
                    "access_token": access_token,
                    "refresh_token": "refresh_fixture"
                }
            })
            .to_string(),
        )
        .expect("OAuth JSON should parse");

        let account = account_from_auth(auth).expect("OAuth auth should project to account");

        assert_eq!(account.account_id.as_deref(), Some("acct_from_access"));
        assert_eq!(account.plan_type.as_deref(), Some("plus"));
    }

    #[test]
    fn imports_portable_cpa_token_object() {
        let id_token = make_jwt(serde_json::json!({
            "email": "portable@example.test",
            "https://api.openai.com/auth": {
                "chatgpt_user_id": "user_portable",
                "chatgpt_account_id": "acct_portable",
                "chatgpt_plan_type": "team"
            }
        }));
        let accounts = accounts_from_auth_json(
            &serde_json::json!({
                "type": "codex",
                "email": "portable@example.test",
                "id_token": id_token,
                "access_token": make_jwt(serde_json::json!({})),
                "refresh_token": "refresh_fixture",
                "account_id": "acct_portable"
            })
            .to_string(),
        )
        .expect("portable CPA JSON should import");

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].email.as_deref(), Some("portable@example.test"));
        assert_eq!(accounts[0].account_id.as_deref(), Some("acct_portable"));
        assert_eq!(accounts[0].plan_type.as_deref(), Some("team"));
    }

    #[test]
    fn imports_portable_array_and_sub2api_accounts() {
        let id_token = make_jwt(serde_json::json!({
            "email": "sub2api@example.test",
            "https://api.openai.com/auth": {
                "chatgpt_user_id": "user_sub2api",
                "chatgpt_account_id": "acct_sub2api"
            }
        }));
        let array_payload = serde_json::json!([
            {
                "type": "codex",
                "id_token": id_token,
                "access_token": make_jwt(serde_json::json!({})),
                "refresh_token": "refresh_fixture"
            }
        ]);
        let sub2api_payload = serde_json::json!({
            "type": "sub2api-data",
            "version": 1,
            "accounts": [
                {
                    "name": "Fixture Team",
                    "platform": "openai",
                    "type": "oauth",
                    "credentials": {
                        "id_token": array_payload[0]["id_token"],
                        "access_token": array_payload[0]["access_token"],
                        "refresh_token": "refresh_fixture",
                        "email": "sub2api@example.test",
                        "chatgpt_account_id": "acct_sub2api",
                        "plan_type": "team"
                    }
                }
            ]
        });

        let array_accounts =
            accounts_from_auth_json(&array_payload.to_string()).expect("array JSON should import");
        let sub2api_accounts = accounts_from_auth_json(&sub2api_payload.to_string())
            .expect("sub2api JSON should import");

        assert_eq!(array_accounts.len(), 1);
        assert_eq!(sub2api_accounts.len(), 1);
        assert_eq!(sub2api_accounts[0].display_name, "Fixture Team");
        assert_eq!(
            sub2api_accounts[0].email.as_deref(),
            Some("sub2api@example.test")
        );
    }

    #[test]
    fn parses_api_key_fixture_into_account() {
        let auth = parse_auth_json(API_KEY_FIXTURE).expect("API key fixture should parse");
        let account = account_from_auth(auth).expect("API key fixture should project to account");

        assert_eq!(account.auth_mode, CodexAuthMode::ApiKey);
        assert_eq!(account.display_name, "Codex API Key");
        assert_eq!(account.plan_type.as_deref(), Some("API_KEY"));
        assert_eq!(
            account.api_base_url.as_deref(),
            Some("https://api.openai.com/v1")
        );
        assert!(account.api_key.is_some());
        assert!(account.token_bundle.is_none());
    }

    #[test]
    fn rejects_fixture_without_credentials() {
        let auth = parse_auth_json(INVALID_FIXTURE).expect("Invalid fixture JSON should parse");
        let error = account_from_auth(auth).expect_err("Missing credentials should fail");

        assert_eq!(error.code, "CODEX_AUTH_INVALID_FORMAT");
        assert!(!error.retryable);
    }

    #[test]
    fn rejects_malformed_json() {
        let error = parse_auth_json("{").expect_err("Malformed JSON should fail");

        assert_eq!(error.code, "CODEX_AUTH_INVALID_FORMAT");
    }

    #[test]
    fn round_trips_oauth_account_to_auth_file_without_view_leak() {
        let auth = parse_auth_json(OAUTH_FIXTURE).expect("OAuth fixture should parse");
        let account = account_from_auth(auth).expect("OAuth fixture should project to account");
        let view = account.to_view(false);
        let exported = auth_file_from_account(&account).expect("OAuth account should export");

        assert_eq!(view.email.as_deref(), Some("fixture.oauth@example.test"));
        assert!(exported.tokens.is_some());
        assert!(serde_json::to_value(view)
            .expect("view should serialize")
            .get("tokenBundle")
            .is_none());
    }

    #[test]
    fn exports_oauth_auth_file_with_codex_snake_case_keys() {
        let auth = parse_auth_json(OAUTH_FIXTURE).expect("OAuth fixture should parse");
        let account = account_from_auth(auth).expect("OAuth fixture should project to account");
        let exported = auth_file_from_account(&account).expect("OAuth account should export");
        let json = serde_json::to_value(&exported).expect("auth file should serialize");

        // Codex reads snake_case token keys; camelCase would fail to parse and force re-login.
        let tokens = json.get("tokens").expect("tokens object should be present");
        assert!(tokens.get("id_token").is_some());
        assert!(tokens.get("access_token").is_some());
        assert!(tokens.get("idToken").is_none());
        assert!(tokens.get("accessToken").is_none());

        // last_refresh must be an RFC3339 string, not an integer timestamp.
        let last_refresh = json
            .get("last_refresh")
            .and_then(|value| value.as_str())
            .expect("last_refresh should serialize as a string");
        assert!(last_refresh.ends_with('Z'));
        assert!(last_refresh.contains('T'));

        // OAuth auth files omit auth_mode and carry OPENAI_API_KEY as null.
        assert!(json.get("auth_mode").is_none());
        assert!(json
            .get("OPENAI_API_KEY")
            .map(|v| v.is_null())
            .unwrap_or(false));
        // account_id must be present inside tokens for Codex to accept the file.
        assert!(tokens.get("account_id").is_some());
    }

    #[test]
    fn reimports_camel_case_auth_file_written_by_older_builds() {
        // Older codex-lite builds wrote camelCase; those files must still import.
        let camel = serde_json::json!({
            "authMode": "oauth",
            "tokens": {
                "idToken": "legacy-id-token",
                "accessToken": "legacy-access",
                "refreshToken": "legacy-refresh",
                "accountId": "acct_legacy"
            },
            "lastRefresh": 1710000000
        })
        .to_string();

        let auth = parse_auth_json(&camel).expect("camelCase auth should still parse");
        let tokens = auth.tokens.expect("tokens should be read via aliases");
        assert_eq!(tokens.access_token, "legacy-access");
        assert_eq!(tokens.account_id.as_deref(), Some("acct_legacy"));
    }
}
