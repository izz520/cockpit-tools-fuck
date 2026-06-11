use crate::models::account::CodexAccountView;
use crate::models::error::AppResult;
use crate::models::import::{BatchImportSession, ImportResult};
use crate::services::import_service;

#[tauri::command]
pub fn import_codex_from_local() -> AppResult<CodexAccountView> {
    import_service::import_from_local()
}

#[tauri::command]
pub fn import_codex_from_json(json_content: String) -> AppResult<ImportResult> {
    import_service::import_from_json(&json_content)
}

#[tauri::command]
pub fn import_codex_from_files(file_paths: Vec<String>) -> AppResult<ImportResult> {
    import_service::import_from_files(file_paths)
}

#[tauri::command]
pub fn start_codex_batch_import_from_files(
    file_paths: Vec<String>,
    check_quota: bool,
) -> AppResult<BatchImportSession> {
    import_service::start_batch_import_from_files(file_paths, check_quota)
}

#[tauri::command]
pub fn confirm_codex_batch_import(
    session_id: String,
    item_ids: Vec<String>,
) -> AppResult<ImportResult> {
    import_service::confirm_batch_import(session_id, item_ids)
}

#[tauri::command]
pub fn add_codex_account_with_token(
    id_token: String,
    access_token: String,
    refresh_token: Option<String>,
) -> AppResult<CodexAccountView> {
    import_service::add_with_token(id_token, access_token, refresh_token)
}

#[tauri::command]
pub fn add_codex_account_with_api_key(
    api_key: String,
    api_base_url: Option<String>,
    display_name: Option<String>,
) -> AppResult<CodexAccountView> {
    import_service::add_with_api_key(api_key, api_base_url, display_name)
}
