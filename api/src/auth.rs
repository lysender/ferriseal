use snafu::ResultExt;
use validator::Validate;

use crate::token::{create_auth_token, verify_auth_token};
use dto::actor::{Actor, ActorPayload, AuthResponse, Credentials};
use password::verify_password;
use snafu::{OptionExt, ensure};

use crate::error::{
    DbSnafu, InactiveUserSnafu, InvalidClientSnafu, InvalidPasswordSnafu, PasswordSnafu,
    UserNotFoundSnafu, ValidationSnafu,
};
use crate::{Result, state::AppState};
use vault::validators::flatten_errors;

pub async fn authenticate(state: &AppState, credentials: &Credentials) -> Result<AuthResponse> {
    let errors = credentials.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    // Validate user
    let user = state
        .db
        .users
        .find_by_username(&credentials.username)
        .await
        .context(DbSnafu)?;

    let user = user.context(InvalidPasswordSnafu)?;

    ensure!(&user.status == "active", InactiveUserSnafu);

    // Validate org
    let org = state.db.orgs.get(&user.org_id).await.context(DbSnafu)?;
    let org = org.context(InvalidClientSnafu)?;

    // Validate password
    let _ = verify_password(&credentials.password, &user.password).context(PasswordSnafu)?;

    // Generate a token
    let actor = ActorPayload {
        id: user.id.clone(),
        org_id: org.id.clone(),
        scope: "auth vault".to_string(),
    };

    let token = create_auth_token(&actor, &state.config.jwt_secret)?;
    Ok(AuthResponse {
        user: user.into(),
        token,
    })
}

pub async fn authenticate_token(state: &AppState, token: &str) -> Result<Actor> {
    let actor = verify_auth_token(token, &state.config.jwt_secret)?;

    // Validate org
    let org = state.db.orgs.get(&actor.org_id).await.context(DbSnafu)?;
    let org = org.context(InvalidClientSnafu)?;

    let user = state.db.users.get(&actor.id).await.context(DbSnafu)?;
    let user = user.context(UserNotFoundSnafu)?;
    ensure!(&user.org_id == &org.id, UserNotFoundSnafu);

    Ok(Actor::new(actor, user.into()))
}
