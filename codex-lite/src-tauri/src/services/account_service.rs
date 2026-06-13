use crate::infra::{atomic_write, paths, storage};
use crate::models::account::{CodexAccount, CodexAccountView, CodexAuthMode};
use crate::models::error::{AppError, AppResult};
use crate::services::{auth_file_service, codex_config_service};

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

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn sync_active_api_account(account: &CodexAccount) -> AppResult<()> {
    let auth_file = auth_file_service::auth_file_from_account(account)?;
    let auth_content = serde_json::to_vec_pretty(&auth_file).map_err(|err| {
        AppError::new(
            "CODEX_AUTH_SERIALIZE_FAILED",
            format!("Failed to serialize updated API account auth file: {}", err),
            "Check the API account fields and try again.",
        )
    })?;
    let auth_path = paths::default_codex_auth_file()?;
    atomic_write::write_atomic(&auth_path, &auth_content)?;
    let config_path = paths::default_codex_config_file()?;
    codex_config_service::apply_account_config(account, &config_path)
}

pub fn update_api_key_account(
    account_id: &str,
    api_key: String,
    api_base_url: Option<String>,
    display_name: Option<String>,
) -> AppResult<CodexAccountView> {
    let trimmed_api_key = api_key.trim();
    if trimmed_api_key.is_empty() {
        return Err(AppError::new(
            "CODEX_API_KEY_EMPTY",
            "API key cannot be empty.",
            "Paste a valid API key.",
        ));
    }

    let mut file = storage::load_accounts_file()?;
    let is_current = file.current_account_id.as_deref() == Some(account_id);
    let account = file
        .accounts
        .iter_mut()
        .find(|item| item.id == account_id)
        .ok_or_else(|| {
            AppError::new(
                "CODEX_ACCOUNT_NOT_FOUND",
                "Codex account was not found.",
                "Refresh accounts or import it again.",
            )
        })?;

    if account.auth_mode != CodexAuthMode::ApiKey {
        return Err(AppError::new(
            "CODEX_ACCOUNT_NOT_API_KEY",
            "Only API Key accounts can be edited here.",
            "Choose an API Key account and try again.",
        ));
    }

    account.api_key = Some(trimmed_api_key.to_string());
    account.api_base_url = normalize_optional_text(api_base_url);
    if let Some(next_display_name) = normalize_optional_text(display_name) {
        account.display_name = next_display_name;
    }
    account.updated_at = now_timestamp();
    let updated = account.clone();
    storage::save_accounts_file(file)?;

    if is_current {
        sync_active_api_account(&updated)?;
    }

    Ok(updated.to_view(is_current))
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
