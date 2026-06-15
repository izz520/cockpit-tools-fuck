use std::fs;
use std::path::Path;

use crate::infra::atomic_write;
use crate::models::account::{CodexAccount, CodexAuthMode};
use crate::models::error::{AppError, AppResult};

const ACTIVE_START: &str = "# >>> codex-lite active provider start";
const ACTIVE_END: &str = "# <<< codex-lite active provider end";
const PROVIDER_START: &str = "# >>> codex-lite api provider start";
const PROVIDER_END: &str = "# <<< codex-lite api provider end";
pub const API_PROVIDER_ID: &str = "codex_local_access";
pub const DEFAULT_PROVIDER_ID: &str = "openai";
const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const SUPPORTS_WEBSOCKETS_KEY: &str = "supports_websockets";

pub fn account_target_provider(account: &CodexAccount) -> String {
    match account.auth_mode {
        CodexAuthMode::ApiKey => API_PROVIDER_ID.to_string(),
        CodexAuthMode::OAuth => DEFAULT_PROVIDER_ID.to_string(),
    }
}

pub fn apply_account_config_with_provider_base_url(
    account: &CodexAccount,
    config_path: &Path,
    provider_base_url_override: Option<&str>,
) -> AppResult<()> {
    let content = read_optional_config(config_path)?;
    let next = match account.auth_mode {
        CodexAuthMode::ApiKey => {
            let base_url = account
                .api_base_url
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(normalize_base_url)
                .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.to_string());
            let provider_base_url = provider_base_url_override
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(normalize_base_url)
                .unwrap_or(base_url);
            let api_key = account
                .api_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::new(
                        "CODEX_ACCOUNT_MISSING_CREDENTIALS",
                        "Selected API Key account has no API key.",
                        "Re-import this account before switching.",
                    )
                })?;
            apply_api_provider(&content, account, &provider_base_url, api_key)
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

fn apply_api_provider(
    content: &str,
    account: &CodexAccount,
    base_url: &str,
    api_key: &str,
) -> String {
    let previous_model_provider_from_active = read_previous_model_provider(content);
    let content_without_active = remove_marked_block(content, ACTIVE_START, ACTIVE_END);
    let content_without_provider =
        remove_marked_block(&content_without_active, PROVIDER_START, PROVIDER_END);
    let content_without_api_table =
        remove_model_provider_table(&content_without_provider, API_PROVIDER_ID);
    let cleaned_content = remove_orphan_marker_lines(&content_without_api_table);
    let (prelude, rest) = split_prelude(&cleaned_content);
    let (prelude_without_model_provider, top_level_model_provider) =
        remove_top_level_model_provider(prelude);
    let previous_model_provider = previous_model_provider_from_active.or(top_level_model_provider);

    let active_block = build_active_provider_block(previous_model_provider.as_deref());
    let provider_block = build_api_provider_block(account, base_url, api_key);
    format!(
        "{}{}{}{}",
        active_block,
        prelude_without_model_provider,
        ensure_leading_newline(rest),
        provider_block
    )
}

fn clear_active_provider(content: &str) -> String {
    let without_active = remove_marked_block(content, ACTIVE_START, ACTIVE_END);
    let without_provider = remove_marked_block(&without_active, PROVIDER_START, PROVIDER_END);
    let without_api_table = remove_model_provider_table(&without_provider, API_PROVIDER_ID);
    let cleaned_content = remove_orphan_marker_lines(&without_api_table);
    let (prelude, rest) = split_prelude(&cleaned_content);
    let (prelude_without_model_provider, _) = remove_top_level_model_provider(prelude);
    let oauth_prelude = set_top_level_bool(
        &prelude_without_model_provider,
        SUPPORTS_WEBSOCKETS_KEY,
        false,
    );
    format!("{}{}", oauth_prelude, ensure_leading_newline(rest))
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

fn build_api_provider_block(account: &CodexAccount, base_url: &str, api_key: &str) -> String {
    let name = account.display_name.trim();
    let provider_name = if name.is_empty() {
        "Codex Lite API Key"
    } else {
        name
    };

    format!(
        "\n{PROVIDER_START}\n[model_providers.{API_PROVIDER_ID}]\nname = {}\nbase_url = {}\nwire_api = \"responses\"\nsupports_websockets = false\nrequires_openai_auth = true\nexperimental_bearer_token = {}\n{PROVIDER_END}\n",
        toml_string(provider_name),
        toml_string(base_url),
        toml_string(api_key)
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

fn set_top_level_bool(prelude: &str, key: &str, value: bool) -> String {
    let assignment = format!("{key} = {value}");
    let lines = prelude
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed
                .split_once('=')
                .is_some_and(|(candidate, _)| candidate.trim() == key)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut next = ensure_trailing_newline(&lines);
    next.push_str(&assignment);
    next.push('\n');
    next
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

fn remove_model_provider_table(content: &str, provider_id: &str) -> String {
    let mut kept = Vec::new();
    let mut skipping = false;

    for line in content.lines() {
        if let Some(name) = table_header_name(line) {
            skipping = is_model_provider_table(&name, provider_id);
            if skipping {
                continue;
            }
        }

        if !skipping {
            kept.push(line);
        }
    }

    ensure_trailing_newline(&kept.join("\n"))
}

fn table_header_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    let inner = trimmed.strip_prefix('[')?.strip_suffix(']')?.trim();
    if inner.starts_with('[') || inner.ends_with(']') {
        return None;
    }
    Some(inner.to_string())
}

fn is_model_provider_table(name: &str, provider_id: &str) -> bool {
    name == format!("model_providers.{provider_id}")
        || name == format!("model_providers.\"{provider_id}\"")
        || name == format!("model_providers.'{provider_id}'")
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
            "sk-test",
        );

        assert!(output.contains("model_provider = \"codex_local_access\""));
        assert!(output.contains("[model_providers.codex_local_access]"));
        assert!(output.contains("base_url = \"https://api.yaso11.tech/v1\""));
        assert!(output.contains("requires_openai_auth = true"));
        assert!(output.contains("experimental_bearer_token = \"sk-test\""));
    }

    #[test]
    fn clear_active_provider_removes_previous_model_provider() {
        let input = "# >>> codex-lite active provider start\n# previous_model_provider = \"aimami\"\nmodel_provider = \"codex_local_access\"\n# <<< codex-lite active provider end\nmodel = \"gpt-5.5\"\n";

        let output = clear_active_provider(input);

        assert!(!output.contains("model_provider = \"aimami\""));
        assert!(!output.contains("codex_local_access"));
        assert!(output.contains("supports_websockets = false"));
    }

    #[test]
    fn clear_active_provider_sets_oauth_supports_websockets_false_once() {
        let input =
            "supports_websockets = true\nmodel = \"gpt-5.5\"\n[features]\nweb_search = true\n";

        let output = clear_active_provider(input);

        assert!(!output.contains("supports_websockets = true"));
        assert_eq!(output.matches("supports_websockets = false").count(), 1);
        assert!(output.contains("model = \"gpt-5.5\""));
        assert!(output.contains("[features]"));
    }

    #[test]
    fn apply_api_provider_cleans_duplicate_marker_blocks() {
        let input = "# >>> codex-lite active provider start\n# <<< codex-lite active provider end\n# >>> codex-lite active provider start\n# <<< codex-lite active provider end\n# <<< codex-lite active provider end\nmodel = \"gpt-5.5\"\n";

        let output = apply_api_provider(
            input,
            &api_account(),
            "https://api.yaso11.tech/v1",
            "sk-test",
        );

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
                .matches("model_provider = \"codex_local_access\"")
                .count(),
            1
        );
    }

    #[test]
    fn clear_active_provider_removes_api_provider_block() {
        let input = "# >>> codex-lite active provider start\n# previous_model_provider = \"openai\"\nmodel_provider = \"codex_local_access\"\n# <<< codex-lite active provider end\nmodel = \"gpt-5.5\"\n# >>> codex-lite api provider start\n[model_providers.codex_local_access]\nbase_url = \"https://api.yaso11.tech/v1\"\n# <<< codex-lite api provider end\n";

        let output = clear_active_provider(input);

        assert!(!output.contains("model_provider = \"openai\""));
        assert!(!output.contains("[model_providers.codex_local_access]"));
        assert!(!output.contains("codex-lite api provider"));
    }

    #[test]
    fn clear_active_provider_removes_unmarked_api_provider_table() {
        let input = "model_provider = \"codex_local_access\"\nmodel = \"gpt-5.5\"\n[model_providers.codex_local_access]\nname = \"Old API\"\nbase_url = \"https://api.old.test/v1\"\n[features]\nweb_search = true\n";

        let output = clear_active_provider(input);

        assert!(!output.contains("model_provider = \"codex_local_access\""));
        assert!(!output.contains("[model_providers.codex_local_access]"));
        assert!(!output.contains("https://api.old.test/v1"));
        assert!(output.contains("[features]"));
        assert!(output.contains("web_search = true"));
    }

    #[test]
    fn apply_api_provider_replaces_unmarked_api_provider_table() {
        let input = "model_provider = \"relay\"\n[model_providers.codex_local_access]\nname = \"Old API\"\nbase_url = \"https://api.old.test/v1\"\n[model_providers.other]\nname = \"Other\"\n";

        let output = apply_api_provider(
            input,
            &api_account(),
            "https://api.yaso11.tech/v1",
            "sk-test",
        );

        assert_eq!(
            output
                .matches("[model_providers.codex_local_access]")
                .count(),
            1
        );
        assert!(!output.contains("https://api.old.test/v1"));
        assert!(output.contains("[model_providers.other]"));
    }
}
