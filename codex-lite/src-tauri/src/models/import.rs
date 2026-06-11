use crate::models::account::{CodexAccount, CodexAccountView, CodexAuthMode, CodexQuotaView};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFailure {
    pub source: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: Vec<CodexAccountView>,
    pub skipped: Vec<CodexAccountView>,
    pub failed: Vec<ImportFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchImportItemStatus {
    Importable,
    Existing,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportPreviewItem {
    pub id: String,
    pub source: String,
    pub status: BatchImportItemStatus,
    pub selected: bool,
    pub selectable: bool,
    pub reason: Option<String>,
    pub account_id: Option<String>,
    pub user_id: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub auth_mode: Option<CodexAuthMode>,
    pub plan_type: Option<String>,
    pub api_base_url: Option<String>,
    pub quota: Option<CodexQuotaView>,
    pub quota_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportSession {
    pub session_id: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub check_quota: bool,
    pub items: Vec<BatchImportPreviewItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportSessionItem {
    pub preview: BatchImportPreviewItem,
    pub account: Option<CodexAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredBatchImportSession {
    pub session: BatchImportSession,
    pub items: Vec<BatchImportSessionItem>,
}
