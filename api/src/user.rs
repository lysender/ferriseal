use snafu::{OptionExt, ResultExt, ensure};
use validator::Validate;

use crate::error::{DbSnafu, PasswordSnafu, ValidationSnafu, WhateverSnafu};
use crate::state::AppState;
use crate::{Error, Result};
use db::user::{ChangeCurrentPassword, UpdateUserPassword};
use password::verify_password;
use vault::validators::flatten_errors;

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

    let user = state.db.users.get(user_id).await.context(DbSnafu)?;
    let user = user.context(WhateverSnafu {
        msg: "Unable to re-query user".to_string(),
    })?;

    // Validate current password
    let verify_res = verify_password(&data.current_password, &user.password).context(PasswordSnafu);
    if let Err(verify_err) = verify_res {
        return match verify_err {
            #[allow(unused_variables)]
            Error::Password { source, backtrace } => match source {
                password::Error::Incorrect => Err(Error::Validation {
                    msg: "Current password is incorrect".to_string(),
                }),
                other => Err(Error::Whatever {
                    msg: format!("{}", other),
                }),
            },
            _ => Err(verify_err),
        };
    }

    let new_data = UpdateUserPassword {
        password: data.new_password.clone(),
    };

    state
        .db
        .users
        .update_password(user_id, &new_data)
        .await
        .context(DbSnafu)
}
