use crate::models::account::CodexAccountView;
use crate::models::error::AppResult;
use crate::services::oauth_service::{self, OAuthStartResult, OAuthStatusResult};

#[tauri::command]
pub fn codex_oauth_login_start() -> AppResult<OAuthStartResult> {
    oauth_service::start_login()
}

#[tauri::command]
pub fn codex_oauth_submit_callback_url(login_id: String, callback_url: String) -> AppResult<()> {
    oauth_service::submit_callback_url(login_id, callback_url)
}

#[tauri::command]
pub fn codex_oauth_login_status(login_id: String) -> AppResult<OAuthStatusResult> {
    oauth_service::login_status(login_id)
}

#[tauri::command]
pub async fn codex_oauth_login_completed(login_id: String) -> AppResult<CodexAccountView> {
    oauth_service::complete_login(login_id).await
}

#[tauri::command]
pub fn codex_oauth_login_cancel(login_id: Option<String>) -> AppResult<()> {
    oauth_service::cancel_login(login_id)
}

#[tauri::command]
pub fn is_codex_oauth_port_in_use() -> AppResult<bool> {
    oauth_service::is_callback_port_in_use()
}
