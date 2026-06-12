use crate::models::error::AppResult;
use crate::models::session::{CodexSessionView, SessionMutationResult};
use crate::services::codex_session_visibility_service;

#[tauri::command]
pub fn list_codex_sessions() -> AppResult<Vec<CodexSessionView>> {
    codex_session_visibility_service::list_default_codex_sessions()
}

#[tauri::command]
pub fn restore_codex_sessions_visibility(
    session_ids: Vec<String>,
) -> AppResult<SessionMutationResult> {
    codex_session_visibility_service::restore_default_codex_sessions_visibility(session_ids)
}

#[tauri::command]
pub fn delete_codex_sessions(session_ids: Vec<String>) -> AppResult<SessionMutationResult> {
    codex_session_visibility_service::delete_default_codex_sessions(session_ids)
}
