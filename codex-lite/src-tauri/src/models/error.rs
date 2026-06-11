use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub action: String,
    pub details: Option<serde_json::Value>,
    pub retryable: bool,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            action: action.into(),
            details: None,
            retryable: false,
        }
    }

    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn app_error_round_trips_with_camel_case_fields() {
        let error = AppError {
            code: "CODEX_FIXTURE".to_string(),
            message: "Fixture message.".to_string(),
            action: "Try again.".to_string(),
            details: Some(serde_json::json!({ "requestId": "req_fixture" })),
            retryable: true,
        };

        let json = serde_json::to_string(&error).expect("error should serialize");
        let parsed: AppError = serde_json::from_str(&json).expect("error should deserialize");

        assert!(json.contains("requestId"));
        assert_eq!(parsed.code, "CODEX_FIXTURE");
        assert_eq!(
            parsed.details,
            Some(serde_json::json!({ "requestId": "req_fixture" }))
        );
        assert!(parsed.retryable);
    }
}
