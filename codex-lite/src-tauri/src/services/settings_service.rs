use crate::infra::{paths, storage};
use crate::models::error::AppResult;
use crate::models::settings::AppSettings;

pub fn get_settings() -> AppResult<AppSettings> {
    storage::load_settings()
}

pub fn save_settings(settings: AppSettings) -> AppResult<AppSettings> {
    storage::save_settings(settings)
}

pub fn detect_codex_paths() -> AppResult<AppSettings> {
    let mut settings = storage::load_settings()?;
    settings.codex_home_path = Some(paths::default_codex_home()?.display().to_string());
    settings.auth_file_path = Some(paths::default_codex_auth_file()?.display().to_string());
    storage::save_settings(settings)
}
