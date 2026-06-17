use std::fs;
use std::path::PathBuf;

use crate::infra::{atomic_write, paths, storage};
use crate::models::account::{CodexAccount, CodexAccountView, CodexAuthMode};
use crate::models::error::{AppError, AppResult};
use crate::models::import::{
    BatchImportItemStatus, BatchImportPreviewItem, BatchImportSession, BatchImportSessionItem,
    ImportFailure, ImportResult, StoredBatchImportSession,
};
use crate::services::{account_service, auth_file_service};

const BATCH_IMPORT_SESSION_TTL_SECONDS: i64 = 60 * 60;

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn batch_sessions_dir() -> AppResult<PathBuf> {
    Ok(paths::app_data_dir()?.join("batch-import-sessions"))
}

fn batch_session_path(session_id: &str) -> AppResult<PathBuf> {
    if session_id.trim().is_empty()
        || session_id.contains('/')
        || session_id.contains('\\')
        || session_id.contains("..")
    {
        return Err(AppError::new(
            "BATCH_IMPORT_INVALID_SESSION_ID",
            "Batch import session id is invalid.",
            "Start a new batch import preview.",
        ));
    }

    Ok(batch_sessions_dir()?.join(format!("{}.json", session_id)))
}

fn save_batch_session(session: &StoredBatchImportSession) -> AppResult<()> {
    let path = batch_session_path(&session.session.session_id)?;
    let content = serde_json::to_vec_pretty(session).map_err(|err| {
        AppError::new(
            "BATCH_IMPORT_SESSION_SERIALIZE_FAILED",
            format!("Failed to serialize batch import session: {}", err),
            "Start a new batch import preview.",
        )
    })?;
    atomic_write::write_atomic(&path, &content)
}

fn load_batch_session(session_id: &str) -> AppResult<StoredBatchImportSession> {
    let path = batch_session_path(session_id)?;
    let content = fs::read_to_string(&path).map_err(|err| {
        AppError::new(
            "BATCH_IMPORT_SESSION_READ_FAILED",
            format!(
                "Failed to read batch import session {}: {}",
                path.display(),
                err
            ),
            "Start a new batch import preview.",
        )
    })?;
    let session: StoredBatchImportSession = serde_json::from_str(&content).map_err(|err| {
        AppError::new(
            "BATCH_IMPORT_SESSION_INVALID",
            format!(
                "Failed to parse batch import session {}: {}",
                path.display(),
                err
            ),
            "Start a new batch import preview.",
        )
    })?;

    if session.session.expires_at < now_timestamp() {
        return Err(AppError::new(
            "BATCH_IMPORT_SESSION_EXPIRED",
            "Batch import session has expired.",
            "Start a new batch import preview.",
        ));
    }

    Ok(session)
}

fn preview_from_account(
    id: String,
    source: String,
    account: &CodexAccount,
    status: BatchImportItemStatus,
    selected: bool,
    reason: Option<String>,
    quota_warning: Option<String>,
) -> BatchImportPreviewItem {
    BatchImportPreviewItem {
        id,
        source,
        status,
        selected,
        selectable: true,
        reason,
        account_id: account.account_id.clone(),
        user_id: account.user_id.clone(),
        display_name: Some(account.display_name.clone()),
        email: account.email.clone(),
        auth_mode: Some(account.auth_mode.clone()),
        plan_type: account.plan_type.clone(),
        api_base_url: account.api_base_url.clone(),
        quota: account.quota.clone(),
        quota_warning,
    }
}

fn failed_preview(id: String, source: String, error: String) -> BatchImportPreviewItem {
    BatchImportPreviewItem {
        id,
        source,
        status: BatchImportItemStatus::Failed,
        selected: false,
        selectable: false,
        reason: Some(error),
        account_id: None,
        user_id: None,
        display_name: None,
        email: None,
        auth_mode: None,
        plan_type: None,
        api_base_url: None,
        quota: None,
        quota_warning: None,
    }
}

fn read_accounts_from_file(file_path: &str) -> AppResult<Vec<CodexAccount>> {
    let path = PathBuf::from(file_path);
    let content = fs::read_to_string(&path).map_err(|err| {
        AppError::new(
            "BATCH_IMPORT_FILE_READ_FAILED",
            format!("Failed to read {}: {}", path.display(), err),
            "Choose a readable Codex auth JSON file.",
        )
    })?;
    auth_file_service::accounts_from_auth_json(&content)
}

pub fn import_from_local() -> AppResult<CodexAccountView> {
    let auth = auth_file_service::read_auth_file(&paths::default_codex_auth_file()?)?;
    let account = auth_file_service::account_from_auth(auth)?;
    account_service::upsert_account(account)
}

pub fn import_from_json(json_content: &str) -> AppResult<ImportResult> {
    let accounts = auth_file_service::accounts_from_auth_json(json_content)?;
    let mut imported = Vec::new();
    let mut failed = Vec::new();

    for account in accounts {
        match account_service::upsert_account(account) {
            Ok(view) => imported.push(view),
            Err(error) => failed.push(ImportFailure {
                source: "json".to_string(),
                error: error.message,
            }),
        }
    }

    Ok(ImportResult {
        imported,
        skipped: Vec::new(),
        failed,
    })
}

pub fn import_from_files(file_paths: Vec<String>) -> AppResult<ImportResult> {
    let mut imported = Vec::new();
    let mut failed = Vec::new();

    for file_path in file_paths {
        let path = PathBuf::from(&file_path);
        match fs::read_to_string(&path)
            .map_err(|err| err.to_string())
            .and_then(|content| {
                auth_file_service::accounts_from_auth_json(&content).map_err(|err| err.message)
            }) {
            Ok(accounts) => {
                for account in accounts {
                    match account_service::upsert_account(account) {
                        Ok(view) => imported.push(view),
                        Err(err) => failed.push(ImportFailure {
                            source: file_path.clone(),
                            error: err.message,
                        }),
                    }
                }
            }
            Err(error) => failed.push(ImportFailure {
                source: file_path,
                error,
            }),
        }
    }

    Ok(ImportResult {
        imported,
        skipped: Vec::new(),
        failed,
    })
}

pub fn start_batch_import_from_files(
    file_paths: Vec<String>,
    check_quota: bool,
) -> AppResult<BatchImportSession> {
    if file_paths.is_empty() {
        return Err(AppError::new(
            "BATCH_IMPORT_EMPTY_FILES",
            "Batch import requires at least one file.",
            "Choose one or more Codex auth JSON files.",
        ));
    }

    let existing_ids = storage::load_accounts_file()?
        .accounts
        .into_iter()
        .map(|account| account.id)
        .collect::<std::collections::HashSet<_>>();
    let mut seen_ids = std::collections::HashSet::new();
    let mut stored_items = Vec::new();

    for (index, file_path) in file_paths.into_iter().enumerate() {
        let item_id = uuid::Uuid::new_v4().to_string();
        let source = file_path.clone();
        if source.trim().is_empty() {
            return Err(AppError::new(
                "BATCH_IMPORT_INVALID_SOURCE",
                format!("Batch import file path at index {} is empty.", index),
                "Choose valid Codex auth JSON files.",
            ));
        }

        match read_accounts_from_file(&file_path) {
            Ok(accounts) => {
                for (account_index, account) in accounts.into_iter().enumerate() {
                    let is_existing =
                        existing_ids.contains(&account.id) || seen_ids.contains(&account.id);
                    let status = if is_existing {
                        BatchImportItemStatus::Existing
                    } else {
                        BatchImportItemStatus::Importable
                    };
                    let selected = status == BatchImportItemStatus::Importable;
                    let reason = if is_existing {
                        Some("Account already exists or appears earlier in this batch.".to_string())
                    } else {
                        None
                    };
                    let quota_warning = if check_quota && account.auth_mode == CodexAuthMode::OAuth
                    {
                        Some(
                            "Quota check is deferred until after import in this version."
                                .to_string(),
                        )
                    } else {
                        None
                    };
                    seen_ids.insert(account.id.clone());
                    let preview_source = if account_index == 0 {
                        source.clone()
                    } else {
                        format!("{}#{}", source, account_index + 1)
                    };
                    stored_items.push(BatchImportSessionItem {
                        preview: preview_from_account(
                            uuid::Uuid::new_v4().to_string(),
                            preview_source,
                            &account,
                            status,
                            selected,
                            reason,
                            quota_warning,
                        ),
                        account: Some(account),
                    });
                }
            }
            Err(error) => {
                stored_items.push(BatchImportSessionItem {
                    preview: failed_preview(
                        item_id,
                        source,
                        format!("{}: {}", error.code, error.message),
                    ),
                    account: None,
                });
            }
        }
    }

    let created_at = now_timestamp();
    let session = BatchImportSession {
        session_id: uuid::Uuid::new_v4().to_string(),
        created_at,
        expires_at: created_at + BATCH_IMPORT_SESSION_TTL_SECONDS,
        check_quota,
        items: stored_items
            .iter()
            .map(|item| item.preview.clone())
            .collect::<Vec<_>>(),
    };
    let stored_session = StoredBatchImportSession {
        session: session.clone(),
        items: stored_items,
    };
    save_batch_session(&stored_session)?;

    Ok(session)
}

pub fn confirm_batch_import(session_id: String, item_ids: Vec<String>) -> AppResult<ImportResult> {
    let selected_ids = item_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let stored_session = load_batch_session(&session_id)?;
    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for selected_id in &selected_ids {
        if !stored_session
            .items
            .iter()
            .any(|item| item.preview.id == *selected_id)
        {
            return Err(AppError::new(
                "BATCH_IMPORT_UNKNOWN_ITEM",
                format!(
                    "Batch import session {} does not contain item {}.",
                    session_id, selected_id
                ),
                "Refresh the preview and select accounts again.",
            ));
        }
    }

    for item in stored_session.items {
        if !selected_ids.contains(&item.preview.id) {
            if let Some(account) = item.account {
                skipped.push(account.to_view(false));
            }
            continue;
        }

        if !item.preview.selectable || item.preview.status == BatchImportItemStatus::Failed {
            failed.push(ImportFailure {
                source: item.preview.source,
                error: item
                    .preview
                    .reason
                    .unwrap_or_else(|| "Item is not importable.".to_string()),
            });
            continue;
        }

        match item.account {
            Some(account) => match account_service::upsert_account(account) {
                Ok(view) => imported.push(view),
                Err(error) => failed.push(ImportFailure {
                    source: item.preview.source,
                    error: error.message,
                }),
            },
            None => failed.push(ImportFailure {
                source: item.preview.source,
                error: "Preview item has no account payload.".to_string(),
            }),
        }
    }

    Ok(ImportResult {
        imported,
        skipped,
        failed,
    })
}

pub fn add_with_token(
    id_token: String,
    access_token: String,
    refresh_token: Option<String>,
) -> AppResult<crate::models::account::CodexAccountView> {
    let auth = crate::models::auth::CodexAuthFile {
        auth_mode: Some("oauth".to_string()),
        openai_api_key: None,
        base_url: None,
        tokens: Some(crate::models::auth::CodexAuthTokens {
            id_token,
            access_token,
            refresh_token,
            account_id: None,
        }),
        last_refresh: None,
    };
    account_service::upsert_account(auth_file_service::account_from_auth(auth)?)
}

pub fn add_with_api_key(
    api_key: String,
    api_base_url: Option<String>,
    display_name: Option<String>,
) -> AppResult<crate::models::account::CodexAccountView> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "CODEX_API_KEY_EMPTY",
            "API key cannot be empty.",
            "Paste a valid API key.",
        ));
    }
    let now = chrono::Utc::now().timestamp();
    let mut account = CodexAccount {
        id: auth_file_service::stable_id(trimmed),
        display_name: display_name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "Codex API Key".to_string()),
        email: None,
        auth_mode: CodexAuthMode::ApiKey,
        bound_oauth_account_id: None,
        account_id: None,
        user_id: None,
        plan_type: Some("API_KEY".to_string()),
        subscription_active_until: None,
        token_bundle: None,
        api_key: Some(trimmed.to_string()),
        api_base_url,
        quota: None,
        quota_error: None,
        tags: Vec::new(),
        note: None,
        created_at: now,
        updated_at: now,
        last_used_at: None,
    };
    account.display_name = account.display_name.trim().to_string();
    account_service::upsert_account(account)
}

#[cfg(test)]
mod tests {
    use super::{
        add_with_token, confirm_batch_import, import_from_files, import_from_json,
        start_batch_import_from_files,
    };
    use crate::infra::storage;
    use crate::models::import::BatchImportItemStatus;
    use crate::test_support::TestEnv;

    fn copy_fixture(env: &TestEnv, fixture_name: &str, file_name: &str) -> String {
        let path = env.root.join(file_name);
        std::fs::copy(TestEnv::fixture_path(fixture_name), &path)
            .expect("fixture should copy to temp file");
        path.display().to_string()
    }

    #[test]
    fn import_from_json_upserts_duplicate_account() {
        let _env = TestEnv::new("import-duplicate");
        let content = TestEnv::fixture_content("oauth.json");

        let first = import_from_json(&content).expect("first import should succeed");
        let second = import_from_json(&content).expect("duplicate import should update");
        let stored = storage::load_accounts_file().expect("accounts should load");

        assert_eq!(first.imported.len(), 1);
        assert_eq!(second.imported.len(), 1);
        assert_eq!(stored.accounts.len(), 1);
        assert_eq!(
            stored.accounts[0].email.as_deref(),
            Some("fixture.oauth@example.test")
        );
    }

    #[test]
    fn import_from_files_keeps_successes_when_one_file_fails() {
        let env = TestEnv::new("import-files-partial");
        let valid_path = copy_fixture(&env, "oauth.json", "oauth.json");
        let invalid_path = copy_fixture(&env, "invalid-empty.json", "invalid-empty.json");

        let result = import_from_files(vec![valid_path.clone(), invalid_path.clone()])
            .expect("import should finish");

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].source, invalid_path);
        assert!(!result.failed[0].error.trim().is_empty());
    }

    #[test]
    fn batch_preview_marks_existing_and_failed_items_unselected() {
        let env = TestEnv::new("batch-preview");
        let api_key_path = copy_fixture(&env, "api-key.json", "api-key.json");
        let oauth_path = copy_fixture(&env, "oauth.json", "oauth.json");
        let invalid_path = copy_fixture(&env, "invalid-empty.json", "invalid-empty.json");

        import_from_files(vec![api_key_path.clone()])
            .expect("existing account should import first");
        let session = start_batch_import_from_files(
            vec![
                api_key_path.clone(),
                oauth_path.clone(),
                invalid_path.clone(),
            ],
            false,
        )
        .expect("batch preview should succeed");

        let existing = session
            .items
            .iter()
            .find(|item| item.source == api_key_path)
            .expect("existing item should be present");
        let failed = session
            .items
            .iter()
            .find(|item| item.source == invalid_path)
            .expect("failed item should be present");
        let importable = session
            .items
            .iter()
            .find(|item| item.source == oauth_path)
            .expect("importable item should be present");

        assert_eq!(existing.status, BatchImportItemStatus::Existing);
        assert!(!existing.selected);
        assert!(existing.selectable);
        assert_eq!(failed.status, BatchImportItemStatus::Failed);
        assert!(!failed.selected);
        assert!(!failed.selectable);
        assert_eq!(importable.status, BatchImportItemStatus::Importable);
        assert!(importable.selected);
        assert!(importable.selectable);
    }

    #[test]
    fn confirm_batch_import_imports_only_selected_item_ids() {
        let env = TestEnv::new("batch-confirm");
        let api_key_path = copy_fixture(&env, "api-key.json", "api-key.json");
        let oauth_path = copy_fixture(&env, "oauth.json", "oauth.json");

        let session = start_batch_import_from_files(vec![api_key_path, oauth_path.clone()], false)
            .expect("batch preview should succeed");
        let selected_oauth_id = session
            .items
            .iter()
            .find(|item| item.source == oauth_path)
            .expect("OAuth item should be present")
            .id
            .clone();

        let result = confirm_batch_import(session.session_id, vec![selected_oauth_id])
            .expect("batch confirm should succeed");
        let stored = storage::load_accounts_file().expect("accounts should load");

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(stored.accounts.len(), 1);
        assert_eq!(
            stored.accounts[0].email.as_deref(),
            Some("fixture.oauth@example.test")
        );
    }

    #[test]
    fn add_with_token_rejects_missing_id_token() {
        let _env = TestEnv::new("token-missing-id-token");

        let error = add_with_token("".to_string(), "fixture-access".to_string(), None)
            .expect_err("missing ID token should fail");

        assert_eq!(error.code, "CODEX_TOKEN_INVALID");
    }
}
