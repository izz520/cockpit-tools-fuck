use crate::infra::{paths, storage};
use crate::models::account::{CodexAccount, CodexAccountView};
use crate::models::error::{AppError, AppResult};
use crate::services::auth_file_service;

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn list_accounts() -> AppResult<Vec<CodexAccountView>> {
    let file = storage::load_accounts_file()?;
    Ok(file
        .accounts
        .iter()
        .map(|account| {
            account.to_view(file.current_account_id.as_deref() == Some(account.id.as_str()))
        })
        .collect())
}

pub fn upsert_account(mut account: CodexAccount) -> AppResult<CodexAccountView> {
    let mut file = storage::load_accounts_file()?;
    let existing = file.accounts.iter().position(|item| item.id == account.id);
    let is_current = file.current_account_id.as_deref() == Some(account.id.as_str());
    account.updated_at = now_timestamp();
    match existing {
        Some(index) => {
            account.created_at = file.accounts[index].created_at;
            file.accounts[index] = account.clone();
        }
        None => file.accounts.insert(0, account.clone()),
    }
    storage::save_accounts_file(file)?;
    Ok(account.to_view(is_current))
}

pub fn get_account(account_id: &str) -> AppResult<CodexAccount> {
    let file = storage::load_accounts_file()?;
    file.accounts
        .into_iter()
        .find(|account| account.id == account_id)
        .ok_or_else(|| {
            AppError::new(
                "CODEX_ACCOUNT_NOT_FOUND",
                "Codex account was not found.",
                "Refresh accounts or import it again.",
            )
        })
}

pub fn delete_account(account_id: &str) -> AppResult<()> {
    let mut file = storage::load_accounts_file()?;
    file.accounts.retain(|account| account.id != account_id);
    if file.current_account_id.as_deref() == Some(account_id) {
        file.current_account_id = None;
    }
    storage::save_accounts_file(file)
}

pub fn get_current_account() -> AppResult<Option<CodexAccountView>> {
    let auth_path = paths::default_codex_auth_file()?;
    if !auth_path.exists() {
        return Ok(None);
    }
    let auth = auth_file_service::read_auth_file(&auth_path)?;
    let parsed = auth_file_service::account_from_auth(auth)?;
    let file = storage::load_accounts_file()?;
    let matched = file.accounts.iter().find(|account| account.id == parsed.id);
    Ok(matched
        .map(|account| account.to_view(true))
        .or_else(|| Some(parsed.to_view(true))))
}

pub fn mark_current(account_id: &str) -> AppResult<CodexAccountView> {
    let mut file = storage::load_accounts_file()?;
    let mut selected = None;
    for account in &mut file.accounts {
        if account.id == account_id {
            account.last_used_at = Some(now_timestamp());
            account.updated_at = now_timestamp();
            selected = Some(account.clone());
        }
    }
    let account = selected.ok_or_else(|| {
        AppError::new(
            "CODEX_ACCOUNT_NOT_FOUND",
            "Codex account was not found.",
            "Refresh accounts or import it again.",
        )
    })?;
    file.current_account_id = Some(account_id.to_string());
    storage::save_accounts_file(file)?;
    Ok(account.to_view(true))
}
