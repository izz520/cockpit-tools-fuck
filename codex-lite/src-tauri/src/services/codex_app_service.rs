use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::models::error::{AppError, AppResult};

const CODEX_BUNDLE_ID: &str = "com.openai.codex";
const CODEX_MAIN_EXECUTABLE: &str = "/Codex.app/Contents/MacOS/Codex";
const FORCE_QUIT_WAIT_ATTEMPTS: usize = 20;
const OPEN_WAIT_ATTEMPTS: usize = 30;
const QUIT_WAIT_INTERVAL: Duration = Duration::from_millis(150);

#[cfg(target_os = "macos")]
pub fn quit_codex_for_switch() -> AppResult<()> {
    force_quit_codex_main_process()?;

    if wait_until_codex_exits(FORCE_QUIT_WAIT_ATTEMPTS)? {
        return Ok(());
    }

    Err(AppError::new(
        "CODEX_APP_QUIT_TIMEOUT",
        "Codex did not quit before switching accounts.",
        "Close Codex and try switching again.",
    ))
}

#[cfg(target_os = "macos")]
fn wait_until_codex_exits(attempts: usize) -> AppResult<bool> {
    for _ in 0..attempts {
        if codex_main_process_ids()?.is_empty() {
            return Ok(true);
        }
        thread::sleep(QUIT_WAIT_INTERVAL);
    }

    Ok(false)
}

#[cfg(target_os = "macos")]
fn force_quit_codex_main_process() -> AppResult<()> {
    let process_ids = codex_main_process_ids()?;
    if process_ids.is_empty() {
        return Ok(());
    }

    for process_id in process_ids {
        run_command(
            Command::new("kill").args(["-9", process_id.as_str()]),
            "CODEX_APP_FORCE_QUIT_FAILED",
            "Failed to force quit Codex before switching accounts.",
            "Close Codex and try switching again.",
        )?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn codex_main_process_ids() -> AppResult<Vec<String>> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,args="])
        .output()
        .map_err(|err| {
            AppError::new(
                "CODEX_APP_STATUS_FAILED",
                format!("Failed to list Codex processes: {}", err),
                "Close Codex and try switching again.",
            )
        })?;
    if !output.status.success() {
        return Err(command_error(
            "CODEX_APP_STATUS_FAILED",
            "Failed to list Codex processes.",
            "Close Codex and try switching again.",
            &output,
        ));
    }

    let current_process_id = std::process::id().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(parse_codex_main_process_id)
        .filter(|process_id| *process_id != current_process_id)
        .collect())
}

#[cfg(target_os = "macos")]
fn parse_codex_main_process_id(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let (process_id, command) = trimmed.split_once(' ')?;
    let command = command.trim_start();
    if command.ends_with(CODEX_MAIN_EXECUTABLE)
        || command.contains(&format!("{CODEX_MAIN_EXECUTABLE} "))
    {
        return Some(process_id.to_string());
    }
    None
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
    )?;

    for _ in 0..OPEN_WAIT_ATTEMPTS {
        if !codex_main_process_ids()?.is_empty() || is_codex_running()? {
            return Ok(());
        }
        thread::sleep(QUIT_WAIT_INTERVAL);
    }

    Err(AppError::new(
        "CODEX_APP_OPEN_TIMEOUT",
        "Codex did not open after switching accounts.",
        "Open Codex manually.",
    ))
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

    #[cfg(target_os = "macos")]
    #[test]
    fn parses_codex_main_process_with_or_without_args() {
        assert_eq!(
            super::parse_codex_main_process_id(
                "79496 /Applications/Codex.app/Contents/MacOS/Codex"
            )
            .as_deref(),
            Some("79496")
        );
        assert_eq!(
            super::parse_codex_main_process_id(
                "65675 /Applications/Codex.app/Contents/MacOS/Codex --remote-debugging-port=53067"
            )
            .as_deref(),
            Some("65675")
        );
        assert!(super::parse_codex_main_process_id(
            "73525 /Applications/Codex Lite.app/Contents/MacOS/codex-lite"
        )
        .is_none());
    }
}
