use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryDto {
    pub id: String,
    pub vault_id: String,
    pub label: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub notes: Option<String>,
    pub extra_notes: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}
