use std::fs;
use std::path::Path;

use crate::infra::{atomic_write, codex_keychain, paths};
use crate::models::account::SwitchResult;
use crate::models::error::{AppError, AppResult};
use crate::services::{
    account_service, codex_app_service, codex_config_service, codex_session_visibility_service,
};

pub fn switch_account(account_id: String) -> AppResult<SwitchResult> {
    switch_account_with_writer_and_codex_control(
        account_id,
        atomic_write::write_atomic,
        codex_app_service::quit_codex_for_switch,
        codex_app_service::open_codex_after_switch,
    )
}

#[cfg(test)]
fn switch_account_with_writer(
    account_id: String,
    write_auth: impl Fn(&Path, &[u8]) -> AppResult<()>,
) -> AppResult<SwitchResult> {
    switch_account_with_writer_and_codex_control(account_id, write_auth, || Ok(()), || Ok(()))
}

fn switch_account_with_writer_and_codex_control(
    account_id: String,
    write_auth: impl Fn(&Path, &[u8]) -> AppResult<()>,
    quit_codex: impl Fn() -> AppResult<()>,
    open_codex: impl Fn() -> AppResult<()>,
) -> AppResult<SwitchResult> {
    let account = account_service::get_account(&account_id)?;
    let (auth_file, uses_oauth_auth) = account_service::auth_file_for_account_switch(&account)?;
    let auth_path = paths::default_codex_auth_file()?;
    let backup_path = if auth_path.exists() {
        fs::create_dir_all(paths::backups_dir()?).map_err(|err| {
            AppError::new(
                "CODEX_AUTH_BACKUP_FAILED",
                format!("Failed to create backup directory: {}", err),
                "Check app data directory permissions.",
            )
        })?;
        let backup_path = paths::backups_dir()?.join(format!(
            "auth-before-switch-{}.json",
            chrono::Utc::now().timestamp()
        ));
        fs::copy(&auth_path, &backup_path).map_err(|err| {
            AppError::new(
                "CODEX_AUTH_BACKUP_FAILED",
                format!("Failed to back up {}: {}", auth_path.display(), err),
                "Check file permissions before switching.",
            )
        })?;
        Some(backup_path)
    } else {
        None
    };

    let content = serde_json::to_vec_pretty(&auth_file).map_err(|err| {
        AppError::new(
            "CODEX_AUTH_SERIALIZE_FAILED",
            format!("Failed to serialize auth file: {}", err),
            "Re-import this account.",
        )
    })?;

    quit_codex()?;

    if let Err(write_error) = write_auth(&auth_path, &content) {
        if let Some(path) = backup_path.as_ref() {
            let _ = fs::copy(path, &auth_path);
        }
        return Err(write_error);
    }

    let config_path = paths::default_codex_config_file()?;
    if let Err(config_error) = codex_config_service::apply_account_config(&account, &config_path) {
        if let Some(path) = backup_path.as_ref() {
            let _ = fs::copy(path, &auth_path);
        }
        return Err(config_error);
    }

    let target_provider = codex_config_service::account_target_provider(&account);
    codex_session_visibility_service::repair_default_codex_home_for_provider(&target_provider)?;

    // On macOS, Codex reads OAuth credentials from the login keychain too. Update
    // it so the switch fully takes effect. Failure here is non-fatal: auth.json is
    // already written, so we log and continue rather than abort the switch.
    if uses_oauth_auth {
        if let Some(codex_home) = auth_path.parent() {
            if let Ok(auth_json) = std::str::from_utf8(&content) {
                if let Err(keychain_error) =
                    codex_keychain::write_codex_keychain(codex_home, auth_json)
                {
                    tracing::warn!(
                        code = keychain_error.code,
                        message = keychain_error.message,
                        "failed to update Codex login keychain during switch"
                    );
                }
            }
        }
    }

    let view = account_service::mark_current(&account_id)?;
    open_codex()?;
    Ok(SwitchResult {
        account: view,
        backup_path: backup_path.map(|path| path.display().to_string()),
        restored: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{switch_account, switch_account_with_writer};
    use crate::infra::{atomic_write, paths, storage};
    use crate::models::account::{AccountsFile, CodexAuthMode};
    use crate::models::error::AppError;
    use crate::services::auth_file_service;
    use crate::test_support::TestEnv;

    fn oauth_account() -> crate::models::account::CodexAccount {
        let auth = auth_file_service::parse_auth_json(&TestEnv::fixture_content("oauth.json"))
            .expect("OAuth fixture should parse");
        auth_file_service::account_from_auth(auth).expect("OAuth fixture should become account")
    }

    fn save_single_account(account: crate::models::account::CodexAccount) {
        storage::save_accounts_file(AccountsFile {
            schema_version: "1.0.0".to_string(),
            current_account_id: None,
            accounts: vec![account],
            updated_at: 0,
        })
        .expect("accounts file should save");
    }

    #[test]
    fn switch_account_writes_target_auth_and_marks_current() {
        let _env = TestEnv::new("switch-success");
        let account = oauth_account();
        let account_id = account.id.clone();
        save_single_account(account);
        let auth_path = paths::default_codex_auth_file().expect("auth path should resolve");
        std::fs::write(&auth_path, TestEnv::fixture_content("api-key.json"))
            .expect("existing auth should be written");

        let result = switch_account_with_writer(account_id.clone(), atomic_write::write_atomic)
            .expect("switch should succeed");
        let written_auth =
            auth_file_service::read_auth_file(&auth_path).expect("written auth should parse");
        let stored = storage::load_accounts_file().expect("accounts should load");

        assert_eq!(result.account.id, account_id);
        assert!(result.account.is_current);
        assert!(result.backup_path.is_some());
        assert_eq!(
            stored.current_account_id.as_deref(),
            Some(account_id.as_str())
        );
        // OAuth auth files omit auth_mode (Codex treats absence as ChatGPT mode).
        assert!(written_auth.auth_mode.is_none());
        assert!(written_auth.tokens.is_some());
    }

    #[test]
    fn switch_account_missing_credentials_does_not_write_auth_file() {
        let _env = TestEnv::new("switch-missing-credentials");
        let mut account = oauth_account();
        account.token_bundle = None;
        let account_id = account.id.clone();
        save_single_account(account);
        let auth_path = paths::default_codex_auth_file().expect("auth path should resolve");

        let error = switch_account(account_id).expect_err("missing credentials should fail");

        assert_eq!(error.code, "CODEX_ACCOUNT_MISSING_CREDENTIALS");
        assert!(!auth_path.exists());
    }

    #[test]
    fn switch_account_restores_original_auth_when_write_fails() {
        let _env = TestEnv::new("switch-restore");
        let account = oauth_account();
        let account_id = account.id.clone();
        save_single_account(account);
        let auth_path = paths::default_codex_auth_file().expect("auth path should resolve");
        let original_auth = TestEnv::fixture_content("api-key.json");
        std::fs::write(&auth_path, &original_auth).expect("existing auth should be written");

        let error = switch_account_with_writer(account_id, |_path, _content| {
            Err(AppError::new(
                "TEST_WRITE_FAILED",
                "Injected write failure.",
                "This failure is expected in test.",
            ))
        })
        .expect_err("injected write failure should fail switch");
        let restored_auth =
            std::fs::read_to_string(&auth_path).expect("restored auth should be readable");
        let stored = storage::load_accounts_file().expect("accounts should load");

        assert_eq!(error.code, "TEST_WRITE_FAILED");
        assert_eq!(restored_auth, original_auth);
        assert!(stored.current_account_id.is_none());
    }

    #[test]
    fn api_key_account_exports_switchable_auth_file() {
        let _env = TestEnv::new("switch-api-key");
        let auth = auth_file_service::parse_auth_json(&TestEnv::fixture_content("api-key.json"))
            .expect("API key fixture should parse");
        let account =
            auth_file_service::account_from_auth(auth).expect("API key fixture should import");
        let account_id = account.id.clone();
        save_single_account(account);

        switch_account_with_writer(account_id, atomic_write::write_atomic)
            .expect("API key switch should succeed");
        let written_auth =
            auth_file_service::read_auth_file(&paths::default_codex_auth_file().unwrap())
                .expect("written API key auth should parse");

        assert_eq!(written_auth.auth_mode.as_deref(), Some("apikey"));
        assert!(written_auth.openai_api_key.is_some());
        assert_eq!(
            auth_file_service::account_from_auth(written_auth)
                .expect("written API key auth should project")
                .auth_mode,
            CodexAuthMode::ApiKey
        );
    }

    #[test]
    fn api_key_account_with_bound_oauth_writes_oauth_auth_and_keeps_api_current() {
        let _env = TestEnv::new("switch-bound-oauth");
        let oauth = oauth_account();
        let oauth_id = oauth.id.clone();
        let api_auth =
            auth_file_service::parse_auth_json(&TestEnv::fixture_content("api-key.json"))
                .expect("API key fixture should parse");
        let mut api_account =
            auth_file_service::account_from_auth(api_auth).expect("API key fixture should import");
        api_account.bound_oauth_account_id = Some(oauth_id);
        let api_account_id = api_account.id.clone();
        storage::save_accounts_file(AccountsFile {
            schema_version: "1.0.0".to_string(),
            current_account_id: None,
            accounts: vec![api_account, oauth],
            updated_at: 0,
        })
        .expect("accounts file should save");

        switch_account_with_writer(api_account_id.clone(), atomic_write::write_atomic)
            .expect("bound API key switch should succeed");
        let written_auth =
            auth_file_service::read_auth_file(&paths::default_codex_auth_file().unwrap())
                .expect("written auth should parse");
        let stored = storage::load_accounts_file().expect("accounts should load");

        assert!(written_auth.auth_mode.is_none());
        assert!(written_auth.tokens.is_some());
        assert_eq!(
            stored.current_account_id.as_deref(),
            Some(api_account_id.as_str())
        );
    }
}
