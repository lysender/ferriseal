use axum::extract::State;
use axum::handler::HandlerWithoutStateExt;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, get_service, post};
use axum::{Extension, Router, middleware};
use reqwest::StatusCode;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};
use tracing::error;

use crate::ctx::Ctx;
use crate::error::ErrorInfo;
use crate::models::Pref;
use crate::run::AppState;
use crate::web::{error_handler, index_handler, login_handler, logout_handler, post_login_handler};

use super::entries::{
    edit_entry_controls_handler, edit_entry_handler, entry_page_handler, get_delete_entry_handler,
    new_entry_handler, post_delete_entry_handler, post_edit_entry_handler, post_new_entry_handler,
    search_entries_handler,
};
use super::middleware::{
    auth_middleware, entry_middleware, my_vault_middleware, org_middleware, pref_middleware,
    require_auth_middleware, user_middleware, vault_middleware,
};
use super::my_vault::my_vault_page_handler;
use super::orgs::{
    delete_org_handler, edit_org_controls_handler, edit_org_handler, new_org_handler,
    org_page_handler, orgs_handler, orgs_listing_handler, post_delete_org_handler,
    post_edit_org_handler, post_new_org_handler,
};
use super::profile::{
    change_user_password_handler, post_change_password_handler, profile_controls_handler,
    profile_page_handler,
};
use super::users::{
    delete_user_handler, new_user_handler, post_delete_user_handler, post_new_user_handler,
    post_reset_password_handler, post_update_user_role_handler, post_update_user_status_handler,
    reset_user_password_handler, update_user_role_handler, update_user_status_handler,
    user_controls_handler, user_page_handler, users_handler,
};
use super::vaults::{
    delete_vault_handler, new_vault_handler, post_delete_vault_handler, post_new_vault_handler,
    vault_controls_handler, vault_page_handler, vaults_handler,
};
use super::{dark_theme_handler, handle_error, light_theme_handler};

pub fn all_routes(state: AppState, frontend_entry: &PathBuf) -> Router {
    Router::new()
        .merge(public_routes(state.clone()))
        .merge(private_routes(state.clone()))
        .merge(assets_routes(frontend_entry))
        .fallback(any(error_handler).with_state(state))
}

pub fn assets_routes(entry: &PathBuf) -> Router {
    let target_entry = entry.join("public");
    Router::new()
        .route(
            "/manifest.json",
            get_service(ServeFile::new(target_entry.join("manifest.json"))),
        )
        .route(
            "/favicon.ico",
            get_service(ServeFile::new(target_entry.join("favicon.ico"))),
        )
        .nest_service(
            "/assets",
            get_service(
                ServeDir::new(target_entry.join("assets"))
                    .not_found_service(file_not_found.into_service()),
            ),
        )
}

async fn file_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "File not found")
}

pub fn private_routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/prefs/theme/light", post(light_theme_handler))
        .route("/prefs/theme/dark", post(dark_theme_handler))
        .route("/profile", get(profile_page_handler))
        .route("/profile/profile_controls", get(profile_controls_handler))
        .route(
            "/profile/change_password",
            get(change_user_password_handler).post(post_change_password_handler),
        )
        .nest("/orgs", org_routes(state.clone()))
        .nest("/vaults/{vault_id}", my_vault_routes(state.clone()))
        .layer(middleware::map_response_with_state(
            state.clone(),
            response_mapper,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn(pref_middleware))
        .with_state(state)
}

fn org_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(orgs_handler))
        .route("/listing", get(orgs_listing_handler))
        .route("/new", get(new_org_handler).post(post_new_org_handler))
        .nest("/{org_id}", org_inner_routes(state.clone()))
        .with_state(state)
}

fn org_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(org_page_handler))
        .route("/edit_controls", get(edit_org_controls_handler))
        .route("/edit", get(edit_org_handler).post(post_edit_org_handler))
        .route(
            "/delete",
            get(delete_org_handler).post(post_delete_org_handler),
        )
        .nest("/users", users_routes(state.clone()))
        .nest("/vaults", vaults_routes(state.clone()))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            org_middleware,
        ))
        .with_state(state)
}

fn users_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(users_handler))
        .route("/new", get(new_user_handler).post(post_new_user_handler))
        .nest("/{user_id}", user_inner_routes(state.clone()))
        .with_state(state)
}

fn user_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(user_page_handler))
        .route("/edit_controls", get(user_controls_handler))
        .route(
            "/update_status",
            get(update_user_status_handler).post(post_update_user_status_handler),
        )
        .route(
            "/update_role",
            get(update_user_role_handler).post(post_update_user_role_handler),
        )
        .route(
            "/reset_password",
            get(reset_user_password_handler).post(post_reset_password_handler),
        )
        .route(
            "/delete",
            get(delete_user_handler).post(post_delete_user_handler),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            user_middleware,
        ))
        .with_state(state)
}

fn vaults_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(vaults_handler))
        .route("/new", get(new_vault_handler).post(post_new_vault_handler))
        .nest("/{vault_id}", vault_inner_routes(state.clone()))
        .with_state(state)
}

fn vault_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(vault_page_handler))
        .route("/edit_controls", get(vault_controls_handler))
        .route(
            "/delete",
            get(delete_vault_handler).post(post_delete_vault_handler),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            vault_middleware,
        ))
        .with_state(state)
}

fn my_vault_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(my_vault_page_handler))
        .route("/search_entries", get(search_entries_handler))
        .route(
            "/new_entry",
            get(new_entry_handler).post(post_new_entry_handler),
        )
        .nest("/entries/{entry_id}", my_entry_inner_routes(state.clone()))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            my_vault_middleware,
        ))
        .with_state(state)
}

fn my_entry_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(entry_page_handler))
        .route("/edit_controls", get(edit_entry_controls_handler))
        .route(
            "/edit",
            get(edit_entry_handler).post(post_edit_entry_handler),
        )
        .route(
            "/delete",
            get(get_delete_entry_handler).post(post_delete_entry_handler),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            entry_middleware,
        ))
        .with_state(state)
}

pub fn public_routes(state: AppState) -> Router {
    Router::new()
        .route("/login", get(login_handler).post(post_login_handler))
        .route("/logout", post(logout_handler))
        .layer(middleware::map_response_with_state(
            state.clone(),
            response_mapper,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn(pref_middleware))
        .with_state(state)
}

async fn response_mapper(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    headers: HeaderMap,
    res: Response,
) -> Response {
    let error = res.extensions().get::<ErrorInfo>();
    if let Some(e) = error {
        if e.status_code.is_server_error() {
            // Build the error response
            error!("{}", e.message);
            if let Some(bt) = &e.backtrace {
                error!("{}", bt);
            }
        }

        let full_page = headers.get("HX-Request").is_none();
        let actor = ctx.actor().map(|t| t.clone());
        return handle_error(&state, actor, &pref, e.clone(), full_page);
    }
    res
}
