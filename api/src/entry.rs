use db::entry::{Entry, EntryPayload};
use dto::entry::EntryDto;

use crate::Result;
use crate::error::{
    DbInteractSnafu, DbPoolSnafu, DbQuerySnafu, ExifInfoSnafu, UploadFileSnafu, ValidationSnafu,
};

use crate::state::AppState;
use dto::vault::VaultDto;
use vault::utils::generate_id;

const MAX_PER_PAGE: i32 = 50;
const MAX_ENTRIES: i32 = 10000;

pub async fn create_entry(state: AppState, vault: &VaultDto, data: &EntryPayload) -> Result<Entry> {
    // Limit the number of entries per vault
    let count = state.db.entries.count_by_vault(&vault.id).await?;
    if count >= MAX_ENTRIES as i64 {
        return ValidationSnafu {
            msg: format!("Vault already reached the maximum entries: {}", MAX_ENTRIES),
        }
        .fail();
    }

    let today = chrono::Utc::now().timestamp();
    let entry_dto = EntryDto {
        id: generate_id(),
        vault_id: vault.id,
        label: data.label,
        cipher_username: data.cipher_username,
        cipher_password: data.cipher_password,
        cipher_notes: data.cipher_notes,
        cipher_extra_notes: data.cipher_extra_notes,
        status: "active".to_string(),
        created_at: today.clone(),
        updated_at: today,
    };

    state.db.entries.create(entry_dto.clone()).await
}
