use std::fs;
use std::path::Path;

use crate::infra::{atomic_write, codex_keychain, paths};
use crate::models::account::{CodexAccount, CodexAuthMode, SwitchResult};
use crate::models::auth::CodexAuthFile;
use crate::models::error::{AppError, AppResult};
use crate::models::session::SessionRepairSummary;
use crate::services::{
    account_service, auth_file_service, codex_app_service, codex_config_service,
    codex_local_access_gateway, codex_session_visibility_service, oauth_service,
};

fn launch_credential_kind_for_provider(provider: &str) -> &'static str {
    if provider == codex_config_service::DEFAULT_PROVIDER_ID {
        "account"
    } else {
        "api"
    }
}

fn should_repair_session_visibility(before_provider: Option<&str>, after_provider: &str) -> bool {
    let Some(before_provider) = before_provider else {
        return false;
    };
    if before_provider == after_provider {
        return false;
    }
    launch_credential_kind_for_provider(before_provider)
        != launch_credential_kind_for_provider(after_provider)
}

fn previous_launch_provider(config_path: &Path) -> Option<String> {
    codex_config_service::read_active_provider(config_path).ok()
}

struct FileSnapshot {
    path: std::path::PathBuf,
    content: Option<Vec<u8>>,
}

impl FileSnapshot {
    fn capture(path: &Path) -> AppResult<Self> {
        let content = match fs::read(path) {
            Ok(content) => Some(content),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => {
                return Err(AppError::new(
                    "CODEX_SWITCH_SNAPSHOT_FAILED",
                    format!("Failed to snapshot {}: {}", path.display(), err),
                    "Check file permissions before switching.",
                ));
            }
        };
        Ok(Self {
            path: path.to_path_buf(),
            content,
        })
    }

    fn restore(&self) {
        match self.content.as_deref() {
            Some(content) => {
                let _ = atomic_write::write_atomic(&self.path, content);
            }
            None => match fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {}
            },
        }
    }
}

struct PreparedSwitchAuth {
    auth_file: CodexAuthFile,
    uses_oauth_auth: bool,
    warnings: Vec<String>,
}

async fn prepare_switch_auth(account: &CodexAccount) -> AppResult<PreparedSwitchAuth> {
    if account.auth_mode == CodexAuthMode::OAuth {
        let refreshed = oauth_service::refresh_account_if_needed(account, false).await?;
        return Ok(PreparedSwitchAuth {
            auth_file: auth_file_service::auth_file_from_account(&refreshed)?,
            uses_oauth_auth: true,
            warnings: Vec::new(),
        });
    }

    let Some(bound_oauth_account_id) = account
        .bound_oauth_account_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(PreparedSwitchAuth {
            auth_file: auth_file_service::auth_file_from_account(account)?,
            uses_oauth_auth: false,
            warnings: Vec::new(),
        });
    };

    match account_service::get_account(bound_oauth_account_id) {
        Ok(bound_account) if bound_account.auth_mode == CodexAuthMode::OAuth => {
            match oauth_service::refresh_account_if_needed(&bound_account, false).await {
                Ok(refreshed) => Ok(PreparedSwitchAuth {
                    auth_file: auth_file_service::auth_file_from_account(&refreshed)?,
                    uses_oauth_auth: true,
                    warnings: Vec::new(),
                }),
                Err(error) => Ok(PreparedSwitchAuth {
                    auth_file: auth_file_service::auth_file_from_account(account)?,
                    uses_oauth_auth: false,
                    warnings: vec![format!(
                        "Bound OAuth account refresh failed ({}): {}. Switched with API Key credentials.",
                        error.code, error.message
                    )],
                }),
            }
        }
        Ok(_) => Ok(PreparedSwitchAuth {
            auth_file: auth_file_service::auth_file_from_account(account)?,
            uses_oauth_auth: false,
            warnings: vec![
                "Bound OAuth account is not an OAuth account. Switched with API Key credentials."
                    .to_string(),
            ],
        }),
        Err(error) => Ok(PreparedSwitchAuth {
            auth_file: auth_file_service::auth_file_from_account(account)?,
            uses_oauth_auth: false,
            warnings: vec![format!(
                "Bound OAuth account is unavailable ({}): {}. Switched with API Key credentials.",
                error.code, error.message
            )],
        }),
    }
}

pub async fn switch_account(account_id: String) -> AppResult<SwitchResult> {
    switch_account_with_writer_and_codex_control(
        account_id,
        atomic_write::write_atomic,
        codex_app_service::quit_codex_for_switch,
        codex_app_service::open_codex_after_switch,
    )
    .await
}

#[cfg(test)]
async fn switch_account_with_writer(
    account_id: String,
    write_auth: impl Fn(&Path, &[u8]) -> AppResult<()>,
) -> AppResult<SwitchResult> {
    switch_account_with_writer_and_codex_control(account_id, write_auth, || Ok(()), || Ok(())).await
}

async fn switch_account_with_writer_and_codex_control(
    account_id: String,
    write_auth: impl Fn(&Path, &[u8]) -> AppResult<()>,
    quit_codex: impl Fn() -> AppResult<()>,
    open_codex: impl Fn() -> AppResult<()>,
) -> AppResult<SwitchResult> {
    let account = account_service::get_account(&account_id)?;
    let prepared = prepare_switch_auth(&account).await?;
    let auth_path = paths::default_codex_auth_file()?;
    let config_path = paths::default_codex_config_file()?;
    let previous_provider = previous_launch_provider(&config_path);
    let auth_snapshot = FileSnapshot::capture(&auth_path)?;
    let config_snapshot = FileSnapshot::capture(&config_path)?;
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

    let content = serde_json::to_vec_pretty(&prepared.auth_file).map_err(|err| {
        AppError::new(
            "CODEX_AUTH_SERIALIZE_FAILED",
            format!("Failed to serialize auth file: {}", err),
            "Re-import this account.",
        )
    })?;

    quit_codex()?;

    if let Err(write_error) = write_auth(&auth_path, &content) {
        auth_snapshot.restore();
        return Err(write_error);
    }

    let provider_base_url = match codex_local_access_gateway::ensure_for_account(&account).await {
        Ok(base_url) => base_url,
        Err(error) => {
            auth_snapshot.restore();
            config_snapshot.restore();
            return Err(error);
        }
    };
    if let Err(config_error) = codex_config_service::apply_account_config_with_provider_base_url(
        &account,
        &config_path,
        provider_base_url.as_deref(),
    ) {
        auth_snapshot.restore();
        config_snapshot.restore();
        if let Err(error) = codex_local_access_gateway::restore_for_current_account().await {
            tracing::warn!(
                code = error.code,
                message = error.message,
                "failed to restore gateway after config write failure"
            );
        }
        return Err(config_error);
    }

    let target_provider = codex_config_service::read_active_provider(&config_path)
        .unwrap_or_else(|_| codex_config_service::account_target_provider(&account));
    let mut session_repair: Option<SessionRepairSummary> = None;
    if should_repair_session_visibility(previous_provider.as_deref(), &target_provider) {
        match codex_session_visibility_service::repair_default_codex_home_for_provider(
            &target_provider,
        ) {
            Ok(summary) => session_repair = Some(summary),
            Err(error) => {
                auth_snapshot.restore();
                config_snapshot.restore();
                if let Err(error) = codex_local_access_gateway::restore_for_current_account().await
                {
                    tracing::warn!(
                        code = error.code,
                        message = error.message,
                        "failed to restore gateway after session repair failure"
                    );
                }
                return Err(error);
            }
        }
    }

    // On macOS, Codex reads OAuth credentials from the login keychain too. Update
    // it so the switch fully takes effect. Failure here is non-fatal: auth.json is
    // already written, so we log and continue rather than abort the switch.
    if prepared.uses_oauth_auth {
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
        warnings: prepared.warnings,
        session_repair,
    })
}

#[cfg(test)]
mod tests {
    use super::{previous_launch_provider, switch_account, switch_account_with_writer};
    use crate::infra::{atomic_write, paths, storage};
    use crate::models::account::{AccountsFile, CodexAuthMode};
    use crate::models::error::AppError;
    use crate::services::{auth_file_service, codex_local_access_gateway};
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
    fn previous_launch_provider_reads_config_provider() {
        let _env = TestEnv::new("switch-previous-provider-config");
        let config_path = paths::default_codex_config_file().expect("config path should resolve");
        std::fs::write(&config_path, "model_provider = \"aimami_relay\"\n")
            .expect("config should be written");

        let provider = previous_launch_provider(&config_path).expect("provider should resolve");

        assert_eq!(provider, "aimami_relay");
    }

    #[tokio::test]
    async fn switch_account_writes_target_auth_and_marks_current() {
        let _env = TestEnv::new("switch-success");
        let account = oauth_account();
        let account_id = account.id.clone();
        save_single_account(account);
        let auth_path = paths::default_codex_auth_file().expect("auth path should resolve");
        std::fs::write(&auth_path, TestEnv::fixture_content("api-key.json"))
            .expect("existing auth should be written");

        let result = switch_account_with_writer(account_id.clone(), atomic_write::write_atomic)
            .await
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

    #[tokio::test]
    async fn switch_account_missing_credentials_does_not_write_auth_file() {
        let _env = TestEnv::new("switch-missing-credentials");
        let mut account = oauth_account();
        account.token_bundle = None;
        let account_id = account.id.clone();
        save_single_account(account);
        let auth_path = paths::default_codex_auth_file().expect("auth path should resolve");

        let error = switch_account(account_id)
            .await
            .expect_err("missing credentials should fail");

        assert_eq!(error.code, "CODEX_ACCOUNT_MISSING_CREDENTIALS");
        assert!(!auth_path.exists());
    }

    #[tokio::test]
    async fn switch_account_restores_original_auth_when_write_fails() {
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
        .await
        .expect_err("injected write failure should fail switch");
        let restored_auth =
            std::fs::read_to_string(&auth_path).expect("restored auth should be readable");
        let stored = storage::load_accounts_file().expect("accounts should load");

        assert_eq!(error.code, "TEST_WRITE_FAILED");
        assert_eq!(restored_auth, original_auth);
        assert!(stored.current_account_id.is_none());
    }

    #[tokio::test]
    async fn api_key_account_exports_switchable_auth_file() {
        let _env = TestEnv::new("switch-api-key");
        let auth = auth_file_service::parse_auth_json(&TestEnv::fixture_content("api-key.json"))
            .expect("API key fixture should parse");
        let account =
            auth_file_service::account_from_auth(auth).expect("API key fixture should import");
        let account_id = account.id.clone();
        save_single_account(account);

        switch_account_with_writer(account_id, atomic_write::write_atomic)
            .await
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

    #[tokio::test]
    async fn api_key_account_with_bound_oauth_writes_oauth_auth_and_keeps_api_current() {
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
            .await
            .expect("bound API key switch should succeed");
        let written_auth =
            auth_file_service::read_auth_file(&paths::default_codex_auth_file().unwrap())
                .expect("written auth should parse");
        let written_config = std::fs::read_to_string(paths::default_codex_config_file().unwrap())
            .expect("written config should be readable");
        let stored = storage::load_accounts_file().expect("accounts should load");

        assert!(written_auth.auth_mode.is_none());
        assert!(written_auth.tokens.is_some());
        assert!(written_config.contains("model_provider = \"codex_local_access\""));
        assert!(written_config.contains(&format!(
            "base_url = \"{}\"",
            codex_local_access_gateway::GATEWAY_PROVIDER_BASE_URL
        )));
        assert!(written_config.contains("supports_websockets = false"));
        assert!(!written_config.contains("base_url = \"https://api.openai.com/v1\""));
        assert_eq!(
            stored.current_account_id.as_deref(),
            Some(api_account_id.as_str())
        );
    }
}
