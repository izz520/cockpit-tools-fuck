use crate::models::error::AppResult;
use crate::models::settings::AppSettings;
use crate::services::settings_service;

#[tauri::command]
pub fn get_settings() -> AppResult<AppSettings> {
    settings_service::get_settings()
}

#[tauri::command]
pub fn save_settings(settings: AppSettings) -> AppResult<AppSettings> {
    settings_service::save_settings(settings)
}

#[tauri::command]
pub fn detect_codex_paths() -> AppResult<AppSettings> {
    settings_service::detect_codex_paths()
}
