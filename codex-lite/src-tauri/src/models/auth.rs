use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAuthFile {
    #[serde(default)]
    pub auth_mode: Option<String>,
    #[serde(rename = "OPENAI_API_KEY", default)]
    pub openai_api_key: Option<serde_json::Value>,
    #[serde(default, alias = "api_base_url", alias = "apiBaseUrl")]
    pub base_url: Option<String>,
    #[serde(default)]
    pub tokens: Option<CodexAuthTokens>,
    #[serde(default)]
    pub last_refresh: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAuthTokens {
    #[serde(alias = "id_token")]
    pub id_token: String,
    #[serde(alias = "access_token")]
    pub access_token: String,
    #[serde(default, alias = "refresh_token")]
    pub refresh_token: Option<String>,
    #[serde(default, alias = "account_id")]
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
