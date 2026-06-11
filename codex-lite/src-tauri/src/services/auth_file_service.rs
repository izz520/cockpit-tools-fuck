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
        account_id,
        user_id,
        plan_type,
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
            Ok(CodexAuthFile {
                auth_mode: Some("oauth".to_string()),
                openai_api_key: None,
                base_url: account.api_base_url.clone(),
                tokens: Some(crate::models::auth::CodexAuthTokens {
                    id_token: tokens.id_token.clone(),
                    access_token: tokens.access_token.clone(),
                    refresh_token: tokens.refresh_token.clone(),
                    account_id: account.account_id.clone(),
                }),
                last_refresh: Some(serde_json::json!(now_timestamp())),
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
                last_refresh: Some(serde_json::json!(now_timestamp())),
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
        account_id: None,
        user_id: None,
        plan_type: Some("API_KEY".to_string()),
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

fn decode_jwt_payload(id_token: &str) -> AppResult<JwtPayload> {
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
    use super::{account_from_auth, auth_file_from_account, parse_auth_json};
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
}
