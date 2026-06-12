use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::models::error::{AppError, AppResult};

const CODEX_BUNDLE_ID: &str = "com.openai.codex";
const QUIT_WAIT_ATTEMPTS: usize = 20;
const QUIT_WAIT_INTERVAL: Duration = Duration::from_millis(150);

#[cfg(target_os = "macos")]
pub fn quit_codex_for_switch() -> AppResult<()> {
    if !is_codex_running()? {
        return Ok(());
    }

    run_command(
        Command::new("pkill").args(["-x", "Codex"]),
        "CODEX_APP_QUIT_FAILED",
        "Failed to quit Codex before switching accounts.",
        "Close Codex and try switching again.",
    )?;

    for _ in 0..QUIT_WAIT_ATTEMPTS {
        if !is_codex_running()? {
            return Ok(());
        }
        thread::sleep(QUIT_WAIT_INTERVAL);
    }

    Err(AppError::new(
        "CODEX_APP_QUIT_TIMEOUT",
        "Codex did not quit before switching accounts.",
        "Close Codex and try switching again.",
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn quit_codex_for_switch() -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn open_codex_after_switch() -> AppResult<()> {
    run_command(
        Command::new("open").args(["-b", CODEX_BUNDLE_ID]),
        "CODEX_APP_OPEN_FAILED",
        "Failed to open Codex after switching accounts.",
        "Open Codex manually.",
    )
}

#[cfg(not(target_os = "macos"))]
pub fn open_codex_after_switch() -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn is_codex_running() -> AppResult<bool> {
    let script = format!("application id \"{CODEX_BUNDLE_ID}\" is running");
    let output = Command::new("osascript")
        .args(["-e", script.as_str()])
        .output()
        .map_err(|err| {
            AppError::new(
                "CODEX_APP_STATUS_FAILED",
                format!("Failed to check Codex running status: {}", err),
                "Close Codex and try switching again.",
            )
        })?;
    if !output.status.success() {
        return Err(command_error(
            "CODEX_APP_STATUS_FAILED",
            "Failed to check Codex running status.",
            "Close Codex and try switching again.",
            &output,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim() == "true")
}

#[cfg(target_os = "macos")]
fn run_command(command: &mut Command, code: &str, message: &str, action: &str) -> AppResult<()> {
    let output = command
        .output()
        .map_err(|err| AppError::new(code, format!("{} {}", message, err), action))?;
    if output.status.success() {
        return Ok(());
    }

    Err(command_error(code, message, action, &output))
}

#[cfg(target_os = "macos")]
fn command_error(
    code: &str,
    message: &str,
    action: &str,
    output: &std::process::Output,
) -> AppError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    AppError::new(
        code,
        format!(
            "{} status: {:?}, stdout: {}, stderr: {}",
            message,
            output.status.code(),
            stdout.trim(),
            stderr.trim()
        ),
        action,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn codex_bundle_id_targets_official_codex() {
        assert!(super::CODEX_BUNDLE_ID.contains("codex"));
    }
}
