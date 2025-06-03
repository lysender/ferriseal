use axum::{
    Extension,
    body::Body,
    extract::{Path, Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use snafu::{OptionExt, ResultExt, ensure};

use crate::{
    Result,
    auth::authenticate_token,
    error::{
        BadRequestSnafu, DbSnafu, ForbiddenSnafu, InsufficientAuthScopeSnafu,
        InvalidAuthTokenSnafu, NotFoundSnafu,
    },
    state::AppState,
};
use dto::{actor::Actor, entry::EntryDto, org::OrgDto, role::Permission, user::UserDto};
use vault::utils::valid_id;

use super::params::{EntryParams, OrgParams, UserParams, VaultParams};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    // Middleware to extract actor information from the request
    // Do not enforce authentication here, just extract the actor information
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    // Start with an empty actor
    let mut actor: Actor = Actor::empty();

    if let Some(auth_header) = auth_header {
        // At this point, authentication must be verified
        ensure!(auth_header.starts_with("Bearer "), InvalidAuthTokenSnafu);
        let token = auth_header.replace("Bearer ", "");

        actor = authenticate_token(&state, &token).await?;
    }

    // Forward to the next middleware/handler passing the actor information
    request.extensions_mut().insert(actor);

    let response = next.run(request).await;
    Ok(response)
}

pub async fn require_auth_middleware(
    actor: Extension<Actor>,
    request: Request,
    next: Next,
) -> Result<Response<Body>> {
    ensure!(actor.has_auth_scope(), InsufficientAuthScopeSnafu);

    Ok(next.run(request).await)
}

pub async fn org_middleware(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(params): Path<OrgParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgsView];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    ensure!(
        valid_id(&params.org_id),
        BadRequestSnafu {
            msg: "Invalid org id"
        }
    );

    // Ensure regular orgs can only view their own org
    if !actor.is_system_admin() {
        ensure!(
            actor.org_id.as_str() == params.org_id.as_str(),
            NotFoundSnafu {
                msg: "Org not found"
            }
        )
    }

    let org = state.db.orgs.get(&params.org_id).await.context(DbSnafu)?;
    let org = org.context(NotFoundSnafu {
        msg: "Org not found",
    })?;

    // Forward to the next middleware/handler passing the org information
    request.extensions_mut().insert(org);
    let response = next.run(request).await;
    Ok(response)
}

/// Org admins should not allow managing users and vaults
pub async fn prevent_admin_org_middleware(
    Extension(org): Extension<OrgDto>,
    request: Request,
    next: Next,
) -> Result<Response<Body>> {
    ensure!(
        org.admin,
        ForbiddenSnafu {
            msg: "Admin orgs does not allow managing users and vaults"
        }
    );

    Ok(next.run(request).await)
}

pub async fn vault_middleware(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(params): Path<VaultParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    ensure!(
        actor.has_vault_scope(),
        ForbiddenSnafu {
            msg: "Insufficient vault scope"
        }
    );

    let permissions = vec![Permission::VaultsList, Permission::VaultsView];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    ensure!(
        valid_id(&params.vault_id),
        BadRequestSnafu {
            msg: "Invalid vault id"
        }
    );

    let vault = state
        .db
        .vaults
        .get(&params.vault_id)
        .await
        .context(DbSnafu)?;

    let vault = vault.context(NotFoundSnafu {
        msg: "Vault not found",
    })?;

    if !actor.is_system_admin() {
        ensure!(
            &vault.org_id == &actor.org_id,
            NotFoundSnafu {
                msg: "Vault not found"
            }
        );
    }

    // Forward to the next middleware/handler passing the vault information
    request.extensions_mut().insert(vault);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn user_middleware(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(params): Path<UserParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::UsersList, Permission::UsersView];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    ensure!(
        valid_id(&params.user_id),
        BadRequestSnafu {
            msg: "Invalid user id"
        }
    );

    let user = state.db.users.get(&params.user_id).await.context(DbSnafu)?;
    let user = user.context(NotFoundSnafu {
        msg: "User not found",
    })?;

    if !actor.is_system_admin() {
        ensure!(
            &user.org_id == &actor.org_id,
            NotFoundSnafu {
                msg: "User not found"
            }
        );
    }

    let user: UserDto = user.into();

    // Forward to the next middleware/handler passing the user information
    request.extensions_mut().insert(user);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn entry_middleware(
    state: State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(params): Path<EntryParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    ensure!(
        actor.has_vault_scope(),
        ForbiddenSnafu {
            msg: "Insufficient vault scope"
        }
    );

    let permissions = vec![Permission::EntriesList, Permission::EntriesView];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let id = params.entry_id.clone();
    let entry_res = state.db.entries.get(&id).await.context(DbSnafu)?;

    let entry = entry_res.context(NotFoundSnafu {
        msg: "Entry not found",
    })?;

    ensure!(
        &entry.vault_id == &params.vault_id,
        NotFoundSnafu {
            msg: "Entry not found"
        }
    );

    let dto: EntryDto = entry.into();

    // Forward to the next middleware/handler passing the entry information
    request.extensions_mut().insert(dto);
    let response = next.run(request).await;
    Ok(response)
}
