use askama::Template;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use snafu::ResultExt;
use urlencoding::encode;

use crate::models::PaginationLinks;
use crate::models::tokens::TokenFormData;
use crate::services::entries::{
    EntryFormData, SearchEntriesParams, create_entry, delete_entry, list_entries,
};
use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::token::create_csrf_token,
    web::{Action, Resource, enforce_policy},
};
use dto::entry::EntryDto;
use dto::vault::VaultDto;

#[derive(Template)]
#[template(path = "widgets/search_entries.html")]
struct SearchEntriesTemplate {
    vault: VaultDto,
    entries: Vec<EntryDto>,
    pagination: Option<PaginationLinks>,
    can_create: bool,
    error_message: Option<String>,
}

pub async fn search_entries_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    State(state): State<AppState>,
    Query(query): Query<SearchEntriesParams>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Entry, Action::Read)?;

    let oid = vault.org_id.clone();
    let vid = vault.id.clone();

    let mut tpl = SearchEntriesTemplate {
        vault,
        entries: Vec::new(),
        pagination: None,
        can_create: enforce_policy(actor, Resource::Entry, Action::Create).is_ok(),
        error_message: None,
    };

    let token = ctx.token().expect("token is required");
    match list_entries(&state.config.api_url, token, &oid, &vid, &query).await {
        Ok(entries) => {
            let mut keyword_param: String = "".to_string();
            if let Some(keyword) = &query.keyword {
                keyword_param = format!("&keyword={}", encode(keyword).to_string());
            }
            tpl.entries = entries.data;
            tpl.pagination = Some(PaginationLinks::new(&entries.meta, "", &keyword_param));

            Ok(Response::builder()
                .status(200)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            tpl.error_message = Some(error_info.message);

            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "pages/new_entry.html")]
struct NewEntryTemplate {
    t: TemplateData,
    vault: VaultDto,
    payload: EntryFormData,
    error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/new_entry_form.html")]
struct EntryFormTemplate {
    vault: VaultDto,
    payload: EntryFormData,
    error_message: Option<String>,
}

pub async fn new_entry_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(vault): Extension<VaultDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Entry, Action::Create)?;

    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);
    t.title = "New Entry".to_string();

    let token = create_csrf_token("new_entry", &config.jwt_secret)?;

    let tpl = NewEntryTemplate {
        t,
        vault,
        payload: EntryFormData {
            label: "".to_string(),
            cipher_username: None,
            cipher_password: None,
            cipher_notes: None,
            cipher_extra_notes: None,
            token,
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_new_entry_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    State(state): State<AppState>,
    payload: Form<EntryFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Entry, Action::Create)?;

    let token = create_csrf_token("new_entry", &config.jwt_secret)?;
    let oid = vault.org_id.clone();
    let vid = vault.id.clone();

    let mut tpl = EntryFormTemplate {
        vault,
        payload: EntryFormData {
            label: "".to_string(),
            cipher_username: None,
            cipher_password: None,
            cipher_notes: None,
            cipher_extra_notes: None,
            token,
        },
        error_message: None,
    };

    let status: StatusCode;

    let entry = EntryFormData {
        label: payload.label.clone(),
        cipher_username: payload.cipher_username.clone(),
        cipher_password: payload.cipher_password.clone(),
        cipher_notes: payload.cipher_notes.clone(),
        cipher_extra_notes: payload.cipher_extra_notes.clone(),
        token: payload.token.clone(),
    };

    let token = ctx.token().expect("token is required");
    let result = create_entry(&config, token, &oid, &vid, entry).await;

    match result {
        Ok(_) => {
            let next_url = format!("/vaults/{}", &vid);
            // Weird but can't do a redirect here, let htmx handle it
            Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", next_url)
                .body(Body::from("".to_string()))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            status = error_info.status_code;
            tpl.error_message = Some(error_info.message);

            tpl.payload.label = payload.label.clone();

            // Will only arrive here on error
            Ok(Response::builder()
                .status(status)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "pages/entry.html")]
struct DirTemplate {
    t: TemplateData,
    vault: VaultDto,
    entry: EntryDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

pub async fn dir_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(vault): Extension<VaultDto>,
    Extension(dir): Extension<Dir>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");
    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);

    t.title = format!("Photos - {}", &dir.label);
    t.styles = vec![config.assets.gallery_css.clone()];
    t.scripts = vec![config.assets.gallery_js.clone()];

    let tpl = DirTemplate {
        t,
        vault,
        dir,
        updated: false,
        can_edit: enforce_policy(actor, Resource::Album, Action::Update).is_ok(),
        can_delete: enforce_policy(actor, Resource::Album, Action::Delete).is_ok(),
        can_add_files: enforce_policy(actor, Resource::Photo, Action::Create).is_ok(),
        can_delete_files: enforce_policy(actor, Resource::Photo, Action::Delete).is_ok(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/edit_dir_controls.html")]
struct EditDirControlsTemplate {
    vault: VaultDto,
    dir: Dir,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
    can_add_files: bool,
    can_delete_files: bool,
}

/// Simply re-renders the edit and delete dir controls
pub async fn edit_dir_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(dir): Extension<Dir>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Album, Action::Update)?;

    let tpl = EditDirControlsTemplate {
        vault,
        dir,
        updated: false,
        can_edit: enforce_policy(actor, Resource::Album, Action::Update).is_ok(),
        can_delete: enforce_policy(actor, Resource::Album, Action::Delete).is_ok(),
        can_add_files: enforce_policy(actor, Resource::Photo, Action::Create).is_ok(),
        can_delete_files: enforce_policy(actor, Resource::Photo, Action::Delete).is_ok(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/edit_dir_form.html")]
struct EditDirFormTemplate {
    payload: UpdateDirFormData,
    vault: VaultDto,
    dir: Dir,
    error_message: Option<String>,
}

/// Renders the edit album form
pub async fn edit_dir_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(dir): Extension<Dir>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Album, Action::Update)?;

    let token = create_csrf_token(&dir.id, &config.jwt_secret)?;

    let label = dir.label.clone();
    let tpl = EditDirFormTemplate {
        vault,
        dir,
        payload: UpdateDirFormData { label, token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

/// Handles the edit album submission
pub async fn post_edit_dir_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(dir): Extension<Dir>,
    State(state): State<AppState>,
    payload: Form<UpdateDirFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let cid = vault.org_id.clone();
    let bid = vault.id.clone();
    let dir_id = dir.id.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Album, Action::Update)?;

    let token = create_csrf_token(&dir_id, &config.jwt_secret)?;

    let mut tpl = EditDirFormTemplate {
        vault: vault.clone(),
        dir: dir.clone(),
        payload: UpdateDirFormData {
            label: "".to_string(),
            token,
        },
        error_message: None,
    };

    tpl.payload.label = payload.label.clone();

    let token = ctx.token().expect("token is required");
    let result = update_dir(&config, token, &cid, &bid, &dir_id, &payload).await;
    match result {
        Ok(updated_dir) => {
            // Render the controls again with an out-of-bound swap for title
            let tpl = EditDirControlsTemplate {
                vault,
                dir: updated_dir,
                updated: true,
                can_edit: enforce_policy(actor, Resource::Album, Action::Update).is_ok(),
                can_delete: enforce_policy(actor, Resource::Album, Action::Delete).is_ok(),
                can_add_files: enforce_policy(actor, Resource::Photo, Action::Create).is_ok(),
                can_delete_files: enforce_policy(actor, Resource::Photo, Action::Delete).is_ok(),
            };
            Ok(Response::builder()
                .status(200)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let status;
            match err {
                Error::Validation { msg } => {
                    status = 400;
                    tpl.error_message = Some(msg);
                }
                Error::LoginRequired => {
                    status = 401;
                    tpl.error_message = Some("Login required.".to_string());
                }
                any_err => {
                    status = 500;
                    tpl.error_message = Some(any_err.to_string());
                }
            }

            Ok(Response::builder()
                .status(status)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/delete_dir_form.html")]
struct DeleteDirTemplate {
    vault: VaultDto,
    dir: Dir,
    payload: TokenFormData,
    error_message: Option<String>,
}

pub async fn get_delete_dir_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(dir): Extension<Dir>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Album, Action::Delete)?;
    let token = create_csrf_token(&dir.id, &config.jwt_secret)?;

    let tpl = DeleteDirTemplate {
        vault,
        dir,
        payload: TokenFormData { token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

/// Deletes album then redirect or show error
pub async fn post_delete_dir_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(dir): Extension<Dir>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Album, Action::Delete)?;

    let token = create_csrf_token(&dir.id, &config.jwt_secret)?;

    let auth_token = ctx.token().expect("token is required");

    let result = delete_dir(
        &config,
        auth_token,
        &vault.org_id,
        &vault.id,
        &dir.id,
        &payload.token,
    )
    .await;

    match result {
        Ok(_) => {
            let bid = vault.id.clone();

            // Render same form but trigger a redirect to home
            let tpl = DeleteDirTemplate {
                vault,
                dir,
                payload: TokenFormData {
                    token: "".to_string(),
                },
                error_message: None,
            };
            Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", format!("/vaults/{}", &bid))
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            let error_message = Some(error_info.message);

            // Just render the form on first load or on error
            let tpl = DeleteDirTemplate {
                vault,
                dir,
                payload: TokenFormData { token },
                error_message,
            };

            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}
