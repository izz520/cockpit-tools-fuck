use crate::models::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CodexAuthMode {
    #[serde(rename = "oauth", alias = "o_auth")]
    OAuth,
    #[serde(rename = "api_key")]
    ApiKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBundle {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexQuotaView {
    pub hourly_remaining_percent: Option<i32>,
    pub hourly_reset_at: Option<i64>,
    pub weekly_remaining_percent: Option<i32>,
    pub weekly_reset_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAccount {
    pub id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub auth_mode: CodexAuthMode,
    #[serde(default)]
    pub bound_oauth_account_id: Option<String>,
    pub account_id: Option<String>,
    pub user_id: Option<String>,
    pub plan_type: Option<String>,
    pub token_bundle: Option<TokenBundle>,
    pub api_key: Option<String>,
    pub api_base_url: Option<String>,
    pub quota: Option<CodexQuotaView>,
    pub quota_error: Option<AppError>,
    pub tags: Vec<String>,
    pub note: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAccountView {
    pub id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub auth_mode: CodexAuthMode,
    pub bound_oauth_account_id: Option<String>,
    pub account_id: Option<String>,
    pub user_id: Option<String>,
    pub plan_type: Option<String>,
    pub api_key: Option<String>,
    pub api_base_url: Option<String>,
    pub quota: Option<CodexQuotaView>,
    pub quota_error: Option<AppError>,
    pub tags: Vec<String>,
    pub note: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
    pub is_current: bool,
    pub capability_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsFile {
    pub schema_version: String,
    pub current_account_id: Option<String>,
    pub accounts: Vec<CodexAccount>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchResult {
    pub account: CodexAccountView,
    pub backup_path: Option<String>,
    pub restored: bool,
}

impl CodexAccount {
    pub fn to_view(&self, is_current: bool) -> CodexAccountView {
        CodexAccountView {
            id: self.id.clone(),
            display_name: self.display_name.clone(),
            email: self.email.clone(),
            auth_mode: self.auth_mode.clone(),
            bound_oauth_account_id: self.bound_oauth_account_id.clone(),
            account_id: self.account_id.clone(),
            user_id: self.user_id.clone(),
            plan_type: self.plan_type.clone(),
            api_key: self.api_key.clone(),
            api_base_url: self.api_base_url.clone(),
            quota: self.quota.clone(),
            quota_error: self.quota_error.clone(),
            tags: self.tags.clone(),
            note: self.note.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_used_at: self.last_used_at,
            is_current,
            capability_warning: match self.auth_mode {
                CodexAuthMode::OAuth => None,
                CodexAuthMode::ApiKey => {
                    Some("API Key accounts may not support Codex quota checks.".to_string())
                }
            },
        }
    }
}
