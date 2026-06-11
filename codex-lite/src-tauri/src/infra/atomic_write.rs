use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::models::error::{AppError, AppResult};

pub fn write_atomic(path: &Path, content: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            "ATOMIC_WRITE_INVALID_PATH",
            "Target path has no parent directory.",
            "Choose a valid file path.",
        )
    })?;
    fs::create_dir_all(parent).map_err(|err| {
        AppError::new(
            "ATOMIC_WRITE_CREATE_DIR_FAILED",
            format!("Failed to create directory {}: {}", parent.display(), err),
            "Check directory permissions.",
        )
    })?;

    let temp_path = path.with_extension("tmp");
    {
        let mut file = File::create(&temp_path).map_err(|err| {
            AppError::new(
                "ATOMIC_WRITE_CREATE_FAILED",
                format!(
                    "Failed to create temp file {}: {}",
                    temp_path.display(),
                    err
                ),
                "Check file permissions.",
            )
        })?;
        file.write_all(content).map_err(|err| {
            AppError::new(
                "ATOMIC_WRITE_FAILED",
                format!("Failed to write temp file {}: {}", temp_path.display(), err),
                "Check disk space and file permissions.",
            )
        })?;
        file.sync_all().map_err(|err| {
            AppError::new(
                "ATOMIC_WRITE_SYNC_FAILED",
                format!("Failed to sync temp file {}: {}", temp_path.display(), err),
                "Check disk health.",
            )
        })?;
    }

    fs::rename(&temp_path, path).map_err(|err| {
        AppError::new(
            "ATOMIC_WRITE_RENAME_FAILED",
            format!("Failed to replace {}: {}", path.display(), err),
            "Check file permissions and try again.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::write_atomic;

    #[test]
    fn writes_complete_content_to_target_file() {
        let root =
            std::env::temp_dir().join(format!("codex-lite-atomic-write-{}", uuid::Uuid::new_v4()));
        let target = root.join("nested").join("accounts.json");

        write_atomic(&target, br#"{"schemaVersion":"1.0.0"}"#)
            .expect("atomic write should succeed");

        let content = std::fs::read_to_string(&target).expect("target file should be readable");
        assert_eq!(content, r#"{"schemaVersion":"1.0.0"}"#);
        assert!(
            !target.with_extension("tmp").exists(),
            "temporary file should be renamed away"
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
