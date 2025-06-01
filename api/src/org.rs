use snafu::ensure;
use validator::Validate;

use crate::Result;
use crate::error::{
    DbInteractSnafu, DbPoolSnafu, DbQuerySnafu, MaxorgsReachedSnafu, ValidationSnafu,
};
use crate::state::AppState;
use db::org::{NewOrg, Org, UpdateOrg};
use vault::validators::flatten_errors;

// Can't have too many orgs
const MAX_ORGS: i32 = 10;

pub async fn create_org(state: &AppState, data: &NewOrg, admin: bool) -> Result<Org> {
    let valid_res = data.validate();
    ensure!(
        valid_res.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&valid_res.unwrap_err()),
        }
    );

    // Limit the number of orgs because we are poor!
    let count = state.db.orgs.count().await?;
    ensure!(count < MAX_ORGS as i64, MaxorgsReachedSnafu,);

    state.db.orgs.create(data, admin).await
}

pub async fn update_org(state: &AppState, id: &str, data: &UpdateOrg) -> Result<bool> {
    let valid_res = data.validate();
    ensure!(
        valid_res.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&valid_res.unwrap_err()),
        }
    );

    // We can't tell whether we are setting default bucket to null or skipping it
    // Will just use a separate function for that
    if let Some(bucket_id) = data.default_bucket_id.clone() {
        if let Some(bid) = bucket_id {
            let bucket = state.db.buckets.get(&bid).await?;
            ensure!(
                bucket.is_some(),
                ValidationSnafu {
                    msg: "Default bucket not found".to_string(),
                }
            );
        }
    }

    state.db.orgs.update(id, data).await
}

pub async fn delete_org(state: &AppState, id: &str) -> Result<()> {
    let Some(client) = state.db.orgs.get(id).await? else {
        return ValidationSnafu {
            msg: "Org not found".to_string(),
        }
        .fail();
    };

    ensure!(
        !client.admin,
        ValidationSnafu {
            msg: "Cannot delete admin org".to_string(),
        }
    );

    let vault_count = state.db.vaults.count_by_org(id).await?;
    ensure!(
        vault_count == 0,
        ValidationSnafu {
            msg: "Org still has vaults".to_string(),
        }
    );

    let users_count = state.db.users.count_by_org(id).await?;
    ensure!(
        users_count == 0,
        ValidationSnafu {
            msg: "Org still has users".to_string(),
        }
    );

    state.db.orgs.delete(id).await
}
