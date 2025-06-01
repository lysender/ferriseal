use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDto {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub test_cipher: String,
    pub created_at: i64,
    pub updated_at: i64,
}
