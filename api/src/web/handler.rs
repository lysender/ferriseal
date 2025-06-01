use axum::{
    Extension,
    extract::{Json, Query, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use core::result::Result as CoreResult;
use serde::Serialize;
use snafu::{OptionExt, ResultExt, ensure};

use crate::{
    auth::authenticate,
    entry::create_entry,
    error::{DbSnafu, ErrorResponse, ForbiddenSnafu, JsonRejectionSnafu, Result, WhateverSnafu},
    health::{check_liveness, check_readiness},
    org::{create_org, delete_org, update_org},
    state::AppState,
    user::change_current_password,
    vault::{create_vault, delete_vault},
    web::response::JsonResponse,
};
use db::{
    entry::{EntryPayload, ListEntriesParams},
    org::{NewOrg, UpdateOrg},
    user::{ChangeCurrentPassword, NewUser, UpdateUserPassword, UpdateUserRoles, UpdateUserStatus},
    vault::NewVault,
};
use dto::{
    actor::{Actor, Credentials},
    entry::EntryDto,
    org::OrgDto,
    pagination::PaginatedDto,
    role::Permission,
    user::UserDto,
    vault::VaultDto,
};

#[derive(Serialize)]
pub struct AppMeta {
    pub name: String,
    pub version: String,
}

pub async fn authenticate_handler(
    State(state): State<AppState>,
    payload: CoreResult<Json<Credentials>, JsonRejection>,
) -> Result<JsonResponse> {
    let credentials = payload.context(JsonRejectionSnafu {
        msg: "Invalid credentials payload",
    })?;

    let res = authenticate(&state, &credentials).await?;
    Ok(JsonResponse::new(serde_json::to_string(&res).unwrap()))
}

pub async fn profile_handler(Extension(actor): Extension<Actor>) -> Result<JsonResponse> {
    Ok(JsonResponse::new(
        serde_json::to_string(&actor.user).unwrap(),
    ))
}

pub async fn user_permissions_handler(Extension(actor): Extension<Actor>) -> Result<JsonResponse> {
    let mut items: Vec<String> = actor.permissions.iter().map(|p| p.to_string()).collect();
    items.sort();
    Ok(JsonResponse::new(serde_json::to_string(&items).unwrap()))
}

pub async fn user_authz_handler(Extension(actor): Extension<Actor>) -> Result<JsonResponse> {
    Ok(JsonResponse::new(serde_json::to_string(&actor).unwrap()))
}

pub async fn change_password_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    payload: CoreResult<Json<ChangeCurrentPassword>, JsonRejection>,
) -> Result<JsonResponse> {
    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    let _ = change_current_password(&state, &actor.user.id, &data).await?;

    Ok(JsonResponse::with_status(
        StatusCode::NO_CONTENT,
        "".to_string(),
    ))
}

pub async fn home_handler() -> impl IntoResponse {
    Json(AppMeta {
        name: "vault-rs".to_string(),
        version: "0.1.0".to_string(),
    })
}

pub async fn not_found_handler(State(_state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            status_code: StatusCode::NOT_FOUND.as_u16(),
            message: "Not Found",
            error: "Not Found",
        }),
    )
}

pub async fn health_live_handler() -> Result<JsonResponse> {
    let health = check_liveness().await?;
    Ok(JsonResponse::new(serde_json::to_string(&health).unwrap()))
}

pub async fn health_ready_handler(State(state): State<AppState>) -> Result<JsonResponse> {
    let health = check_readiness(state.db).await?;
    let status = if health.is_healthy() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    Ok(JsonResponse::with_status(
        status,
        serde_json::to_string(&health).unwrap(),
    ))
}

pub async fn list_orgs_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::OrgsList];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let mut org_id: Option<String> = None;
    if !actor.is_system_admin() {
        org_id = Some(actor.org_id.clone());
    }
    let orgs = state.db.orgs.list(org_id).await.context(DbSnafu)?;
    let dtos: Vec<OrgDto> = orgs.into_iter().map(|x| x.into()).collect();
    Ok(JsonResponse::new(serde_json::to_string(&dtos).unwrap()))
}

pub async fn create_org_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    payload: CoreResult<Json<NewOrg>, JsonRejection>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::OrgsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    let created = create_org(&state, &data, false).await?;
    let dto: OrgDto = created.into();
    Ok(JsonResponse::new(serde_json::to_string(&dto).unwrap()))
}

pub async fn get_org_handler(Extension(client): Extension<OrgDto>) -> Result<JsonResponse> {
    Ok(JsonResponse::new(serde_json::to_string(&client).unwrap()))
}

pub async fn update_org_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
    payload: CoreResult<Json<UpdateOrg>, JsonRejection>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::OrgsEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    // No changes, just return the org
    if data.name.is_none() {
        return Ok(JsonResponse::new(serde_json::to_string(&org).unwrap()));
    }

    let updated = update_org(&state, org.id.as_str(), &data).await?;
    if !updated {
        // No changes, just return the org
        return Ok(JsonResponse::new(serde_json::to_string(&org).unwrap()));
    }

    let updated_org = state.db.orgs.get(org.id.as_str()).await.context(DbSnafu)?;
    let updated_org = updated_org.context(WhateverSnafu {
        msg: "Unable to find updated org",
    })?;

    Ok(JsonResponse::new(
        serde_json::to_string(&updated_org).unwrap(),
    ))
}

pub async fn delete_org_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::OrgsDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );
    ensure!(
        !org.admin,
        ForbiddenSnafu {
            msg: "Cannot delete admin org"
        }
    );

    let _ = delete_org(&state, &org.id).await?;

    Ok(JsonResponse::with_status(
        StatusCode::NO_CONTENT,
        "".to_string(),
    ))
}

pub async fn list_vaults_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::VaultsList];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );
    let vaults = state.db.vaults.list(&org.id).await.context(DbSnafu)?;
    Ok(JsonResponse::new(serde_json::to_string(&vaults).unwrap()))
}

pub async fn get_vault_handler(Extension(vault): Extension<VaultDto>) -> Result<JsonResponse> {
    Ok(JsonResponse::new(serde_json::to_string(&vault).unwrap()))
}

pub async fn delete_vault_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(vault): Extension<VaultDto>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::VaultsDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let _ = delete_vault(&state, vault.id.as_str()).await?;

    Ok(JsonResponse::with_status(
        StatusCode::NO_CONTENT,
        "".to_string(),
    ))
}

pub async fn create_vault_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
    payload: CoreResult<Json<NewVault>, JsonRejection>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::VaultsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    let vault = create_vault(&state, &org.id, &data).await?;

    Ok(JsonResponse::with_status(
        StatusCode::CREATED,
        serde_json::to_string(&vault).unwrap(),
    ))
}

pub async fn list_users_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::UsersList];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );
    let users = state.db.users.list(&org.id).await.context(DbSnafu)?;
    let dto: Vec<UserDto> = users.into_iter().map(|x| x.into()).collect();
    Ok(JsonResponse::new(serde_json::to_string(&dto).unwrap()))
}

pub async fn create_user_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
    payload: CoreResult<Json<NewUser>, JsonRejection>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::UsersCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    let user = state
        .db
        .users
        .create(&org.id, &data, false)
        .await
        .context(DbSnafu)?;
    let dto: UserDto = user.into();

    Ok(JsonResponse::with_status(
        StatusCode::CREATED,
        serde_json::to_string(&dto).unwrap(),
    ))
}

pub async fn get_user_handler(Extension(user): Extension<UserDto>) -> Result<JsonResponse> {
    Ok(JsonResponse::new(serde_json::to_string(&user).unwrap()))
}

pub async fn update_user_status_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(user): Extension<UserDto>,
    payload: CoreResult<Json<UpdateUserStatus>, JsonRejection>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::UsersEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Do not allow updating your own user
    ensure!(
        &actor.user.id != &user.id,
        ForbiddenSnafu {
            msg: "Updating your own user account not allowed"
        }
    );

    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    // Ideally, should not update if status do not change
    let _ = state
        .db
        .users
        .update_status(&user.id, &data)
        .await
        .context(DbSnafu)?;

    // Re-query and show
    let updated_user = state.db.users.get(&user.id).await.context(DbSnafu)?;
    let updated_user = updated_user.context(WhateverSnafu {
        msg: "Unable to re-query user information.",
    })?;
    let dto: UserDto = updated_user.into();

    Ok(JsonResponse::new(serde_json::to_string(&dto).unwrap()))
}

pub async fn update_user_roles_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(user): Extension<UserDto>,
    payload: CoreResult<Json<UpdateUserRoles>, JsonRejection>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::UsersEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Do not allow updating your own user
    ensure!(
        &actor.user.id != &user.id,
        ForbiddenSnafu {
            msg: "Updating your own user account not allowed"
        }
    );

    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    // Ideally, should not update if roles do not change
    let _ = state
        .db
        .users
        .update_roles(&user.id, &data)
        .await
        .context(DbSnafu)?;

    // Re-query and show
    let updated_user = state.db.users.get(&user.id).await.context(DbSnafu)?;
    let updated_user = updated_user.context(WhateverSnafu {
        msg: "Unable to re-query user information.",
    })?;
    let dto: UserDto = updated_user.into();

    Ok(JsonResponse::new(serde_json::to_string(&dto).unwrap()))
}

pub async fn reset_user_password_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(user): Extension<UserDto>,
    payload: CoreResult<Json<UpdateUserPassword>, JsonRejection>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::UsersEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Do not allow updating your own user
    ensure!(
        &actor.user.id != &user.id,
        ForbiddenSnafu {
            msg: "Updating your own user account not allowed"
        }
    );

    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    let _ = state
        .db
        .users
        .update_password(&user.id, &data)
        .await
        .context(DbSnafu)?;

    // Re-query and show
    let updated_user = state.db.users.get(&user.id).await.context(DbSnafu)?;
    let updated_user = updated_user.context(WhateverSnafu {
        msg: "Unable to re-query user information.",
    })?;
    let dto: UserDto = updated_user.into();

    Ok(JsonResponse::new(serde_json::to_string(&dto).unwrap()))
}

pub async fn delete_user_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(user): Extension<UserDto>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::UsersDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Do not allow deleting your own user account
    ensure!(
        &actor.user.id != &user.id,
        ForbiddenSnafu {
            msg: "Deleting your own user account not allowed"
        }
    );

    let _ = state.db.users.delete(&user.id).await.context(DbSnafu)?;

    Ok(JsonResponse::with_status(
        StatusCode::NO_CONTENT,
        "".to_string(),
    ))
}

pub async fn list_entries_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(vault): Extension<VaultDto>,
    query: Query<ListEntriesParams>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::EntriesList, Permission::EntriesView];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let entries = state
        .db
        .entries
        .list(&vault.id, &query)
        .await
        .context(DbSnafu)?;

    // Generate download urls for each files
    let items: Vec<EntryDto> = entries.data.into_iter().map(|f| f.into()).collect();

    let listing = PaginatedDto::new(
        items,
        entries.meta.page,
        entries.meta.per_page,
        entries.meta.total_records,
    );
    Ok(JsonResponse::new(serde_json::to_string(&listing).unwrap()))
}

pub async fn create_entry_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(vault): Extension<VaultDto>,
    payload: CoreResult<Json<EntryPayload>, JsonRejection>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::EntriesCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    let entry = create_entry(state, &vault, &data).await?;
    let dto: EntryDto = entry.into();

    Ok(JsonResponse::with_status(
        StatusCode::CREATED,
        serde_json::to_string(&dto).unwrap(),
    ))
}

pub async fn get_entry_handler(Extension(entry): Extension<EntryDto>) -> Result<JsonResponse> {
    Ok(JsonResponse::new(serde_json::to_string(&entry).unwrap()))
}

#[axum::debug_handler]
pub async fn update_entry_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(entry): Extension<EntryDto>,
    payload: CoreResult<Json<EntryPayload>, JsonRejection>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::EntriesEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = payload.context(JsonRejectionSnafu {
        msg: "Invalid request payload",
    })?;

    let _ = state
        .db
        .entries
        .update(&entry.id, &data)
        .await
        .context(DbSnafu)?;

    // Re-query and show
    let updated_entry = state.db.entries.get(&entry.id).await.context(DbSnafu)?;
    let updated_entry = updated_entry.context(WhateverSnafu {
        msg: "Unable to re-query entry information.",
    })?;
    let dto: EntryDto = updated_entry.into();

    Ok(JsonResponse::new(serde_json::to_string(&dto).unwrap()))
}

pub async fn delete_entry_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(entry): Extension<EntryDto>,
) -> Result<JsonResponse> {
    let permissions = vec![Permission::EntriesDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Delete record
    let _ = state.db.entries.delete(&entry.id).await.context(DbSnafu)?;

    Ok(JsonResponse::with_status(
        StatusCode::NO_CONTENT,
        "".to_string(),
    ))
}
