use std::fs;
use std::path::Path;

use crate::infra::atomic_write;
use crate::models::account::{CodexAccount, CodexAuthMode};
use crate::models::error::{AppError, AppResult};

const ACTIVE_START: &str = "# >>> codex-lite active provider start";
const ACTIVE_END: &str = "# <<< codex-lite active provider end";
const PROVIDER_START: &str = "# >>> codex-lite api provider start";
const PROVIDER_END: &str = "# <<< codex-lite api provider end";
pub const API_PROVIDER_ID: &str = "codex_lite_api_key";
pub const DEFAULT_PROVIDER_ID: &str = "openai";

pub fn account_target_provider(account: &CodexAccount) -> String {
    match account.auth_mode {
        CodexAuthMode::ApiKey => account
            .api_base_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(normalize_base_url)
            .filter(|value| !is_openai_base_url(value))
            .map(|_| API_PROVIDER_ID.to_string())
            .unwrap_or_else(|| DEFAULT_PROVIDER_ID.to_string()),
        CodexAuthMode::OAuth => DEFAULT_PROVIDER_ID.to_string(),
    }
}

pub fn apply_account_config(account: &CodexAccount, config_path: &Path) -> AppResult<()> {
    let content = read_optional_config(config_path)?;
    let next = match account.auth_mode {
        CodexAuthMode::ApiKey => {
            let base_url = account
                .api_base_url
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(normalize_base_url);

            match base_url {
                Some(value) if !is_openai_base_url(&value) => {
                    apply_api_provider(&content, account, &value)
                }
                _ => clear_active_provider(&content),
            }
        }
        CodexAuthMode::OAuth => clear_active_provider(&content),
    };

    atomic_write::write_atomic(config_path, next.as_bytes())
}

pub fn read_active_provider(config_path: &Path) -> AppResult<String> {
    let content = read_optional_config(config_path)?;
    let (prelude, _) = split_prelude(&content);
    let provider = prelude
        .lines()
        .find_map(|line| {
            let (key, value) = line.trim_start().split_once('=')?;
            if key.trim() == "model_provider" {
                parse_toml_string(value)
            } else {
                None
            }
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PROVIDER_ID.to_string());

    Ok(provider)
}

fn read_optional_config(config_path: &Path) -> AppResult<String> {
    match fs::read_to_string(config_path) {
        Ok(content) => Ok(content),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(AppError::new(
            "CODEX_CONFIG_READ_FAILED",
            format!("Failed to read {}: {}", config_path.display(), err),
            "Check Codex config file permissions.",
        )),
    }
}

fn apply_api_provider(content: &str, account: &CodexAccount, base_url: &str) -> String {
    let previous_model_provider_from_active = read_previous_model_provider(content);
    let content_without_active = remove_marked_block(content, ACTIVE_START, ACTIVE_END);
    let content_without_provider =
        remove_marked_block(&content_without_active, PROVIDER_START, PROVIDER_END);
    let cleaned_content = remove_orphan_marker_lines(&content_without_provider);
    let (prelude, rest) = split_prelude(&cleaned_content);
    let (prelude_without_model_provider, top_level_model_provider) =
        remove_top_level_model_provider(prelude);
    let previous_model_provider = previous_model_provider_from_active.or(top_level_model_provider);

    let active_block = build_active_provider_block(previous_model_provider.as_deref());
    let provider_block = build_api_provider_block(account, base_url);
    format!(
        "{}{}{}{}",
        active_block,
        prelude_without_model_provider,
        ensure_leading_newline(rest),
        provider_block
    )
}

fn clear_active_provider(content: &str) -> String {
    let previous_model_provider = read_previous_model_provider(content);
    let without_active = remove_marked_block(content, ACTIVE_START, ACTIVE_END);
    let without_provider = remove_marked_block(&without_active, PROVIDER_START, PROVIDER_END);
    let cleaned_content = remove_orphan_marker_lines(&without_provider);
    let restored = match previous_model_provider {
        Some(value) => {
            let (prelude, rest) = split_prelude(&cleaned_content);
            let (prelude_without_model_provider, _) = remove_top_level_model_provider(prelude);
            format!(
                "{}{}{}",
                format!("model_provider = {}\n", toml_string(&value)),
                prelude_without_model_provider,
                ensure_leading_newline(rest)
            )
        }
        None => cleaned_content,
    };

    restored
}

fn build_active_provider_block(previous_model_provider: Option<&str>) -> String {
    let previous_line = previous_model_provider
        .filter(|value| !value.is_empty() && *value != API_PROVIDER_ID)
        .map(|value| format!("# previous_model_provider = {}\n", toml_string(value)))
        .unwrap_or_default();

    format!(
        "{ACTIVE_START}\n{previous_line}model_provider = {}\n{ACTIVE_END}\n",
        toml_string(API_PROVIDER_ID)
    )
}

fn build_api_provider_block(account: &CodexAccount, base_url: &str) -> String {
    let name = account.display_name.trim();
    let provider_name = if name.is_empty() {
        "Codex Lite API Key"
    } else {
        name
    };

    format!(
        "\n{PROVIDER_START}\n[model_providers.{API_PROVIDER_ID}]\nname = {}\nbase_url = {}\nwire_api = \"responses\"\nsupports_websockets = false\nrequires_openai_auth = true\n{PROVIDER_END}\n",
        toml_string(provider_name),
        toml_string(base_url)
    )
}

fn normalize_base_url(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

fn is_openai_base_url(value: &str) -> bool {
    let normalized = value.trim().trim_end_matches('/');
    normalized == "https://api.openai.com/v1" || normalized == "https://api.openai.com"
}

fn split_prelude(content: &str) -> (&str, &str) {
    match content.find("\n[") {
        Some(index) => content.split_at(index + 1),
        None if content.trim_start().starts_with('[') => ("", content),
        None => (content, ""),
    }
}

fn remove_top_level_model_provider(prelude: &str) -> (String, Option<String>) {
    let mut previous = None;
    let lines = prelude
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("model_provider") {
                if previous.is_none() {
                    previous = parse_toml_string_assignment(trimmed);
                }
                false
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    (ensure_trailing_newline(&lines), previous)
}

fn read_previous_model_provider(content: &str) -> Option<String> {
    let start = content.find(ACTIVE_START)?;
    let after_start = &content[start..];
    let end = after_start.find(ACTIVE_END)?;
    after_start[..end]
        .lines()
        .find_map(|line| {
            line.trim_start()
                .strip_prefix("# previous_model_provider = ")
        })
        .and_then(parse_toml_string)
}

fn remove_marked_block(content: &str, start_marker: &str, end_marker: &str) -> String {
    let mut next = content.to_string();
    while let Some(start) = next.find(start_marker) {
        let after_start = &next[start..];
        let Some(relative_end) = after_start.find(end_marker) else {
            break;
        };
        let end = start + relative_end + end_marker.len();
        let end_with_newline = if next[end..].starts_with('\n') {
            end + 1
        } else {
            end
        };

        next.replace_range(start..end_with_newline, "");
    }

    next
}

fn remove_orphan_marker_lines(content: &str) -> String {
    let lines = content
        .lines()
        .filter(|line| {
            !matches!(
                line.trim(),
                ACTIVE_START | ACTIVE_END | PROVIDER_START | PROVIDER_END
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    ensure_trailing_newline(&lines)
}

fn parse_toml_string_assignment(line: &str) -> Option<String> {
    let (_, value) = line.split_once('=')?;
    parse_toml_string(value.trim())
}

fn parse_toml_string(value: &str) -> Option<String> {
    let value = value.trim();
    let unquoted = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(unquoted.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn toml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn ensure_trailing_newline(value: &str) -> String {
    if value.is_empty() || value.ends_with('\n') {
        value.to_string()
    } else {
        format!("{value}\n")
    }
}

fn ensure_leading_newline(value: &str) -> String {
    if value.is_empty() || value.starts_with('\n') {
        value.to_string()
    } else {
        format!("\n{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_api_provider, clear_active_provider};
    use crate::models::account::{CodexAccount, CodexAuthMode};

    fn api_account() -> CodexAccount {
        CodexAccount {
            id: "account".to_string(),
            display_name: "FunCode".to_string(),
            email: None,
            auth_mode: CodexAuthMode::ApiKey,
            bound_oauth_account_id: None,
            account_id: None,
            user_id: None,
            plan_type: Some("API_KEY".to_string()),
            token_bundle: None,
            api_key: Some("sk-test".to_string()),
            api_base_url: Some("https://api.yaso11.tech".to_string()),
            quota: None,
            quota_error: None,
            tags: Vec::new(),
            note: None,
            created_at: 0,
            updated_at: 0,
            last_used_at: None,
        }
    }

    #[test]
    fn apply_api_provider_sets_top_level_provider_and_normalized_base_url() {
        let output = apply_api_provider(
            "model = \"gpt-5.5\"\n[features]\n",
            &api_account(),
            "https://api.yaso11.tech/v1",
        );

        assert!(output.contains("model_provider = \"codex_lite_api_key\""));
        assert!(output.contains("[model_providers.codex_lite_api_key]"));
        assert!(output.contains("base_url = \"https://api.yaso11.tech/v1\""));
        assert!(output.contains("requires_openai_auth = true"));
    }

    #[test]
    fn clear_active_provider_restores_previous_model_provider() {
        let input = "# >>> codex-lite active provider start\n# previous_model_provider = \"aimami\"\nmodel_provider = \"codex_lite_api_key\"\n# <<< codex-lite active provider end\nmodel = \"gpt-5.5\"\n";

        let output = clear_active_provider(input);

        assert!(output.contains("model_provider = \"aimami\""));
        assert!(!output.contains("codex_lite_api_key"));
    }

    #[test]
    fn apply_api_provider_cleans_duplicate_marker_blocks() {
        let input = "# >>> codex-lite active provider start\n# <<< codex-lite active provider end\n# >>> codex-lite active provider start\n# <<< codex-lite active provider end\n# <<< codex-lite active provider end\nmodel = \"gpt-5.5\"\n";

        let output = apply_api_provider(input, &api_account(), "https://api.yaso11.tech/v1");

        assert_eq!(
            output
                .matches("# >>> codex-lite active provider start")
                .count(),
            1
        );
        assert_eq!(
            output
                .matches("# <<< codex-lite active provider end")
                .count(),
            1
        );
        assert_eq!(
            output
                .matches("model_provider = \"codex_lite_api_key\"")
                .count(),
            1
        );
    }

    #[test]
    fn clear_active_provider_removes_api_provider_block() {
        let input = "# >>> codex-lite active provider start\n# previous_model_provider = \"openai\"\nmodel_provider = \"codex_lite_api_key\"\n# <<< codex-lite active provider end\nmodel = \"gpt-5.5\"\n# >>> codex-lite api provider start\n[model_providers.codex_lite_api_key]\nbase_url = \"https://api.yaso11.tech/v1\"\n# <<< codex-lite api provider end\n";

        let output = clear_active_provider(input);

        assert!(output.contains("model_provider = \"openai\""));
        assert!(!output.contains("[model_providers.codex_lite_api_key]"));
        assert!(!output.contains("codex-lite api provider"));
    }
}
