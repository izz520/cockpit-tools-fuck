use std::fs;

use crate::infra::{atomic_write, paths};
use crate::models::account::AccountsFile;
use crate::models::error::{AppError, AppResult};
use crate::models::settings::AppSettings;

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn load_accounts_file() -> AppResult<AccountsFile> {
    let path = paths::accounts_file_path()?;
    if !path.exists() {
        return Ok(AccountsFile {
            schema_version: "1.0.0".to_string(),
            current_account_id: None,
            accounts: Vec::new(),
            updated_at: now_timestamp(),
        });
    }

    let content = fs::read_to_string(&path).map_err(|err| {
        AppError::new(
            "STORAGE_READ_FAILED",
            format!("Failed to read {}: {}", path.display(), err),
            "Open the data directory and check file permissions.",
        )
    })?;
    serde_json::from_str(&content).map_err(|err| {
        AppError::new(
            "STORAGE_INVALID_FORMAT",
            format!("Failed to parse {}: {}", path.display(), err),
            "Back up the file, then re-import accounts.",
        )
    })
}

pub fn save_accounts_file(mut file: AccountsFile) -> AppResult<()> {
    file.updated_at = now_timestamp();
    let path = paths::accounts_file_path()?;
    let content = serde_json::to_vec_pretty(&file).map_err(|err| {
        AppError::new(
            "STORAGE_SERIALIZE_FAILED",
            format!("Failed to serialize accounts: {}", err),
            "Try again.",
        )
    })?;
    atomic_write::write_atomic(&path, &content)
}

pub fn load_settings() -> AppResult<AppSettings> {
    let path = paths::settings_file_path()?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        AppError::new(
            "SETTINGS_READ_FAILED",
            format!("Failed to read {}: {}", path.display(), err),
            "Open the data directory and check file permissions.",
        )
    })?;
    serde_json::from_str(&content).map_err(|err| {
        AppError::new(
            "SETTINGS_INVALID_FORMAT",
            format!("Failed to parse {}: {}", path.display(), err),
            "Reset settings or edit the file manually.",
        )
    })
}

pub fn save_settings(settings: AppSettings) -> AppResult<AppSettings> {
    let path = paths::settings_file_path()?;
    let content = serde_json::to_vec_pretty(&settings).map_err(|err| {
        AppError::new(
            "SETTINGS_SERIALIZE_FAILED",
            format!("Failed to serialize settings: {}", err),
            "Try again.",
        )
    })?;
    atomic_write::write_atomic(&path, &content)?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::{load_accounts_file, load_settings, save_settings};
    use crate::infra::paths;
    use crate::models::settings::AppSettings;
    use crate::test_support::TestEnv;

    #[test]
    fn load_accounts_file_returns_default_for_empty_data_dir() {
        let _env = TestEnv::new("empty-accounts");

        let file = load_accounts_file().expect("empty data dir should load default accounts");

        assert_eq!(file.schema_version, "1.0.0");
        assert!(file.current_account_id.is_none());
        assert!(file.accounts.is_empty());
    }

    #[test]
    fn load_accounts_file_rejects_corrupt_json() {
        let _env = TestEnv::new("corrupt-accounts");
        let path = paths::accounts_file_path().expect("accounts path should resolve");
        std::fs::write(&path, "{not-json").expect("corrupt accounts file should be written");

        let error = load_accounts_file().expect_err("corrupt accounts file should fail");

        assert_eq!(error.code, "STORAGE_INVALID_FORMAT");
    }

    #[test]
    fn save_settings_can_be_loaded_again() {
        let _env = TestEnv::new("settings-roundtrip");
        let settings = AppSettings {
            schema_version: "1.0.0".to_string(),
            codex_home_path: Some("/tmp/codex-home".to_string()),
            auth_file_path: Some("/tmp/codex-home/auth.json".to_string()),
            theme: "dark".to_string(),
            quota_refresh_on_start: true,
        };

        save_settings(settings.clone()).expect("settings should save");
        let loaded = load_settings().expect("settings should load");

        assert_eq!(loaded.schema_version, settings.schema_version);
        assert_eq!(loaded.codex_home_path, settings.codex_home_path);
        assert_eq!(loaded.auth_file_path, settings.auth_file_path);
        assert_eq!(loaded.theme, settings.theme);
        assert_eq!(
            loaded.quota_refresh_on_start,
            settings.quota_refresh_on_start
        );
    }
}
