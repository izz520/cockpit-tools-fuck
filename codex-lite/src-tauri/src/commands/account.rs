use crate::models::account::{CodexAccountView, SwitchResult};
use crate::models::error::AppResult;
use crate::services::{account_service, switch_service};

#[tauri::command]
pub fn list_codex_accounts() -> AppResult<Vec<CodexAccountView>> {
    account_service::list_accounts()
}

#[tauri::command]
pub fn get_current_codex_account() -> AppResult<Option<CodexAccountView>> {
    account_service::get_current_account()
}

#[tauri::command]
pub fn delete_codex_account(account_id: String) -> AppResult<()> {
    account_service::delete_account(&account_id)
}

#[tauri::command]
pub fn switch_codex_account(account_id: String) -> AppResult<SwitchResult> {
    switch_service::switch_account(account_id)
}
