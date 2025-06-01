use db::entry::{Entry, EntryPayload};
use dto::entry::EntryDto;
use snafu::{ResultExt, ensure};

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};

use crate::state::AppState;
use dto::vault::VaultDto;
use vault::utils::generate_id;

const MAX_ENTRIES: i32 = 10000;

pub async fn create_entry(state: AppState, vault: &VaultDto, data: &EntryPayload) -> Result<Entry> {
    // Limit the number of entries per vault
    let count = state
        .db
        .entries
        .count_by_vault(&vault.id)
        .await
        .context(DbSnafu)?;

    ensure!(
        count < MAX_ENTRIES as i64,
        ValidationSnafu {
            msg: format!("Vault already reached the maximum entries: {}", MAX_ENTRIES),
        }
    );

    let today = chrono::Utc::now().timestamp();
    let entry_dto = EntryDto {
        id: generate_id(),
        vault_id: vault.id.clone(),
        label: data.label.clone(),
        cipher_username: data.cipher_username.clone(),
        cipher_password: data.cipher_password.clone(),
        cipher_notes: data.cipher_notes.clone(),
        cipher_extra_notes: data.cipher_extra_notes.clone(),
        status: "active".to_string(),
        created_at: today.clone(),
        updated_at: today,
    };

    state
        .db
        .entries
        .create(entry_dto.clone())
        .await
        .context(DbSnafu)
}
