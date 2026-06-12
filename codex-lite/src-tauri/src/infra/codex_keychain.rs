//! macOS login-keychain integration for Codex OAuth credentials.
//!
//! On macOS the Codex CLI reads OAuth credentials from the login keychain
//! (service `"Codex Auth"`), not only from `auth.json`. When switching accounts
//! we must update that keychain entry as well, otherwise Codex keeps using the
//! previous (or missing) credentials and prompts for a fresh login.
//!
//! The keychain account name mirrors the Codex CLI: `cli|<first-16-hex of
//! sha256(canonicalized codex home)>`.

use std::path::Path;

use crate::models::error::{AppError, AppResult};

const CODEX_KEYCHAIN_SERVICE: &str = "Codex Auth";

/// Writes the auth payload into the macOS login keychain for the given Codex
/// home directory. No-op on non-macOS targets.
#[cfg(target_os = "macos")]
pub fn write_codex_keychain(codex_home: &Path, auth_json: &str) -> AppResult<()> {
    use sha2::{Digest, Sha256};

    // Escape hatch for tests and headless environments so we never write the
    // real macOS login keychain outside of an actual user-driven switch.
    if std::env::var("CODEX_LITE_DISABLE_KEYCHAIN")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return Ok(());
    }

    let resolved = std::fs::canonicalize(codex_home).unwrap_or_else(|_| codex_home.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(resolved.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let digest_hex = digest
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();
    let keychain_account = format!("cli|{}", &digest_hex[..16]);

    let output = std::process::Command::new("security")
        .arg("add-generic-password")
        .arg("-U")
        .arg("-s")
        .arg(CODEX_KEYCHAIN_SERVICE)
        .arg("-a")
        .arg(&keychain_account)
        .arg("-w")
        .arg(auth_json)
        .output()
        .map_err(|err| {
            AppError::new(
                "CODEX_KEYCHAIN_WRITE_FAILED",
                format!("Failed to run security command: {}", err),
                "Check that the macOS security tool is available.",
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::new(
            "CODEX_KEYCHAIN_WRITE_FAILED",
            format!(
                "security add-generic-password failed: status={}, stderr={}",
                output.status,
                stderr.trim()
            ),
            "Switching still updated auth.json; Codex may require a fresh login.",
        ));
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn write_codex_keychain(_codex_home: &Path, _auth_json: &str) -> AppResult<()> {
    Ok(())
}
