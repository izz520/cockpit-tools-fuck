use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, REFERER, USER_AGENT};
use reqwest::Url;
use serde::Deserialize;

use crate::infra::storage;
use crate::models::account::{CodexAccount, CodexAccountView, CodexAuthMode, CodexQuotaView};
use crate::models::error::{AppError, AppResult};
use crate::services::auth_file_service;

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const SUBSCRIPTION_ACCOUNTS_CHECK_URL: &str =
    "https://chatgpt.com/backend-api/accounts/check/v4-2023-04-27";
const SUBSCRIPTIONS_URL: &str = "https://chatgpt.com/backend-api/subscriptions";
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

#[derive(Debug, Clone, Default)]
struct SubscriptionSnapshot {
    account_id: Option<String>,
    plan_type: Option<String>,
    subscription_active_until: Option<String>,
}

struct AccountCheckRecord<'a> {
    key: Option<&'a str>,
    node: &'a serde_json::Value,
}

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn current_chatgpt_timezone_offset_min() -> i32 {
    -(chrono::Local::now().offset().local_minus_utc() / 60)
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

fn value_to_trimmed_string(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(text) => trim_optional(Some(text.clone())),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn string_value_at<'a>(
    value: &'a serde_json::Value,
    keys: &[&str],
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    Some(current)
}

fn read_text_path(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    value_to_trimmed_string(string_value_at(value, keys))
}

fn extract_account_id_from_access_token(account: &CodexAccount) -> Option<String> {
    let access_token = account.token_bundle.as_ref()?.access_token.as_str();
    let payload = auth_file_service::decode_jwt_payload(access_token).ok()?;
    let auth_data = payload.auth_data.as_ref()?;
    trim_optional(auth_data.chatgpt_account_id.clone())
        .or_else(|| trim_optional(auth_data.account_id.clone()))
}

fn parse_subscription_timestamp(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(raw) = trimmed.parse::<i64>() {
        return Some(if raw > 10_000_000_000 {
            raw / 1000
        } else {
            raw
        });
    }

    chrono::DateTime::parse_from_rfc3339(trimmed)
        .map(|date| date.timestamp())
        .ok()
}

fn subscription_missing_or_expired(value: Option<&str>) -> bool {
    let Some(timestamp) = value.and_then(parse_subscription_timestamp) else {
        return true;
    };
    timestamp <= now_timestamp()
}

fn account_record_matches(record: &serde_json::Value, account_id: Option<&str>) -> bool {
    let Some(expected) = account_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };

    [
        read_text_path(record, &["account_id"]),
        read_text_path(record, &["accountId"]),
        read_text_path(record, &["id"]),
        read_text_path(record, &["account", "id"]),
        read_text_path(record, &["account", "account_id"]),
        read_text_path(record, &["account", "chatgpt_account_id"]),
        read_text_path(record, &["account", "workspace_id"]),
    ]
    .into_iter()
    .flatten()
    .any(|candidate| candidate == expected)
}

fn collect_account_check_records(payload: &serde_json::Value) -> Vec<AccountCheckRecord<'_>> {
    let mut records = Vec::new();

    if let Some(accounts) = payload.get("accounts") {
        if let Some(items) = accounts.as_array() {
            for item in items {
                if item.is_object() {
                    records.push(AccountCheckRecord {
                        key: None,
                        node: item,
                    });
                }
            }
        } else if let Some(items) = accounts.as_object() {
            for (key, item) in items {
                if item.is_object() {
                    records.push(AccountCheckRecord {
                        key: Some(key.as_str()),
                        node: item,
                    });
                }
            }
        }
    }

    if records.is_empty() {
        if let Some(items) = payload.as_array() {
            for item in items {
                if item.is_object() {
                    records.push(AccountCheckRecord {
                        key: None,
                        node: item,
                    });
                }
            }
        }
    }

    records
}

fn parse_account_check_record(record: &serde_json::Value) -> SubscriptionSnapshot {
    SubscriptionSnapshot {
        account_id: read_text_path(record, &["account_id"])
            .or_else(|| read_text_path(record, &["accountId"]))
            .or_else(|| read_text_path(record, &["id"]))
            .or_else(|| read_text_path(record, &["account", "id"]))
            .or_else(|| read_text_path(record, &["account", "account_id"]))
            .or_else(|| read_text_path(record, &["account", "chatgpt_account_id"]))
            .or_else(|| read_text_path(record, &["account", "workspace_id"])),
        plan_type: read_text_path(record, &["plan_type"])
            .or_else(|| read_text_path(record, &["planType"]))
            .or_else(|| read_text_path(record, &["account", "plan_type"]))
            .or_else(|| read_text_path(record, &["account", "planType"]))
            .or_else(|| read_text_path(record, &["entitlement", "subscription_plan"]))
            .or_else(|| read_text_path(record, &["entitlement", "plan_type"])),
        subscription_active_until: read_text_path(record, &["entitlement", "expires_at"])
            .or_else(|| read_text_path(record, &["expires_at"]))
            .or_else(|| read_text_path(record, &["account", "expires_at"]))
            .or_else(|| read_text_path(record, &["active_until"])),
    }
}

fn parse_account_check_snapshot(
    body: &str,
    account: &CodexAccount,
) -> AppResult<SubscriptionSnapshot> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|err| {
        AppError::new(
            "CODEX_SUBSCRIPTION_INVALID_RESPONSE",
            format!(
                "Subscription account-check response is not valid JSON: {}",
                err
            ),
            "Retry later. If it keeps failing, the upstream response shape may have changed.",
        )
    })?;

    let candidates = collect_account_check_records(&value);
    let preferred_account_id = trim_optional(account.account_id.clone())
        .or_else(|| extract_account_id_from_access_token(account));
    let ordering_first_key = value
        .get("account_ordering")
        .and_then(|item| item.as_array())
        .and_then(|items| items.first())
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty());

    candidates
        .iter()
        .find(|record| account_record_matches(record.node, preferred_account_id.as_deref()))
        .or_else(|| {
            candidates.iter().find(|record| {
                record.key.map(str::trim).filter(|item| !item.is_empty()) == ordering_first_key
            })
        })
        .or_else(|| candidates.first())
        .map(|record| parse_account_check_record(record.node))
        .ok_or_else(|| {
            AppError::new(
                "CODEX_SUBSCRIPTION_ACCOUNT_NOT_FOUND",
                "Subscription account-check response did not include this account.",
                "Refresh again or re-authenticate this account.",
            )
        })
}

fn parse_subscription_snapshot(body: &str) -> AppResult<SubscriptionSnapshot> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|err| {
        AppError::new(
            "CODEX_SUBSCRIPTION_INVALID_RESPONSE",
            format!("Subscription response is not valid JSON: {}", err),
            "Retry later. If it keeps failing, the upstream response shape may have changed.",
        )
    })?;

    Ok(SubscriptionSnapshot {
        account_id: read_text_path(&value, &["account_id"])
            .or_else(|| read_text_path(&value, &["accountId"])),
        plan_type: read_text_path(&value, &["subscription_plan"])
            .or_else(|| read_text_path(&value, &["plan_type"]))
            .or_else(|| read_text_path(&value, &["planType"])),
        subscription_active_until: read_text_path(&value, &["active_until"])
            .or_else(|| read_text_path(&value, &["expires_at"])),
    })
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

fn add_target_headers(headers: &mut HeaderMap, target_path: &str) -> AppResult<()> {
    let value = HeaderValue::from_str(target_path).map_err(|err| {
        AppError::new(
            "CODEX_SUBSCRIPTION_INVALID_TARGET_PATH",
            format!("Failed to build subscription target headers: {}", err),
            "Retry subscription refresh later.",
        )
    })?;
    headers.insert("x-openai-target-path", value.clone());
    headers.insert("x-openai-target-route", value);
    Ok(())
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

async fn fetch_subscription_account_check(
    account: &CodexAccount,
) -> AppResult<SubscriptionSnapshot> {
    let client = reqwest::Client::new();
    let mut url = Url::parse(SUBSCRIPTION_ACCOUNTS_CHECK_URL).map_err(|err| {
        AppError::new(
            "CODEX_SUBSCRIPTION_URL_INVALID",
            format!("Failed to build subscription account-check URL: {}", err),
            "Retry subscription refresh later.",
        )
    })?;
    let timezone_offset_min = current_chatgpt_timezone_offset_min().to_string();
    url.query_pairs_mut()
        .append_pair("timezone_offset_min", &timezone_offset_min);
    let mut headers = build_headers(account)?;
    add_target_headers(&mut headers, "/backend-api/accounts/check/v4-2023-04-27")?;

    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|err| {
            quota_error(
                "CODEX_SUBSCRIPTION_NETWORK_FAILED",
                format!("Subscription account-check request failed: {}", err),
                "Check network connectivity and retry.",
                true,
            )
        })?;
    let status = response.status();
    let body = response.text().await.map_err(|err| {
        quota_error(
            "CODEX_SUBSCRIPTION_READ_FAILED",
            format!(
                "Failed to read subscription account-check response: {}",
                err
            ),
            "Retry subscription refresh.",
            true,
        )
    })?;

    if !status.is_success() {
        return Err(classify_http_error(status, &body));
    }

    parse_account_check_snapshot(&body, account)
}

async fn fetch_subscriptions_snapshot(
    account: &CodexAccount,
    account_id: &str,
) -> AppResult<SubscriptionSnapshot> {
    let client = reqwest::Client::new();
    let mut url = Url::parse(SUBSCRIPTIONS_URL).map_err(|err| {
        AppError::new(
            "CODEX_SUBSCRIPTION_URL_INVALID",
            format!("Failed to build subscription URL: {}", err),
            "Retry subscription refresh later.",
        )
    })?;
    url.query_pairs_mut().append_pair("account_id", account_id);
    let mut headers = build_headers(account)?;
    add_target_headers(&mut headers, "/backend-api/subscriptions")?;

    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|err| {
            quota_error(
                "CODEX_SUBSCRIPTION_NETWORK_FAILED",
                format!("Subscription request failed: {}", err),
                "Check network connectivity and retry.",
                true,
            )
        })?;
    let status = response.status();
    let body = response.text().await.map_err(|err| {
        quota_error(
            "CODEX_SUBSCRIPTION_READ_FAILED",
            format!("Failed to read subscription response: {}", err),
            "Retry subscription refresh.",
            true,
        )
    })?;

    if !status.is_success() {
        return Err(classify_http_error(status, &body));
    }

    parse_subscription_snapshot(&body)
}

async fn fetch_subscription_status(account: &CodexAccount) -> AppResult<SubscriptionSnapshot> {
    let mut snapshot = fetch_subscription_account_check(account).await?;
    if subscription_missing_or_expired(snapshot.subscription_active_until.as_deref()) {
        let account_id = snapshot
            .account_id
            .clone()
            .or_else(|| trim_optional(account.account_id.clone()))
            .or_else(|| extract_account_id_from_access_token(account));
        if let Some(account_id) = account_id {
            if let Ok(fallback) = fetch_subscriptions_snapshot(account, &account_id).await {
                snapshot.account_id = Some(account_id);
                if fallback.account_id.is_some() {
                    snapshot.account_id = fallback.account_id;
                }
                if fallback.plan_type.is_some() {
                    snapshot.plan_type = fallback.plan_type;
                }
                if fallback.subscription_active_until.is_some() {
                    snapshot.subscription_active_until = fallback.subscription_active_until;
                }
            }
        }
    }
    Ok(snapshot)
}

async fn sync_subscription_status(account: &mut CodexAccount) {
    match fetch_subscription_status(account).await {
        Ok(snapshot) => {
            if snapshot.account_id.is_some() {
                account.account_id = snapshot.account_id;
            }
            if snapshot.plan_type.is_some() {
                account.plan_type = snapshot.plan_type;
            }
            if snapshot.subscription_active_until.is_some() {
                account.subscription_active_until = snapshot.subscription_active_until;
            }
        }
        Err(error) => {
            tracing::warn!(
                code = error.code,
                message = error.message,
                "failed to refresh Codex subscription status"
            );
        }
    }
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
            sync_subscription_status(&mut account).await;
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
            subscription_active_until: None,
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
