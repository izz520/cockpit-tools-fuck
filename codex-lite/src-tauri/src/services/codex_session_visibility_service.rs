use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::Connection;
use serde_json::Value as JsonValue;

use crate::infra::{atomic_write, paths};
use crate::models::error::{AppError, AppResult};

const STATE_DB_FILE: &str = "state_5.sqlite";
const SESSION_DIRS: [&str; 2] = ["sessions", "archived_sessions"];

pub fn repair_default_codex_home_for_provider(target_provider: &str) -> AppResult<()> {
    let codex_home = paths::default_codex_home()?;
    repair_codex_home_for_provider(&codex_home, target_provider)
}

fn repair_codex_home_for_provider(codex_home: &Path, target_provider: &str) -> AppResult<()> {
    if target_provider.trim().is_empty() {
        return Err(AppError::new(
            "CODEX_SESSION_PROVIDER_EMPTY",
            "Cannot repair Codex sessions with an empty provider.",
            "Switch to a valid account and try again.",
        ));
    }

    let rollout_paths = list_rollout_files(codex_home)?;
    for rollout_path in rollout_paths {
        rewrite_rollout_provider(&rollout_path, target_provider)?;
    }

    update_sqlite_threads_provider(codex_home, target_provider)?;
    Ok(())
}

fn list_rollout_files(codex_home: &Path) -> AppResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    for dir_name in SESSION_DIRS {
        let root = codex_home.join(dir_name);
        if root.exists() {
            collect_rollout_files(&root, &mut files)?;
        }
    }
    files.sort();
    Ok(files)
}

fn collect_rollout_files(dir: &Path, files: &mut Vec<PathBuf>) -> AppResult<()> {
    let entries = fs::read_dir(dir).map_err(|err| {
        AppError::new(
            "CODEX_SESSION_DIR_READ_FAILED",
            format!(
                "Failed to read Codex session directory {}: {}",
                dir.display(),
                err
            ),
            "Check Codex session directory permissions.",
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|err| {
            AppError::new(
                "CODEX_SESSION_DIR_READ_FAILED",
                format!("Failed to read Codex session directory entry: {}", err),
                "Check Codex session directory permissions.",
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|err| {
            AppError::new(
                "CODEX_SESSION_ENTRY_READ_FAILED",
                format!(
                    "Failed to inspect Codex session entry {}: {}",
                    path.display(),
                    err
                ),
                "Check Codex session directory permissions.",
            )
        })?;
        if file_type.is_dir() {
            collect_rollout_files(&path, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_name.starts_with("rollout-") && file_name.ends_with(".jsonl") {
            files.push(path);
        }
    }

    Ok(())
}

fn rewrite_rollout_provider(rollout_path: &Path, target_provider: &str) -> AppResult<()> {
    let Some(first_line) = read_first_line(rollout_path)? else {
        return Ok(());
    };
    let Some(updated_first_line) = updated_session_meta_line(&first_line, target_provider)? else {
        return Ok(());
    };

    let original_modified_at = fs::metadata(rollout_path)
        .and_then(|metadata| metadata.modified())
        .ok();
    let bytes = fs::read(rollout_path).map_err(|err| {
        AppError::new(
            "CODEX_ROLLOUT_READ_FAILED",
            format!(
                "Failed to read Codex rollout {}: {}",
                rollout_path.display(),
                err
            ),
            "Check Codex session file permissions.",
        )
    })?;
    let (offset, separator) = first_line_boundary(&bytes);
    let mut next_bytes = Vec::with_capacity(updated_first_line.len() + bytes.len());
    next_bytes.extend_from_slice(updated_first_line.as_bytes());
    next_bytes.extend_from_slice(separator.as_bytes());
    next_bytes.extend_from_slice(&bytes[offset..]);
    atomic_write::write_atomic(rollout_path, &next_bytes)?;
    restore_modified_time(rollout_path, original_modified_at)?;
    Ok(())
}

fn read_first_line(path: &Path) -> AppResult<Option<String>> {
    let file = fs::File::open(path).map_err(|err| {
        AppError::new(
            "CODEX_ROLLOUT_OPEN_FAILED",
            format!("Failed to open Codex rollout {}: {}", path.display(), err),
            "Check Codex session file permissions.",
        )
    })?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).map_err(|err| {
        AppError::new(
            "CODEX_ROLLOUT_READ_FAILED",
            format!(
                "Failed to read Codex rollout first line {}: {}",
                path.display(),
                err
            ),
            "Check Codex session file permissions.",
        )
    })?;
    if bytes == 0 {
        return Ok(None);
    }
    Ok(Some(line.trim_end_matches(['\r', '\n']).to_string()))
}

fn updated_session_meta_line(first_line: &str, target_provider: &str) -> AppResult<Option<String>> {
    let mut parsed: JsonValue = match serde_json::from_str(first_line) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    if parsed.get("type").and_then(JsonValue::as_str) != Some("session_meta") {
        return Ok(None);
    }
    let Some(payload) = parsed.get_mut("payload").and_then(JsonValue::as_object_mut) else {
        return Ok(None);
    };
    if payload.get("model_provider").and_then(JsonValue::as_str) == Some(target_provider) {
        return Ok(None);
    }
    payload.insert(
        "model_provider".to_string(),
        JsonValue::String(target_provider.to_string()),
    );
    let line = serde_json::to_string(&parsed).map_err(|err| {
        AppError::new(
            "CODEX_ROLLOUT_SERIALIZE_FAILED",
            format!("Failed to serialize Codex rollout session_meta: {}", err),
            "Check the rollout file format.",
        )
    })?;
    Ok(Some(line))
}

fn first_line_boundary(bytes: &[u8]) -> (usize, &'static str) {
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            if index > 0 && bytes[index - 1] == b'\r' {
                return (index + 1, "\r\n");
            }
            return (index + 1, "\n");
        }
    }
    (bytes.len(), "")
}

#[cfg(unix)]
fn restore_modified_time(path: &Path, modified_at: Option<SystemTime>) -> AppResult<()> {
    let Some(modified_at) = modified_at else {
        return Ok(());
    };
    let times = fs::FileTimes::new().set_modified(modified_at);
    fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_times(times))
        .map_err(|err| {
            AppError::new(
                "CODEX_ROLLOUT_TIME_RESTORE_FAILED",
                format!(
                    "Failed to restore Codex rollout modified time {}: {}",
                    path.display(),
                    err
                ),
                "Check Codex session file permissions.",
            )
        })
}

#[cfg(not(unix))]
fn restore_modified_time(_path: &Path, _modified_at: Option<SystemTime>) -> AppResult<()> {
    Ok(())
}

fn update_sqlite_threads_provider(codex_home: &Path, target_provider: &str) -> AppResult<usize> {
    let db_path = codex_home.join(STATE_DB_FILE);
    if !db_path.exists() {
        return Ok(0);
    }

    let mut connection = Connection::open(&db_path).map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_OPEN_FAILED",
            format!(
                "Failed to open Codex state database {}: {}",
                db_path.display(),
                err
            ),
            "Close Codex and try switching accounts again.",
        )
    })?;
    connection
        .busy_timeout(std::time::Duration::from_secs(3))
        .map_err(|err| {
            AppError::new(
                "CODEX_STATE_DB_BUSY_TIMEOUT_FAILED",
                format!(
                    "Failed to configure SQLite busy timeout {}: {}",
                    db_path.display(),
                    err
                ),
                "Close Codex and try switching accounts again.",
            )
        })?;
    let columns = read_threads_columns(&connection)?;
    if !columns.contains("model_provider") {
        return Ok(0);
    }

    let transaction = connection.transaction().map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_WRITE_FAILED",
            format!(
                "Failed to start Codex state transaction {}: {}",
                db_path.display(),
                err
            ),
            "Close Codex and try switching accounts again.",
        )
    })?;
    let updated = transaction
        .execute(
            "UPDATE threads SET model_provider = ?1 WHERE COALESCE(model_provider, '') <> ?1",
            [target_provider],
        )
        .map_err(|err| {
            AppError::new(
                "CODEX_STATE_DB_WRITE_FAILED",
                format!(
                    "Failed to update Codex thread provider metadata {}: {}",
                    db_path.display(),
                    err
                ),
                "Close Codex and try switching accounts again.",
            )
        })?;
    transaction.commit().map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_WRITE_FAILED",
            format!(
                "Failed to commit Codex state transaction {}: {}",
                db_path.display(),
                err
            ),
            "Close Codex and try switching accounts again.",
        )
    })?;

    Ok(updated)
}

fn read_threads_columns(connection: &Connection) -> AppResult<HashSet<String>> {
    let mut statement = connection
        .prepare("PRAGMA table_info(threads)")
        .map_err(|err| {
            AppError::new(
                "CODEX_STATE_DB_SCHEMA_READ_FAILED",
                format!("Failed to read Codex threads table schema: {}", err),
                "Close Codex and try switching accounts again.",
            )
        })?;
    let rows = statement
        .query_map([], |row| row.get::<usize, String>(1))
        .map_err(|err| {
            AppError::new(
                "CODEX_STATE_DB_SCHEMA_READ_FAILED",
                format!("Failed to query Codex threads table schema: {}", err),
                "Close Codex and try switching accounts again.",
            )
        })?;
    let mut columns = HashSet::new();
    for row in rows {
        columns.insert(row.map_err(|err| {
            AppError::new(
                "CODEX_STATE_DB_SCHEMA_READ_FAILED",
                format!("Failed to parse Codex threads table schema: {}", err),
                "Close Codex and try switching accounts again.",
            )
        })?);
    }
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::{repair_codex_home_for_provider, update_sqlite_threads_provider, STATE_DB_FILE};
    use rusqlite::Connection;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("codex-lite-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[test]
    fn repairs_rollout_session_meta_provider() {
        let root = temp_dir("session-provider");
        let rollout_dir = root.join("sessions").join("2026").join("06").join("13");
        std::fs::create_dir_all(&rollout_dir).expect("rollout dir should be created");
        let rollout = rollout_dir.join("rollout-test.jsonl");
        std::fs::write(
            &rollout,
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"s1\",\"model_provider\":\"openai\"}}\n{\"type\":\"event\"}\n",
        )
        .expect("rollout should be written");

        repair_codex_home_for_provider(&root, "codex_lite_api_key").expect("repair should work");

        let content = std::fs::read_to_string(&rollout).expect("rollout should be readable");
        assert!(content.contains("\"model_provider\":\"codex_lite_api_key\""));
        assert!(content.contains("{\"type\":\"event\"}"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn repairs_sqlite_thread_provider() {
        let root = temp_dir("session-sqlite");
        let db_path = root.join(STATE_DB_FILE);
        let connection = Connection::open(&db_path).expect("database should open");
        connection
            .execute(
                "CREATE TABLE threads (id TEXT PRIMARY KEY, model_provider TEXT)",
                [],
            )
            .expect("threads table should be created");
        connection
            .execute(
                "INSERT INTO threads (id, model_provider) VALUES ('old', 'openai'), ('same', 'relay')",
                [],
            )
            .expect("rows should be inserted");
        drop(connection);

        let updated = update_sqlite_threads_provider(&root, "relay").expect("update should work");
        assert_eq!(updated, 1);

        let connection = Connection::open(&db_path).expect("database should reopen");
        let provider: String = connection
            .query_row(
                "SELECT model_provider FROM threads WHERE id = 'old'",
                [],
                |row| row.get(0),
            )
            .expect("provider should be read");
        assert_eq!(provider, "relay");
        let _ = std::fs::remove_dir_all(root);
    }
}
