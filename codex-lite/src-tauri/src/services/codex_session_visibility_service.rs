use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::Connection;
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::infra::{atomic_write, paths, storage};
use crate::models::error::{AppError, AppResult};
use crate::models::session::{CodexSessionView, SessionMutationResult};
use crate::services::{codex_app_service, codex_config_service};

const STATE_DB_FILE: &str = "state_5.sqlite";
const SESSION_INDEX_FILE: &str = "session_index.jsonl";
const SESSION_DIRS: [&str; 2] = ["sessions", "archived_sessions"];

#[derive(Debug, Clone)]
struct RolloutSessionMeta {
    id: String,
    provider: String,
    title: Option<String>,
    cwd: Option<String>,
    rollout_path: PathBuf,
    archived: bool,
    updated_at: Option<i64>,
}

#[derive(Debug, Clone)]
struct SqliteSessionRow {
    id: String,
    title: String,
    cwd: String,
    provider: String,
    archived: bool,
    updated_at: Option<i64>,
    created_at: Option<i64>,
    rollout_path: Option<String>,
    preview: Option<String>,
}

pub fn repair_default_codex_home_for_provider(target_provider: &str) -> AppResult<()> {
    let codex_home = paths::default_codex_home()?;
    repair_codex_home_for_provider(&codex_home, target_provider)
}

pub fn list_default_codex_sessions() -> AppResult<Vec<CodexSessionView>> {
    let codex_home = paths::default_codex_home()?;
    let target_provider = current_target_provider()?;
    list_codex_sessions(&codex_home, &target_provider)
}

pub fn restore_default_codex_sessions_visibility(
    session_ids: Vec<String>,
) -> AppResult<SessionMutationResult> {
    codex_app_service::quit_codex_for_switch()?;
    let codex_home = paths::default_codex_home()?;
    let target_provider = current_target_provider()?;
    let restore_result = restore_sessions_visibility(&codex_home, &target_provider, &session_ids);
    let open_result = codex_app_service::open_codex_after_switch();

    match (restore_result, open_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(err), _) => Err(err),
        (Ok(_), Err(err)) => Err(err),
    }
}

pub fn delete_default_codex_sessions(session_ids: Vec<String>) -> AppResult<SessionMutationResult> {
    let codex_home = paths::default_codex_home()?;
    delete_sessions(&codex_home, &session_ids)
}

fn current_target_provider() -> AppResult<String> {
    let accounts_file = storage::load_accounts_file()?;
    if let Some(current_account_id) = accounts_file.current_account_id {
        if let Some(account) = accounts_file
            .accounts
            .iter()
            .find(|account| account.id == current_account_id)
        {
            return Ok(codex_config_service::account_target_provider(account));
        }
    }

    codex_config_service::read_active_provider(&paths::default_codex_config_file()?)
}

fn list_codex_sessions(
    codex_home: &Path,
    target_provider: &str,
) -> AppResult<Vec<CodexSessionView>> {
    let mut sessions = BTreeMap::<String, CodexSessionView>::new();

    for meta in read_rollout_session_metas(codex_home)? {
        let title = meta
            .title
            .clone()
            .unwrap_or_else(|| display_title_from_id(&meta.id));
        let cwd = meta.cwd.clone().unwrap_or_default();
        sessions.insert(
            meta.id.clone(),
            CodexSessionView {
                id: meta.id,
                title,
                project: project_name(&cwd),
                cwd,
                provider: meta.provider,
                target_provider: target_provider.to_string(),
                visible: false,
                archived: meta.archived,
                updated_at: meta.updated_at,
                created_at: None,
                rollout_path: Some(meta.rollout_path.display().to_string()),
                preview: None,
            },
        );
    }

    for row in read_sqlite_session_rows(codex_home)? {
        let rollout_path = row
            .rollout_path
            .clone()
            .filter(|value| !value.trim().is_empty());
        let title = if row.title.trim().is_empty() {
            display_title_from_id(&row.id)
        } else {
            row.title.clone()
        };
        sessions
            .entry(row.id.clone())
            .and_modify(|session| {
                session.title = title.clone();
                session.cwd = row.cwd.clone();
                session.project = project_name(&row.cwd);
                session.provider = row.provider.clone();
                session.visible = row.provider == target_provider;
                session.archived = row.archived;
                session.updated_at = row.updated_at.or(session.updated_at);
                session.created_at = row.created_at;
                session.rollout_path = rollout_path
                    .clone()
                    .or_else(|| session.rollout_path.clone());
                session.preview = row.preview.clone();
            })
            .or_insert_with(|| CodexSessionView {
                id: row.id,
                title,
                project: project_name(&row.cwd),
                cwd: row.cwd,
                provider: row.provider.clone(),
                target_provider: target_provider.to_string(),
                visible: row.provider == target_provider,
                archived: row.archived,
                updated_at: row.updated_at,
                created_at: row.created_at,
                rollout_path,
                preview: row.preview,
            });
    }

    let mut result = sessions.into_values().collect::<Vec<_>>();
    result.sort_by(|left, right| {
        right
            .updated_at
            .unwrap_or(0)
            .cmp(&left.updated_at.unwrap_or(0))
            .then_with(|| left.title.cmp(&right.title))
    });
    Ok(result)
}

fn restore_sessions_visibility(
    codex_home: &Path,
    target_provider: &str,
    session_ids: &[String],
) -> AppResult<SessionMutationResult> {
    let selected_ids = normalized_id_set(session_ids)?;
    let mut updated_count =
        update_sqlite_selected_threads_provider(codex_home, target_provider, &selected_ids)?;

    let metas = read_rollout_session_metas(codex_home)?;
    for meta in &metas {
        if selected_ids.contains(&meta.id) {
            rewrite_rollout_provider(&meta.rollout_path, target_provider)?;
            updated_count += 1;
        }
    }
    updated_count += upsert_session_index_entries(codex_home, &metas, &selected_ids)?;

    Ok(SessionMutationResult {
        updated_count,
        deleted_count: 0,
    })
}

fn delete_sessions(codex_home: &Path, session_ids: &[String]) -> AppResult<SessionMutationResult> {
    let selected_ids = normalized_id_set(session_ids)?;
    let mut deleted_count = delete_sqlite_sessions(codex_home, &selected_ids)?;

    for meta in read_rollout_session_metas(codex_home)? {
        if selected_ids.contains(&meta.id) {
            match fs::remove_file(&meta.rollout_path) {
                Ok(()) => deleted_count += 1,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(AppError::new(
                        "CODEX_SESSION_DELETE_FAILED",
                        format!(
                            "Failed to delete Codex rollout {}: {}",
                            meta.rollout_path.display(),
                            err
                        ),
                        "Close Codex and try deleting the session again.",
                    ));
                }
            }
        }
    }
    remove_session_index_entries(codex_home, &selected_ids)?;

    Ok(SessionMutationResult {
        updated_count: 0,
        deleted_count,
    })
}

fn repair_codex_home_for_provider(codex_home: &Path, target_provider: &str) -> AppResult<()> {
    if target_provider.trim().is_empty() {
        return Err(AppError::new(
            "CODEX_SESSION_PROVIDER_EMPTY",
            "Cannot repair Codex sessions with an empty provider.",
            "Switch to a valid account and try again.",
        ));
    }

    let metas = read_rollout_session_metas(codex_home)?;
    for meta in &metas {
        rewrite_rollout_provider(&meta.rollout_path, target_provider)?;
    }

    update_sqlite_threads_provider(codex_home, target_provider)?;
    let all_ids = metas
        .iter()
        .map(|meta| meta.id.clone())
        .collect::<HashSet<_>>();
    if !all_ids.is_empty() {
        upsert_session_index_entries(codex_home, &metas, &all_ids)?;
    }
    Ok(())
}

fn read_rollout_session_metas(codex_home: &Path) -> AppResult<Vec<RolloutSessionMeta>> {
    let mut result = Vec::new();
    for rollout_path in list_rollout_files(codex_home)? {
        let Some(first_line) = read_first_line(&rollout_path)? else {
            continue;
        };
        let Some(mut meta) = parse_rollout_session_meta(&first_line, rollout_path.clone())? else {
            continue;
        };
        meta.archived = rollout_path
            .strip_prefix(codex_home)
            .ok()
            .and_then(|relative| relative.components().next())
            .and_then(|component| component.as_os_str().to_str())
            == Some("archived_sessions");
        result.push(meta);
    }
    Ok(result)
}

fn parse_rollout_session_meta(
    first_line: &str,
    rollout_path: PathBuf,
) -> AppResult<Option<RolloutSessionMeta>> {
    let parsed: JsonValue = match serde_json::from_str(first_line) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    if parsed.get("type").and_then(JsonValue::as_str) != Some("session_meta") {
        return Ok(None);
    }
    let Some(payload) = parsed.get("payload") else {
        return Ok(None);
    };
    let Some(id) = payload.get("id").and_then(JsonValue::as_str) else {
        return Ok(None);
    };
    let updated_at = rollout_modified_at_ms(payload).or_else(|| file_modified_at_ms(&rollout_path));
    Ok(Some(RolloutSessionMeta {
        id: id.to_string(),
        provider: payload
            .get("model_provider")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        title: payload
            .get("thread_name")
            .or_else(|| payload.get("title"))
            .and_then(JsonValue::as_str)
            .map(ToString::to_string),
        cwd: payload
            .get("cwd")
            .or_else(|| payload.get("workspace_root"))
            .and_then(JsonValue::as_str)
            .map(ToString::to_string),
        rollout_path,
        archived: false,
        updated_at,
    }))
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

fn open_state_connection(codex_home: &Path) -> AppResult<Option<Connection>> {
    let db_path = codex_home.join(STATE_DB_FILE);
    if !db_path.exists() {
        return Ok(None);
    }
    let connection = Connection::open(&db_path).map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_OPEN_FAILED",
            format!(
                "Failed to open Codex state database {}: {}",
                db_path.display(),
                err
            ),
            "Close Codex and try again.",
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
                "Close Codex and try again.",
            )
        })?;
    Ok(Some(connection))
}

fn read_sqlite_session_rows(codex_home: &Path) -> AppResult<Vec<SqliteSessionRow>> {
    let Some(connection) = open_state_connection(codex_home)? else {
        return Ok(Vec::new());
    };
    let columns = read_threads_columns(&connection)?;
    if columns.is_empty() || !columns.contains("id") {
        return Ok(Vec::new());
    }

    let select_exprs = [
        ("id", "id"),
        ("title", "title"),
        ("cwd", "cwd"),
        ("model_provider", "provider"),
        ("archived", "archived"),
        ("updated_at_ms", "updated_at_ms"),
        ("updated_at", "updated_at"),
        ("created_at_ms", "created_at_ms"),
        ("created_at", "created_at"),
        ("rollout_path", "rollout_path"),
        ("preview", "preview"),
        ("first_user_message", "first_user_message"),
    ]
    .into_iter()
    .filter_map(|(column, alias)| {
        if columns.contains(column) {
            Some(format!("{column} AS {alias}"))
        } else {
            None
        }
    })
    .collect::<Vec<_>>();

    let sql = format!("SELECT {} FROM threads", select_exprs.join(", "));
    let mut statement = connection.prepare(&sql).map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_QUERY_FAILED",
            format!("Failed to prepare Codex session query: {}", err),
            "Close Codex and try again.",
        )
    })?;
    let mapped = statement.query_map([], |row| {
        let id = get_row_string(row, "id")?;
        let title = get_row_string(row, "title").unwrap_or_else(|_| String::new());
        let cwd = get_row_string(row, "cwd").unwrap_or_else(|_| String::new());
        let provider = get_row_string(row, "provider").unwrap_or_else(|_| String::new());
        let archived = get_row_i64(row, "archived").unwrap_or(0) != 0;
        let updated_at = get_row_i64(row, "updated_at_ms")
            .or_else(|_| get_row_i64(row, "updated_at"))
            .ok();
        let created_at = get_row_i64(row, "created_at_ms")
            .or_else(|_| get_row_i64(row, "created_at"))
            .ok();
        let rollout_path = get_row_optional_string(row, "rollout_path").ok().flatten();
        let preview = get_row_optional_string(row, "preview")
            .ok()
            .flatten()
            .or_else(|| {
                get_row_optional_string(row, "first_user_message")
                    .ok()
                    .flatten()
            });
        Ok(SqliteSessionRow {
            id,
            title,
            cwd,
            provider,
            archived,
            updated_at,
            created_at,
            rollout_path,
            preview,
        })
    });
    let rows = mapped.map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_QUERY_FAILED",
            format!("Failed to query Codex sessions: {}", err),
            "Close Codex and try again.",
        )
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|err| {
            AppError::new(
                "CODEX_STATE_DB_QUERY_FAILED",
                format!("Failed to read Codex session row: {}", err),
                "Close Codex and try again.",
            )
        })?);
    }
    Ok(result)
}

fn update_sqlite_selected_threads_provider(
    codex_home: &Path,
    target_provider: &str,
    selected_ids: &HashSet<String>,
) -> AppResult<usize> {
    let Some(mut connection) = open_state_connection(codex_home)? else {
        return Ok(0);
    };
    let columns = read_threads_columns(&connection)?;
    if !columns.contains("id") || !columns.contains("model_provider") {
        return Ok(0);
    }
    let transaction = connection.transaction().map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_WRITE_FAILED",
            format!("Failed to start Codex session restore transaction: {}", err),
            "Close Codex and try again.",
        )
    })?;
    let mut updated = 0usize;
    for id in selected_ids {
        updated += transaction
            .execute(
                "UPDATE threads SET model_provider = ?1 WHERE id = ?2 AND COALESCE(model_provider, '') <> ?1",
                (target_provider, id),
            )
            .map_err(|err| {
                AppError::new(
                    "CODEX_STATE_DB_WRITE_FAILED",
                    format!("Failed to restore Codex session visibility for {}: {}", id, err),
                    "Close Codex and try again.",
                )
            })?;
    }
    transaction.commit().map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_WRITE_FAILED",
            format!(
                "Failed to commit Codex session restore transaction: {}",
                err
            ),
            "Close Codex and try again.",
        )
    })?;
    Ok(updated)
}

fn delete_sqlite_sessions(codex_home: &Path, selected_ids: &HashSet<String>) -> AppResult<usize> {
    let Some(mut connection) = open_state_connection(codex_home)? else {
        return Ok(0);
    };
    let columns = read_threads_columns(&connection)?;
    if !columns.contains("id") {
        return Ok(0);
    }
    let transaction = connection.transaction().map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_WRITE_FAILED",
            format!("Failed to start Codex session delete transaction: {}", err),
            "Close Codex and try again.",
        )
    })?;
    let mut deleted = 0usize;
    for id in selected_ids {
        deleted += transaction
            .execute("DELETE FROM threads WHERE id = ?1", [id])
            .map_err(|err| {
                AppError::new(
                    "CODEX_STATE_DB_WRITE_FAILED",
                    format!(
                        "Failed to delete Codex session {} from state database: {}",
                        id, err
                    ),
                    "Close Codex and try again.",
                )
            })?;
    }
    transaction.commit().map_err(|err| {
        AppError::new(
            "CODEX_STATE_DB_WRITE_FAILED",
            format!("Failed to commit Codex session delete transaction: {}", err),
            "Close Codex and try again.",
        )
    })?;
    Ok(deleted)
}

fn remove_session_index_entries(
    codex_home: &Path,
    selected_ids: &HashSet<String>,
) -> AppResult<()> {
    let index_path = codex_home.join(SESSION_INDEX_FILE);
    if !index_path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&index_path).map_err(|err| {
        AppError::new(
            "CODEX_SESSION_INDEX_READ_FAILED",
            format!("Failed to read {}: {}", index_path.display(), err),
            "Close Codex and try again.",
        )
    })?;
    let mut changed = false;
    let mut kept_lines = Vec::new();
    for line in content.lines() {
        let should_delete = serde_json::from_str::<JsonValue>(line)
            .ok()
            .and_then(|value| {
                value
                    .get("id")
                    .and_then(JsonValue::as_str)
                    .map(ToString::to_string)
            })
            .is_some_and(|id| selected_ids.contains(&id));
        if should_delete {
            changed = true;
        } else {
            kept_lines.push(line);
        }
    }
    if !changed {
        return Ok(());
    }
    let next = if kept_lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", kept_lines.join("\n"))
    };
    atomic_write::write_atomic(&index_path, next.as_bytes())
}

fn upsert_session_index_entries(
    codex_home: &Path,
    metas: &[RolloutSessionMeta],
    selected_ids: &HashSet<String>,
) -> AppResult<usize> {
    let index_path = codex_home.join(SESSION_INDEX_FILE);
    let meta_by_id = metas
        .iter()
        .filter(|meta| selected_ids.contains(&meta.id))
        .map(|meta| (meta.id.as_str(), meta))
        .collect::<BTreeMap<_, _>>();

    if meta_by_id.is_empty() {
        return Ok(0);
    }

    let content = match fs::read_to_string(&index_path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Err(AppError::new(
                "CODEX_SESSION_INDEX_READ_FAILED",
                format!("Failed to read {}: {}", index_path.display(), err),
                "Close Codex and try again.",
            ));
        }
    };

    let mut changed = false;
    let mut indexed_ids = HashSet::new();
    let mut lines = Vec::new();
    for line in content.lines() {
        let Some(id) = session_index_line_id(line) else {
            lines.push(line.to_string());
            continue;
        };
        indexed_ids.insert(id.clone());
        if let Some(meta) = meta_by_id.get(id.as_str()) {
            let next_line = build_session_index_line(meta)?;
            if next_line != line {
                changed = true;
            }
            lines.push(next_line);
        } else {
            lines.push(line.to_string());
        }
    }

    for (id, meta) in &meta_by_id {
        if indexed_ids.contains(*id) {
            continue;
        }
        lines.push(build_session_index_line(meta)?);
        changed = true;
    }

    if !changed {
        return Ok(0);
    }

    let next = if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    };
    atomic_write::write_atomic(&index_path, next.as_bytes())?;
    Ok(meta_by_id.len())
}

fn session_index_line_id(line: &str) -> Option<String> {
    serde_json::from_str::<JsonValue>(line)
        .ok()
        .and_then(|value| {
            value
                .get("id")
                .and_then(JsonValue::as_str)
                .map(str::to_string)
        })
}

fn build_session_index_line(meta: &RolloutSessionMeta) -> AppResult<String> {
    let mut object = JsonMap::new();
    object.insert("id".to_string(), JsonValue::String(meta.id.clone()));
    object.insert(
        "thread_name".to_string(),
        JsonValue::String(
            meta.title
                .clone()
                .unwrap_or_else(|| display_title_from_id(&meta.id)),
        ),
    );
    object.insert(
        "updated_at".to_string(),
        JsonValue::String(
            meta.updated_at
                .and_then(timestamp_ms_to_rfc3339)
                .unwrap_or_else(|| {
                    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
                }),
        ),
    );

    serde_json::to_string(&JsonValue::Object(object)).map_err(|err| {
        AppError::new(
            "CODEX_SESSION_INDEX_SERIALIZE_FAILED",
            format!("Failed to serialize Codex session index entry: {}", err),
            "Check the Codex session index format.",
        )
    })
}

fn timestamp_ms_to_rfc3339(timestamp_ms: i64) -> Option<String> {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms)
        .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
}

fn normalized_id_set(session_ids: &[String]) -> AppResult<HashSet<String>> {
    let ids = session_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(ToString::to_string)
        .collect::<HashSet<_>>();
    if ids.is_empty() {
        return Err(AppError::new(
            "CODEX_SESSION_SELECTION_EMPTY",
            "No sessions were selected.",
            "Select at least one session and try again.",
        ));
    }
    Ok(ids)
}

fn get_row_string(row: &rusqlite::Row<'_>, name: &str) -> rusqlite::Result<String> {
    row.get::<&str, String>(name)
}

fn get_row_optional_string(
    row: &rusqlite::Row<'_>,
    name: &str,
) -> rusqlite::Result<Option<String>> {
    row.get::<&str, Option<String>>(name)
}

fn get_row_i64(row: &rusqlite::Row<'_>, name: &str) -> rusqlite::Result<i64> {
    row.get::<&str, i64>(name)
}

fn rollout_modified_at_ms(payload: &JsonValue) -> Option<i64> {
    payload
        .get("updated_at_ms")
        .or_else(|| payload.get("updated_at"))
        .and_then(JsonValue::as_i64)
}

fn file_modified_at_ms(path: &Path) -> Option<i64> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
}

fn project_name(cwd: &str) -> String {
    Path::new(cwd)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Unknown project")
        .to_string()
}

fn display_title_from_id(id: &str) -> String {
    if id.len() <= 8 {
        return id.to_string();
    }
    format!("Session {}", &id[..8])
}

#[cfg(test)]
mod tests {
    use super::{
        repair_codex_home_for_provider, update_sqlite_threads_provider, SESSION_INDEX_FILE,
        STATE_DB_FILE,
    };
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

    #[test]
    fn repair_upserts_rollout_sessions_into_index() {
        let root = temp_dir("session-index");
        let rollout_dir = root.join("sessions").join("2026").join("06").join("13");
        std::fs::create_dir_all(&rollout_dir).expect("rollout dir should be created");
        std::fs::write(
            rollout_dir.join("rollout-s1.jsonl"),
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"s1\",\"thread_name\":\"Hello\",\"model_provider\":\"openai\",\"updated_at_ms\":1791849600000}}\n",
        )
        .expect("rollout should be written");

        repair_codex_home_for_provider(&root, "codex_lite_api_key").expect("repair should work");

        let index = std::fs::read_to_string(root.join(SESSION_INDEX_FILE))
            .expect("index should be written");
        assert!(index.contains("\"id\":\"s1\""));
        assert!(index.contains("\"thread_name\":\"Hello\""));
        let _ = std::fs::remove_dir_all(root);
    }
}
