use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, MaxVaultsReachedSnafu, ValidationSnafu};
use crate::state::AppState;
use db::vault::NewVault;
use dto::vault::VaultDto;
use vault::validators::flatten_errors;

const MAX_VAULTS_PER_ORG: i32 = 10;

pub async fn create_vault(state: &AppState, org_id: &str, data: &NewVault) -> Result<VaultDto> {
    let valid_res = data.validate();
    ensure!(
        valid_res.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&valid_res.unwrap_err()),
        }
    );

    // Limit the number of vaults per org
    let count = state
        .db
        .vaults
        .count_by_org(org_id)
        .await
        .context(DbSnafu)?;

    ensure!(count < MAX_VAULTS_PER_ORG as i64, MaxVaultsReachedSnafu);

    state.db.vaults.create(org_id, data).await.context(DbSnafu)
}

pub async fn delete_vault(state: &AppState, id: &str) -> Result<()> {
    // Do not delete if there are still entries inside
    let entries_count = state.db.entries.count_by_vault(id).await.context(DbSnafu)?;
    ensure!(
        entries_count == 0,
        ValidationSnafu {
            msg: "Cannot delete vault with entries inside".to_string(),
        }
    );

    state.db.vaults.delete(id).await.context(DbSnafu)
}
