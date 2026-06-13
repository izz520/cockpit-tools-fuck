use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, REFERER, USER_AGENT};
use serde::Deserialize;

use crate::infra::storage;
use crate::models::account::{CodexAccount, CodexAccountView, CodexAuthMode, CodexQuotaView};
use crate::models::error::{AppError, AppResult};

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const CHATGPT_WEB_REFERER: &str = "https://chatgpt.com/";
const CHATGPT_WEB_USER_AGENT: &str = "Mozilla/5.0 AppleWebKit/537.36 CodexLite/0.1";
const HTTP_ERROR_BODY_DISPLAY_MAX_CHARS: usize = 1200;

#[derive(Debug, Deserialize)]
struct WindowInfo {
    #[serde(default, rename = "used_percent")]
    used_percent: Option<i32>,
    #[serde(default, rename = "reset_after_seconds")]
    reset_after_seconds: Option<i64>,
    #[serde(default, rename = "reset_at")]
    reset_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RateLimitInfo {
    #[serde(default, rename = "primary_window")]
    primary_window: Option<WindowInfo>,
    #[serde(default, rename = "secondary_window")]
    secondary_window: Option<WindowInfo>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    #[serde(default, rename = "plan_type")]
    plan_type: Option<String>,
    #[serde(default, rename = "rate_limit")]
    rate_limit: Option<RateLimitInfo>,
}

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn normalize_remaining_percent(window: &WindowInfo) -> i32 {
    100 - window.used_percent.unwrap_or(0).clamp(0, 100)
}

fn normalize_reset_at(window: &WindowInfo) -> Option<i64> {
    window.reset_at.or_else(|| {
        window
            .reset_after_seconds
            .filter(|value| *value >= 0)
            .map(|value| now_timestamp() + value)
    })
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_http_error_body_for_display(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }

    let compact = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= HTTP_ERROR_BODY_DISPLAY_MAX_CHARS {
        return compact;
    }

    let mut truncated = compact
        .chars()
        .take(HTTP_ERROR_BODY_DISPLAY_MAX_CHARS)
        .collect::<String>();
    truncated.push_str("...(truncated)");
    truncated
}

fn extract_detail_code(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("detail")
        .and_then(|item| item.get("code"))
        .and_then(|item| item.as_str())
        .or_else(|| {
            value
                .get("error")
                .and_then(|item| item.get("code").or(Some(item)))
                .and_then(|item| item.as_str())
        })
        .or_else(|| value.get("code").and_then(|item| item.as_str()))
        .map(|item| item.to_string())
}

fn quota_error(
    code: &str,
    message: impl Into<String>,
    action: impl Into<String>,
    retryable: bool,
) -> AppError {
    let error = AppError::new(code, message, action);
    if retryable {
        error.retryable()
    } else {
        error
    }
}

fn classify_http_error(status: reqwest::StatusCode, body: &str) -> AppError {
    let detail_code = extract_detail_code(body);
    let display_body = normalize_http_error_body_for_display(body);
    let message = match detail_code {
        Some(code) => format!(
            "Quota API returned HTTP {} with code {} and body {}.",
            status.as_u16(),
            code,
            display_body
        ),
        None => format!(
            "Quota API returned HTTP {} with body {}.",
            status.as_u16(),
            display_body
        ),
    };

    match status.as_u16() {
        401 | 403 => quota_error(
            "CODEX_QUOTA_UNAUTHORIZED",
            message,
            "Re-authenticate this Codex account, then refresh quota again.",
            false,
        ),
        429 => quota_error(
            "CODEX_QUOTA_RATE_LIMITED",
            message,
            "Wait for the quota API rate limit to reset, then retry.",
            true,
        ),
        408 | 425 | 500..=599 => quota_error(
            "CODEX_QUOTA_TEMPORARY_FAILURE",
            message,
            "Wait a moment and retry quota refresh.",
            true,
        ),
        _ => quota_error(
            "CODEX_QUOTA_HTTP_FAILED",
            message,
            "Check the account state and try again.",
            false,
        ),
    }
}

fn parse_quota(body: &str) -> AppResult<(CodexQuotaView, Option<String>)> {
    let usage: UsageResponse = serde_json::from_str(body).map_err(|err| {
        AppError::new(
            "CODEX_QUOTA_INVALID_RESPONSE",
            format!("Quota API response is not valid JSON: {}", err),
            "Retry later. If it keeps failing, the upstream response shape may have changed.",
        )
    })?;

    let rate_limit = usage.rate_limit.as_ref();
    let primary = rate_limit.and_then(|item| item.primary_window.as_ref());
    let secondary = rate_limit.and_then(|item| item.secondary_window.as_ref());

    Ok((
        CodexQuotaView {
            hourly_remaining_percent: primary.map(normalize_remaining_percent),
            hourly_reset_at: primary.and_then(normalize_reset_at),
            weekly_remaining_percent: secondary.map(normalize_remaining_percent),
            weekly_reset_at: secondary.and_then(normalize_reset_at),
            updated_at: Some(now_timestamp()),
            stale: false,
        },
        trim_optional(usage.plan_type),
    ))
}

fn build_headers(account: &CodexAccount) -> AppResult<HeaderMap> {
    let tokens = account.token_bundle.as_ref().ok_or_else(|| {
        AppError::new(
            "CODEX_QUOTA_MISSING_TOKEN",
            "OAuth account has no access token bundle.",
            "Re-import or re-authenticate this account.",
        )
    })?;
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", tokens.access_token)).map_err(|err| {
            AppError::new(
                "CODEX_QUOTA_INVALID_TOKEN",
                format!("Failed to build Authorization header: {}", err),
                "Re-import this account.",
            )
        })?,
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(REFERER, HeaderValue::from_static(CHATGPT_WEB_REFERER));
    headers.insert(USER_AGENT, HeaderValue::from_static(CHATGPT_WEB_USER_AGENT));

    if let Some(account_id) = account
        .account_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.insert(
            "ChatGPT-Account-Id",
            HeaderValue::from_str(account_id).map_err(|err| {
                AppError::new(
                    "CODEX_QUOTA_INVALID_ACCOUNT_ID",
                    format!("Failed to build ChatGPT-Account-Id header: {}", err),
                    "Re-import this account.",
                )
            })?,
        );
    }

    Ok(headers)
}

async fn fetch_quota(account: &CodexAccount) -> AppResult<(CodexQuotaView, Option<String>)> {
    if account.auth_mode != CodexAuthMode::OAuth {
        return Err(AppError::new(
            "CODEX_QUOTA_UNSUPPORTED_AUTH_MODE",
            "API Key accounts do not support Codex quota checks in this version.",
            "Use an OAuth account for quota refresh.",
        ));
    }

    let client = reqwest::Client::new();
    let response = client
        .get(USAGE_URL)
        .headers(build_headers(account)?)
        .send()
        .await
        .map_err(|err| {
            quota_error(
                "CODEX_QUOTA_NETWORK_FAILED",
                format!("Quota API request failed: {}", err),
                "Check network connectivity and retry.",
                true,
            )
        })?;
    let status = response.status();
    let body = response.text().await.map_err(|err| {
        quota_error(
            "CODEX_QUOTA_READ_FAILED",
            format!("Failed to read quota API response: {}", err),
            "Retry quota refresh.",
            true,
        )
    })?;

    if !status.is_success() {
        return Err(classify_http_error(status, &body));
    }

    parse_quota(&body)
}

fn mark_existing_quota_stale(account: &mut CodexAccount) {
    if let Some(quota) = account.quota.as_mut() {
        quota.stale = true;
    }
}

fn record_quota_refresh_error(account: &mut CodexAccount, error: AppError) {
    mark_existing_quota_stale(account);
    account.quota_error = Some(error);
    account.updated_at = now_timestamp();
}

pub async fn refresh_quota(account_id: String) -> AppResult<CodexAccountView> {
    let mut file = storage::load_accounts_file()?;
    let account_index = file
        .accounts
        .iter()
        .position(|item| item.id == account_id)
        .ok_or_else(|| {
            AppError::new(
                "CODEX_ACCOUNT_NOT_FOUND",
                "Codex account was not found.",
                "Refresh accounts or import it again.",
            )
        })?;
    let mut account = file.accounts[account_index].clone();

    match fetch_quota(&account).await {
        Ok((quota, plan_type)) => {
            account.quota = Some(quota);
            account.quota_error = None;
            if plan_type.is_some() {
                account.plan_type = plan_type;
            }
            account.updated_at = now_timestamp();
            file.accounts[account_index] = account;
            let updated_account = file.accounts[account_index].to_view(
                file.current_account_id.as_deref()
                    == Some(file.accounts[account_index].id.as_str()),
            );
            storage::save_accounts_file(file)?;
            Ok(updated_account)
        }
        Err(error) => {
            record_quota_refresh_error(&mut account, error.clone());
            file.accounts[account_index] = account;
            storage::save_accounts_file(file)?;
            Err(error)
        }
    }
}

pub async fn refresh_all_quotas() -> AppResult<Vec<CodexAccountView>> {
    let account_ids = storage::load_accounts_file()?
        .accounts
        .into_iter()
        .filter(|account| account.auth_mode == CodexAuthMode::OAuth)
        .map(|account| account.id)
        .collect::<Vec<_>>();

    for account_id in account_ids {
        let _ = refresh_quota(account_id).await;
    }

    let file = storage::load_accounts_file()?;
    Ok(file
        .accounts
        .iter()
        .map(|account| {
            account.to_view(file.current_account_id.as_deref() == Some(account.id.as_str()))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{classify_http_error, parse_quota, record_quota_refresh_error};
    use crate::models::account::{CodexAccount, CodexAuthMode, CodexQuotaView};
    use crate::models::error::AppError;
    use reqwest::StatusCode;

    #[test]
    fn parses_quota_response_and_normalizes_remaining_percent() {
        let body = r#"{
          "plan_type": "chatgpt_plus",
          "rate_limit": {
            "primary_window": {
              "used_percent": 27,
              "reset_after_seconds": 120
            },
            "secondary_window": {
              "used_percent": 100,
              "reset_at": 1710000000
            }
          }
        }"#;

        let (quota, plan_type) = parse_quota(body).expect("Quota body should parse");

        assert_eq!(plan_type.as_deref(), Some("chatgpt_plus"));
        assert_eq!(quota.hourly_remaining_percent, Some(73));
        assert!(quota.hourly_reset_at.is_some());
        assert_eq!(quota.weekly_remaining_percent, Some(0));
        assert_eq!(quota.weekly_reset_at, Some(1710000000));
        assert_eq!(quota.stale, false);
    }

    #[test]
    fn rejects_invalid_quota_json() {
        let error = parse_quota("{").expect_err("Invalid quota JSON should fail");

        assert_eq!(error.code, "CODEX_QUOTA_INVALID_RESPONSE");
        assert!(!error.retryable);
    }

    #[test]
    fn classifies_unauthorized_http_error_as_non_retryable() {
        let error = classify_http_error(
            StatusCode::UNAUTHORIZED,
            r#"{"detail":{"code":"session_expired"}}"#,
        );

        assert_eq!(error.code, "CODEX_QUOTA_UNAUTHORIZED");
        assert!(!error.retryable);
        assert!(error.message.contains("session_expired"));
    }

    #[test]
    fn classifies_rate_limit_http_error_as_retryable() {
        let error = classify_http_error(StatusCode::TOO_MANY_REQUESTS, "try later");

        assert_eq!(error.code, "CODEX_QUOTA_RATE_LIMITED");
        assert!(error.retryable);
    }

    #[test]
    fn classifies_unexpected_http_error_as_non_retryable() {
        let error = classify_http_error(StatusCode::BAD_REQUEST, "bad request");

        assert_eq!(error.code, "CODEX_QUOTA_HTTP_FAILED");
        assert!(!error.retryable);
    }

    #[test]
    fn quota_error_preserves_existing_values_and_marks_stale() {
        let mut account = CodexAccount {
            id: "codex_fixture".to_string(),
            display_name: "Fixture".to_string(),
            email: Some("fixture@example.test".to_string()),
            auth_mode: CodexAuthMode::OAuth,
            bound_oauth_account_id: None,
            account_id: None,
            user_id: None,
            plan_type: None,
            token_bundle: None,
            api_key: None,
            api_base_url: None,
            quota: Some(CodexQuotaView {
                hourly_remaining_percent: Some(42),
                hourly_reset_at: Some(1_800_000_000),
                weekly_remaining_percent: Some(84),
                weekly_reset_at: Some(1_800_086_400),
                updated_at: Some(1_799_900_000),
                stale: false,
            }),
            quota_error: None,
            tags: Vec::new(),
            note: None,
            created_at: 1_799_800_000,
            updated_at: 1_799_900_000,
            last_used_at: None,
        };

        record_quota_refresh_error(
            &mut account,
            AppError::new(
                "CODEX_QUOTA_NETWORK_FAILED",
                "Quota API request failed.",
                "Check network connectivity and retry.",
            )
            .retryable(),
        );

        let quota = account.quota.expect("old quota should be preserved");
        assert_eq!(quota.hourly_remaining_percent, Some(42));
        assert_eq!(quota.weekly_remaining_percent, Some(84));
        assert!(quota.stale);
        assert_eq!(
            account
                .quota_error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some("CODEX_QUOTA_NETWORK_FAILED")
        );
    }
}
