use axum::{
    Extension,
    extract::{Path, Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use snafu::ensure;

use crate::{
    Error, Result,
    ctx::{Ctx, CtxValue},
    error::{ErrorInfo, ForbiddenSnafu},
    models::{MyEntryParams, MyVaultParams, OrgParams, Pref, UserParams, VaultParams},
    run::AppState,
    services::{
        auth::authenticate_token, entries::get_entry, orgs::get_org, users::get_user,
        vaults::get_vault,
    },
    web::{Action, Resource, enforce_policy, handle_error},
};
use dto::vault::VaultDto;

use super::{AUTH_TOKEN_COOKIE, THEME_COOKIE};

/// Validates auth token but does not require its validity
pub async fn auth_middleware(
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
    cookies: CookieJar,
    mut req: Request,
    next: Next,
) -> Response {
    let config = state.config.clone();
    let token = cookies
        .get(AUTH_TOKEN_COOKIE)
        .map(|c| c.value().to_string());

    let full_page = req.headers().get("HX-Request").is_none();

    // Allow ctx to be always present
    let mut ctx: Ctx = Ctx::new(None);

    if let Some(token) = token {
        // Validate token
        let result = authenticate_token(&config.api_url, &token).await;

        let _ = match result {
            Ok(actor) => {
                ctx = Ctx::new(Some(CtxValue::new(token, actor)));
            }
            Err(err) => match err {
                Error::LoginRequired => {
                    // Allow passing through
                    ()
                }
                _ => return handle_error(&state, None, &pref, ErrorInfo::from(&err), full_page),
            },
        };
    }

    req.extensions_mut().insert(ctx);
    next.run(req).await
}

pub async fn require_auth_middleware(
    Extension(ctx): Extension<Ctx>,
    req: Request,
    next: Next,
) -> Result<Response> {
    let full_page = req.headers().get("HX-Request").is_none();

    if ctx.value.is_none() {
        if full_page {
            return Ok(Redirect::to("/login").into_response());
        } else {
            return Err(Error::LoginRequired);
        }
    }

    Ok(next.run(req).await)
}

pub async fn entry_middleware(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    State(state): State<AppState>,
    Path(params): Path<MyEntryParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Entry, Action::Read)?;

    let token = ctx.token().expect("token is required");
    let entry = get_entry(
        &state.config.api_url,
        token,
        &vault.org_id,
        &vault.id,
        &params.entry_id,
    )
    .await?;

    req.extensions_mut().insert(entry);
    Ok(next.run(req).await)
}

pub async fn org_middleware(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(params): Path<OrgParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Org, Action::Read)?;

    // Regular users cannot view orgs admin pages
    ensure!(
        actor.is_system_admin(),
        ForbiddenSnafu {
            msg: "Org pages require system admin privileges"
        }
    );

    let token = ctx.token().expect("token is required");
    let config = state.config.clone();

    let org = get_org(&config.api_url, token, &params.org_id).await?;

    req.extensions_mut().insert(org);
    Ok(next.run(req).await)
}

pub async fn user_middleware(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(params): Path<UserParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::User, Action::Read)?;

    let token = ctx.token().expect("token is required");
    let config = state.config.clone();

    let user = get_user(&config.api_url, token, &params.org_id, &params.user_id).await?;

    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

pub async fn vault_middleware(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(params): Path<VaultParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Vault, Action::Read)?;

    let token = ctx.token().expect("token is required");
    let config = state.config.clone();

    let vault = get_vault(&config.api_url, token, &params.org_id, &params.vault_id).await?;

    req.extensions_mut().insert(vault);
    Ok(next.run(req).await)
}

pub async fn my_vault_middleware(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(params): Path<MyVaultParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Vault, Action::Read)?;

    let token = ctx.token().expect("token is required");
    let config = state.config.clone();

    let vault = get_vault(&config.api_url, token, &actor.org_id, &params.vault_id).await?;

    req.extensions_mut().insert(vault);
    Ok(next.run(req).await)
}

pub async fn pref_middleware(cookies: CookieJar, mut req: Request, next: Next) -> Response {
    let mut pref = Pref::new();
    let theme = cookies.get(THEME_COOKIE).map(|c| c.value().to_string());

    if let Some(theme) = theme {
        let t = theme.as_str();
        if t == "dark" || t == "light" {
            pref.theme = theme;
        }
    }

    req.extensions_mut().insert(pref);
    next.run(req).await
}
