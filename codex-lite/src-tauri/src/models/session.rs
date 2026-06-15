use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionView {
    pub id: String,
    pub title: String,
    pub project: String,
    pub cwd: String,
    pub provider: String,
    pub target_provider: String,
    pub visible: bool,
    pub archived: bool,
    pub updated_at: Option<i64>,
    pub created_at: Option<i64>,
    pub rollout_path: Option<String>,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMutationResult {
    pub updated_count: usize,
    pub deleted_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionRepairSummary {
    pub repaired: bool,
    pub instance_count: usize,
    pub rollout_file_count: usize,
    pub sqlite_row_count: usize,
    pub index_entry_count: usize,
    pub backup_path: Option<String>,
    pub target_provider: Option<String>,
}
