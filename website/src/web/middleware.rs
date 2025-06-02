use axum::{
    Extension,
    extract::{Path, Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use memo::bucket::BucketDto;
use snafu::ensure;

use crate::{
    Error, Result,
    ctx::{Ctx, CtxValue},
    error::{ErrorInfo, ForbiddenSnafu},
    models::{
        BucketParams, ClientParams, MyBucketParams, MyDirParams, MyFileParams, Pref, UserParams,
    },
    run::AppState,
    services::{
        auth::authenticate_token,
        dirs::{Dir, get_dir},
        entries::get_photo,
        orgs::get_org,
        users::get_user,
        vault::get_bucket,
    },
    web::{Action, Resource, enforce_policy, handle_error},
};

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

pub async fn dir_middleware(
    Extension(ctx): Extension<Ctx>,
    Extension(bucket): Extension<BucketDto>,
    State(state): State<AppState>,
    Path(params): Path<MyDirParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Album, Action::Read)?;

    let token = ctx.token().expect("token is required");
    let dir = get_dir(
        &state.config.api_url,
        token,
        &bucket.client_id,
        &bucket.id,
        &params.dir_id,
    )
    .await?;

    req.extensions_mut().insert(dir);
    Ok(next.run(req).await)
}

pub async fn file_middleware(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Extension(bucket): Extension<BucketDto>,
    Extension(dir): Extension<Dir>,
    Path(params): Path<MyFileParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Photo, Action::Read)?;

    let token = ctx.token().expect("token is required");
    let config = state.config.clone();
    let photo = get_photo(
        &config.api_url,
        token,
        &bucket.client_id,
        &bucket.id,
        &dir.id,
        &params.file_id,
    )
    .await?;

    req.extensions_mut().insert(photo);
    Ok(next.run(req).await)
}

pub async fn client_middleware(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(params): Path<ClientParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Client, Action::Read)?;

    // Regular users cannot view clients admin pages
    ensure!(
        actor.is_system_admin(),
        ForbiddenSnafu {
            msg: "Client pages require system admin privileges"
        }
    );

    let token = ctx.token().expect("token is required");
    let config = state.config.clone();

    let client = get_org(&config.api_url, token, &params.client_id).await?;

    req.extensions_mut().insert(client);
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

    let user = get_user(&config.api_url, token, &params.client_id, &params.user_id).await?;

    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

pub async fn bucket_middleware(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(params): Path<BucketParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Bucket, Action::Read)?;

    let token = ctx.token().expect("token is required");
    let config = state.config.clone();

    let bucket = get_bucket(&config.api_url, token, &params.client_id, &params.bucket_id).await?;

    req.extensions_mut().insert(bucket);
    Ok(next.run(req).await)
}

pub async fn my_bucket_middleware(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(params): Path<MyBucketParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Bucket, Action::Read)?;

    let token = ctx.token().expect("token is required");
    let config = state.config.clone();

    let bucket = get_bucket(&config.api_url, token, &actor.client_id, &params.bucket_id).await?;

    req.extensions_mut().insert(bucket);
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
