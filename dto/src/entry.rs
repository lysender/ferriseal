use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryDto {
    pub id: String,
    pub vault_id: String,
    pub label: String,
    pub cipher_username: Option<String>,
    pub cipher_password: Option<String>,
    pub cipher_notes: Option<String>,
    pub cipher_extra_notes: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}
