use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub schema_version: String,
    pub codex_home_path: Option<String>,
    pub auth_file_path: Option<String>,
    pub theme: String,
    pub quota_refresh_on_start: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: "1.0.0".to_string(),
            codex_home_path: None,
            auth_file_path: None,
            theme: "system".to_string(),
            quota_refresh_on_start: false,
        }
    }
}
