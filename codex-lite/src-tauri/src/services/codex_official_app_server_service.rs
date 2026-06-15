use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{json, Value as JsonValue};

use crate::models::error::{AppError, AppResult};

const CODEX_APP_SERVER_EXECUTABLE: &str = "/Applications/Codex.app/Contents/Resources/codex";
const CODEX_APP_SERVER_EXECUTABLE_ENV: &str = "CODEX_APP_SERVER_EXECUTABLE";
const APP_SERVER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);

pub fn rebuild_thread_metadata(codex_home: &Path) -> AppResult<()> {
    let executable = official_app_server_executable()?;
    let mut child = build_app_server_command(&executable, codex_home)
        .spawn()
        .map_err(|err| {
            AppError::new(
                "CODEX_APP_SERVER_START_FAILED",
                format!(
                    "Failed to start Codex app-server {} with CODEX_HOME={}: {}",
                    executable.display(),
                    codex_home.display(),
                    err
                ),
                "Install the official Codex app and try again.",
            )
        })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        AppError::new(
            "CODEX_APP_SERVER_STDOUT_UNAVAILABLE",
            "Codex app-server stdout is unavailable.",
            "Try again.",
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        AppError::new(
            "CODEX_APP_SERVER_STDERR_UNAVAILABLE",
            "Codex app-server stderr is unavailable.",
            "Try again.",
        )
    })?;
    let mut stdin = child.stdin.take().ok_or_else(|| {
        AppError::new(
            "CODEX_APP_SERVER_STDIN_UNAVAILABLE",
            "Codex app-server stdin is unavailable.",
            "Try again.",
        )
    })?;

    let (sender, receiver) = mpsc::channel::<String>();
    let stdout_reader = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = sender.send(line);
        }
    });
    let stderr_reader = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            eprintln!("[Codex app-server][stderr] {line}");
        }
    });

    let result = (|| {
        send_request(
            &mut stdin,
            json!({
                "method": "initialize",
                "id": 1,
                "params": {
                    "clientInfo": {
                        "name": "codex-lite",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "capabilities": null,
                },
            }),
        )?;
        wait_for_response(&receiver, 1)?;

        send_request(
            &mut stdin,
            json!({
                "method": "thread/list",
                "id": 2,
                "params": {
                    "cursor": null,
                    "limit": 1,
                    "sortKey": "updated_at",
                    "sortDirection": "desc",
                    "modelProviders": null,
                    "sourceKinds": [],
                    "archived": false,
                },
            }),
        )?;
        wait_for_response(&receiver, 2)
    })();

    finish_child(&mut child);
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    result
}

fn official_app_server_executable() -> AppResult<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(executable) = std::env::var_os(CODEX_APP_SERVER_EXECUTABLE_ENV) {
        if !executable.is_empty() {
            candidates.push(PathBuf::from(executable));
        }
    }
    candidates.push(PathBuf::from(CODEX_APP_SERVER_EXECUTABLE));

    for executable in &candidates {
        if executable.exists() {
            return Ok(executable.clone());
        }
    }

    let searched_paths = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(AppError::new(
        "CODEX_APP_SERVER_NOT_FOUND",
        format!("Codex app-server executable was not found: {searched_paths}"),
        "Install the official Codex app and try again.",
    ))
}

fn build_app_server_command(executable: &Path, codex_home: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .args(["app-server", "--listen", "stdio://"])
        .env("CODEX_HOME", codex_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn send_request(stdin: &mut impl Write, request: JsonValue) -> AppResult<()> {
    let line = serde_json::to_string(&request).map_err(|err| {
        AppError::new(
            "CODEX_APP_SERVER_REQUEST_SERIALIZE_FAILED",
            format!("Failed to serialize Codex app-server request: {err}"),
            "Try again.",
        )
    })?;
    stdin
        .write_all(line.as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|err| {
            AppError::new(
                "CODEX_APP_SERVER_REQUEST_WRITE_FAILED",
                format!("Failed to write Codex app-server request: {err}"),
                "Try again.",
            )
        })
}

fn wait_for_response(receiver: &mpsc::Receiver<String>, request_id: i64) -> AppResult<()> {
    loop {
        let line = receiver
            .recv_timeout(APP_SERVER_RESPONSE_TIMEOUT)
            .map_err(|_| {
                AppError::new(
                    "CODEX_APP_SERVER_RESPONSE_TIMEOUT",
                    format!("Timed out waiting for Codex app-server response id={request_id}."),
                    "Try again.",
                )
            })?;
        let Ok(value) = serde_json::from_str::<JsonValue>(&line) else {
            continue;
        };
        if value.get("id").and_then(JsonValue::as_i64) != Some(request_id) {
            continue;
        }
        if let Some(error) = value.get("error") {
            return Err(AppError::new(
                "CODEX_APP_SERVER_RESPONSE_ERROR",
                format!("Codex app-server returned error for id={request_id}: {error}"),
                "Try again.",
            ));
        }
        if value.get("result").is_some() {
            return Ok(());
        }
        return Err(AppError::new(
            "CODEX_APP_SERVER_RESPONSE_INVALID",
            format!("Codex app-server response is missing result for id={request_id}: {value}"),
            "Try again.",
        ));
    }
}

fn finish_child(child: &mut Child) {
    if matches!(child.try_wait(), Ok(Some(_))) {
        return;
    }
    let _ = child.kill();
    let _ = child.wait();
}
