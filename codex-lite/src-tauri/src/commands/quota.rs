use crate::models::account::CodexAccountView;
use crate::models::error::AppResult;
use crate::services::quota_service;

#[tauri::command]
pub async fn refresh_codex_quota(account_id: String) -> AppResult<CodexAccountView> {
    quota_service::refresh_quota(account_id).await
}

#[tauri::command]
pub async fn refresh_all_codex_quotas() -> AppResult<Vec<CodexAccountView>> {
    quota_service::refresh_all_quotas().await
}
