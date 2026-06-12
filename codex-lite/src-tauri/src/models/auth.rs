use serde::{Deserialize, Serialize};

/// Mirrors the real Codex `~/.codex/auth.json` format so files written here can be
/// read back by the Codex CLI. Field names serialize as snake_case (plus the
/// uppercase `OPENAI_API_KEY`); camelCase aliases are accepted on read so that
/// auth files previously written by older builds still import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexAuthFile {
    #[serde(default, alias = "authMode", skip_serializing_if = "Option::is_none")]
    pub auth_mode: Option<String>,
    #[serde(rename = "OPENAI_API_KEY", default, skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<serde_json::Value>,
    #[serde(
        default,
        alias = "api_base_url",
        alias = "apiBaseUrl",
        alias = "baseUrl",
        skip_serializing_if = "Option::is_none"
    )]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<CodexAuthTokens>,
    #[serde(default, alias = "lastRefresh", skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexAuthTokens {
    #[serde(alias = "idToken")]
    pub id_token: String,
    #[serde(alias = "accessToken")]
    pub access_token: String,
    #[serde(default, alias = "refreshToken", skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, alias = "accountId", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtPayload {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default, rename = "https://api.openai.com/auth")]
    pub auth_data: Option<JwtAuthData>,
    #[serde(default, rename = "https://api.openai.com/profile")]
    pub profile_data: Option<JwtProfileData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtAuthData {
    #[serde(default)]
    pub chatgpt_user_id: Option<String>,
    #[serde(default)]
    pub chatgpt_plan_type: Option<String>,
    #[serde(default)]
    pub chatgpt_account_id: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtProfileData {
    #[serde(default)]
    pub email: Option<String>,
}
