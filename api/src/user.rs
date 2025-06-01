use snafu::{OptionExt, ResultExt, ensure};
use validator::Validate;

use crate::error::{
    DbInteractSnafu, DbPoolSnafu, DbQuerySnafu, InvalidRolesSnafu, MaxUsersReachedSnafu,
    ValidationSnafu, WhateverSnafu,
};
use crate::state::AppState;
use crate::{Error, Result};
use db::user::{ChangeCurrentPassword, UpdateUserPassword};
use password::verify_password;
use vault::validators::flatten_errors;

const MAX_USERS_PER_CLIENT: i32 = 50;

pub async fn change_current_password(
    state: &AppState,
    user_id: &str,
    data: &ChangeCurrentPassword,
) -> Result<bool> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let user = state.db.users.get(user_id).await?.context(WhateverSnafu {
        msg: "Unable to re-query user".to_string(),
    })?;

    // Validate current password
    if let Err(verify_err) = verify_password(&data.current_password, &user.password) {
        return match verify_err {
            Error::InvalidPassword => Err(Error::Validation {
                msg: "Current password is incorrect".to_string(),
            }),
            _ => Err(verify_err),
        };
    }

    let new_data = UpdateUserPassword {
        password: data.new_password.clone(),
    };

    state.db.users.update_password(user_id, &new_data).await
}
