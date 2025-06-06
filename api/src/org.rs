use snafu::{OptionExt, ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, MaxOrgsReachedSnafu, ValidationSnafu};
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
    let count = state.db.orgs.count().await.context(DbSnafu)?;
    ensure!(count < MAX_ORGS as i64, MaxOrgsReachedSnafu,);

    state.db.orgs.create(data, admin).await.context(DbSnafu)
}

pub async fn update_org(state: &AppState, id: &str, data: &UpdateOrg) -> Result<bool> {
    let valid_res = data.validate();
    ensure!(
        valid_res.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&valid_res.unwrap_err()),
        }
    );

    state.db.orgs.update(id, data).await.context(DbSnafu)
}

pub async fn delete_org(state: &AppState, id: &str) -> Result<()> {
    let org = state.db.orgs.get(id).await.context(DbSnafu)?;
    let org = org.context(ValidationSnafu {
        msg: "Org not found".to_string(),
    })?;

    ensure!(
        !org.admin,
        ValidationSnafu {
            msg: "Cannot delete admin org".to_string(),
        }
    );

    let vault_count = state.db.vaults.count_by_org(id).await.context(DbSnafu)?;
    ensure!(
        vault_count == 0,
        ValidationSnafu {
            msg: "Org still has vaults".to_string(),
        }
    );

    let users_count = state.db.users.count_by_org(id).await.context(DbSnafu)?;
    ensure!(
        users_count == 0,
        ValidationSnafu {
            msg: "Org still has users".to_string(),
        }
    );

    state.db.orgs.delete(id).await.context(DbSnafu)
}
