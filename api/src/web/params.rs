use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub bucket_id: String,
    pub dir_id: Option<String>,
    pub file_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrgParams {
    pub org_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserParams {
    #[allow(dead_code)]
    pub org_id: String,

    pub user_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct VaultParams {
    pub org_id: String,
    pub vault_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EntryParams {
    pub org_id: String,
    pub vault_id: String,
    pub entry_id: String,
}
