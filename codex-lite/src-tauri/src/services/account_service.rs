use crate::infra::{atomic_write, codex_keychain, paths, storage};
use crate::models::account::{CodexAccount, CodexAccountView, CodexAuthMode};
use crate::models::auth::CodexAuthFile;
use crate::models::error::{AppError, AppResult};
use crate::services::{auth_file_service, codex_config_service, codex_local_access_gateway};

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

fn account_has_refresh_token(account: &CodexAccount) -> bool {
    account
        .token_bundle
        .as_ref()
        .and_then(|bundle| bundle.refresh_token.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

fn validate_bound_oauth_account(
    accounts: &[CodexAccount],
    api_key_account: &CodexAccount,
    bound_oauth_account_id: &str,
) -> AppResult<CodexAccount> {
    if api_key_account.auth_mode != CodexAuthMode::ApiKey {
        return Err(AppError::new(
            "CODEX_ACCOUNT_NOT_API_KEY",
            "Only API Key accounts can bind an OAuth account.",
            "Choose an API Key account and try again.",
        ));
    }
    if api_key_account.id == bound_oauth_account_id {
        return Err(AppError::new(
            "CODEX_BINDING_SELF_REFERENCE",
            "An API Key account cannot bind itself as OAuth.",
            "Choose a different OAuth account.",
        ));
    }
    let oauth_account = accounts
        .iter()
        .find(|account| account.id == bound_oauth_account_id)
        .cloned()
        .ok_or_else(|| {
            AppError::new(
                "CODEX_BOUND_OAUTH_NOT_FOUND",
                "Bound OAuth account was not found.",
                "Refresh accounts or import the OAuth account again.",
            )
        })?;
    if !matches!(oauth_account.auth_mode, CodexAuthMode::OAuth) {
        return Err(AppError::new(
            "CODEX_BOUND_ACCOUNT_NOT_OAUTH",
            "Bound account must be an OAuth account.",
            "Choose an OAuth account with a refresh token.",
        ));
    }
    if !account_has_refresh_token(&oauth_account) {
        return Err(AppError::new(
            "CODEX_BOUND_OAUTH_MISSING_REFRESH_TOKEN",
            "Bound OAuth account has no refresh token.",
            "Re-import this OAuth account or sign in again.",
        ));
    }
    Ok(oauth_account)
}

pub fn resolve_bound_oauth_account(account: &CodexAccount) -> AppResult<Option<CodexAccount>> {
    if account.auth_mode != CodexAuthMode::ApiKey {
        return Ok(None);
    }
    let Some(bound_id) = account
        .bound_oauth_account_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let file = storage::load_accounts_file()?;
    validate_bound_oauth_account(&file.accounts, account, bound_id).map(Some)
}

pub fn auth_file_for_account_switch(account: &CodexAccount) -> AppResult<(CodexAuthFile, bool)> {
    if let Some(oauth_account) = resolve_bound_oauth_account(account)? {
        return Ok((
            auth_file_service::auth_file_from_account(&oauth_account)?,
            true,
        ));
    }
    Ok((
        auth_file_service::auth_file_from_account(account)?,
        matches!(account.auth_mode, CodexAuthMode::OAuth),
    ))
}

async fn sync_active_api_account(account: &CodexAccount) -> AppResult<()> {
    let (auth_file, uses_oauth_auth) = auth_file_for_account_switch(account)?;
    let auth_content = serde_json::to_vec_pretty(&auth_file).map_err(|err| {
        AppError::new(
            "CODEX_AUTH_SERIALIZE_FAILED",
            format!("Failed to serialize updated API account auth file: {}", err),
            "Check the API account fields and try again.",
        )
    })?;
    let auth_path = paths::default_codex_auth_file()?;
    atomic_write::write_atomic(&auth_path, &auth_content)?;
    if uses_oauth_auth {
        if let Some(codex_home) = auth_path.parent() {
            if let Ok(auth_json) = std::str::from_utf8(&auth_content) {
                if let Err(keychain_error) =
                    codex_keychain::write_codex_keychain(codex_home, auth_json)
                {
                    tracing::warn!(
                        code = keychain_error.code,
                        message = keychain_error.message,
                        "failed to update Codex login keychain after OAuth binding change"
                    );
                }
            }
        }
    }
    let config_path = paths::default_codex_config_file()?;
    let provider_base_url = codex_local_access_gateway::ensure_for_account(account).await?;
    codex_config_service::apply_account_config_with_provider_base_url(
        account,
        &config_path,
        provider_base_url.as_deref(),
    )
}

pub async fn update_api_key_account(
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
        sync_active_api_account(&updated).await?;
    }

    Ok(updated.to_view(is_current))
}

pub async fn update_api_key_bound_oauth_account(
    account_id: &str,
    bound_oauth_account_id: Option<String>,
) -> AppResult<CodexAccountView> {
    let next_bound_id = normalize_optional_text(bound_oauth_account_id);
    let mut file = storage::load_accounts_file()?;
    let index = file
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

    if file.accounts[index].auth_mode != CodexAuthMode::ApiKey {
        return Err(AppError::new(
            "CODEX_ACCOUNT_NOT_API_KEY",
            "Only API Key accounts can bind an OAuth account.",
            "Choose an API Key account and try again.",
        ));
    }

    if let Some(bound_id) = next_bound_id.as_deref() {
        validate_bound_oauth_account(&file.accounts, &file.accounts[index], bound_id)?;
    }

    let is_current = file.current_account_id.as_deref() == Some(account_id);
    file.accounts[index].bound_oauth_account_id = next_bound_id;
    file.accounts[index].updated_at = now_timestamp();
    let updated = file.accounts[index].clone();
    storage::save_accounts_file(file)?;

    if is_current {
        sync_active_api_account(&updated).await?;
    }

    Ok(updated.to_view(is_current))
}

pub fn get_current_account() -> AppResult<Option<CodexAccountView>> {
    let file = storage::load_accounts_file()?;
    if let Some(current_account) = file
        .current_account_id
        .as_deref()
        .and_then(|id| file.accounts.iter().find(|account| account.id == id))
    {
        return Ok(Some(current_account.to_view(true)));
    }

    let auth_path = paths::default_codex_auth_file()?;
    if !auth_path.exists() {
        return Ok(None);
    }
    let auth = auth_file_service::read_auth_file(&auth_path)?;
    let parsed = auth_file_service::account_from_auth(auth)?;
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
