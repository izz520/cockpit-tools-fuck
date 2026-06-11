use std::fs;
use std::io::{BufRead, BufReader};

use tauri_plugin_opener::OpenerExt;

use crate::infra::{paths, redaction};
use crate::models::error::{AppError, AppResult};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub timestamp: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSnapshot {
    pub entries: Vec<LogEntry>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSnapshot {
    pub app_data_dir: String,
    pub logs_dir: String,
    pub accounts_file_path: String,
    pub settings_file_path: String,
    pub default_codex_home: String,
    pub default_codex_auth_file: String,
    pub codex_auth_file_exists: bool,
}

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn latest_log_file_path() -> AppResult<Option<std::path::PathBuf>> {
    let dir = paths::logs_dir()?;
    if !dir.exists() {
        return Ok(None);
    }

    let mut candidates = fs::read_dir(&dir)
        .map_err(|err| {
            AppError::new(
                "LOG_DIR_READ_FAILED",
                format!("Failed to read {}: {}", dir.display(), err),
                "Check app log directory permissions.",
            )
        })?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();

    candidates.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    });

    Ok(candidates.pop())
}

#[tauri::command]
pub fn open_data_dir(app: tauri::AppHandle) -> AppResult<()> {
    let dir = paths::app_data_dir()?;
    fs::create_dir_all(&dir).map_err(|err| {
        AppError::new(
            "OPEN_DATA_DIR_FAILED",
            format!("Failed to create {}: {}", dir.display(), err),
            "Check app data permissions.",
        )
    })?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|err| {
            AppError::new(
                "OPEN_DATA_DIR_FAILED",
                format!("Failed to open data directory: {}", err),
                "Open it manually from the filesystem.",
            )
        })
}

#[tauri::command]
pub fn open_log_dir(app: tauri::AppHandle) -> AppResult<()> {
    let dir = paths::logs_dir()?;
    fs::create_dir_all(&dir).map_err(|err| {
        AppError::new(
            "OPEN_LOG_DIR_FAILED",
            format!("Failed to create {}: {}", dir.display(), err),
            "Check app data permissions.",
        )
    })?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|err| {
            AppError::new(
                "OPEN_LOG_DIR_FAILED",
                format!("Failed to open log directory: {}", err),
                "Open it manually from the filesystem.",
            )
        })
}

#[tauri::command]
pub fn get_log_snapshot(_max_lines: u32) -> AppResult<LogSnapshot> {
    let max_lines = _max_lines.clamp(1, 500) as usize;
    let Some(path) = latest_log_file_path()? else {
        return Ok(LogSnapshot {
            entries: Vec::new(),
        });
    };
    let file = fs::File::open(&path).map_err(|err| {
        AppError::new(
            "LOG_FILE_READ_FAILED",
            format!("Failed to read {}: {}", path.display(), err),
            "Check app log file permissions.",
        )
    })?;
    let reader = BufReader::new(file);
    let lines = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::new(
                "LOG_FILE_PARSE_FAILED",
                format!("Failed to parse {}: {}", path.display(), err),
                "Open the log file manually.",
            )
        })?;
    let entries = lines
        .into_iter()
        .rev()
        .take(max_lines)
        .map(|line| LogEntry {
            level: if line.to_ascii_lowercase().contains("error") {
                "error".to_string()
            } else if line.to_ascii_lowercase().contains("warn") {
                "warn".to_string()
            } else {
                "info".to_string()
            },
            message: redaction::redact_sensitive(&line),
            timestamp: now_timestamp(),
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(LogSnapshot { entries })
}

#[tauri::command]
pub fn get_system_snapshot() -> AppResult<SystemSnapshot> {
    let app_data_dir = paths::app_data_dir()?;
    let logs_dir = paths::logs_dir()?;
    let accounts_file_path = paths::accounts_file_path()?;
    let settings_file_path = paths::settings_file_path()?;
    let default_codex_home = paths::default_codex_home()?;
    let default_codex_auth_file = paths::default_codex_auth_file()?;
    let codex_auth_file_exists = default_codex_auth_file.exists();

    Ok(SystemSnapshot {
        app_data_dir: app_data_dir.display().to_string(),
        logs_dir: logs_dir.display().to_string(),
        accounts_file_path: accounts_file_path.display().to_string(),
        settings_file_path: settings_file_path.display().to_string(),
        default_codex_home: default_codex_home.display().to_string(),
        default_codex_auth_file: default_codex_auth_file.display().to_string(),
        codex_auth_file_exists,
    })
}
