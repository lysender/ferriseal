use axum::{
    Router, middleware,
    routing::{any, get, post},
};

use super::{
    handler::{
        authenticate_handler, change_password_handler, create_entry_handler, create_org_handler,
        create_user_handler, create_vault_handler, delete_entry_handler, delete_org_handler,
        delete_user_handler, delete_vault_handler, get_entry_handler, get_org_handler,
        get_user_handler, get_vault_handler, health_live_handler, health_ready_handler,
        home_handler, list_entries_handler, list_orgs_handler, list_users_handler,
        list_vaults_handler, not_found_handler, profile_handler, reset_user_password_handler,
        update_entry_handler, update_org_handler, update_user_roles_handler,
        update_user_status_handler, user_authz_handler, user_permissions_handler,
    },
    middleware::{
        auth_middleware, entry_middleware, org_middleware, prevent_admin_org_middleware,
        require_auth_middleware, user_middleware, vault_middleware,
    },
};
use crate::state::AppState;

pub fn all_routes(state: AppState) -> Router {
    Router::new()
        .merge(public_routes(state.clone()))
        .merge(private_routes(state.clone()))
        .fallback(any(not_found_handler))
        .with_state(state)
}

fn public_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(home_handler))
        .route("/health/liveness", get(health_live_handler))
        .route("/health/readiness", get(health_ready_handler))
        .route("/auth/token", post(authenticate_handler))
        .with_state(state)
}

fn private_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/orgs", orgss_routes(state.clone()))
        .nest("/user", user_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

fn orgss_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_orgs_handler).post(create_org_handler))
        .nest("/{org_id}", inner_org_routes(state.clone()))
        .with_state(state)
}

pub fn user_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(profile_handler))
        .route("/permissions", get(user_permissions_handler))
        .route("/authz", get(user_authz_handler))
        .route("/change_password", post(change_password_handler))
        .with_state(state)
}

fn inner_org_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_org_handler)
                .patch(update_org_handler)
                .delete(delete_org_handler),
        )
        .nest("/users", org_users_routes(state.clone()))
        .nest("/vaults", org_vaults_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            org_middleware,
        ))
        .with_state(state)
}

fn org_users_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_users_handler).post(create_user_handler))
        .nest("/{user_id}", inner_user_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            prevent_admin_org_middleware,
        ))
        .with_state(state)
}

fn inner_user_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_user_handler)
                .patch(delete_vault_handler)
                .delete(delete_user_handler),
        )
        .route("/update_status", post(update_user_status_handler))
        .route("/update_roles", post(update_user_roles_handler))
        .route("/reset_password", post(reset_user_password_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            user_middleware,
        ))
        .with_state(state)
}

fn org_vaults_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_vaults_handler).post(create_vault_handler))
        .nest("/{vault_id}", inner_vault_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            prevent_admin_org_middleware,
        ))
        .with_state(state)
}

fn inner_vault_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_vault_handler).delete(delete_vault_handler))
        .nest("/entries", entry_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            vault_middleware,
        ))
        .with_state(state)
}

fn entry_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_entries_handler).post(create_entry_handler))
        .nest("/{entry_id}", inner_entry_routes(state.clone()))
        .with_state(state)
}

fn inner_entry_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_entry_handler)
                .patch(update_entry_handler)
                .delete(delete_entry_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            entry_middleware,
        ))
        .with_state(state)
}
