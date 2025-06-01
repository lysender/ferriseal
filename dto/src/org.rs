use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgDto {
    pub id: String,
    pub name: String,
    pub admin: bool,
    pub created_at: i64,
}
