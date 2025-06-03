use askama::Template;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use snafu::ResultExt;
use urlencoding::encode;

use crate::models::PaginationLinks;
use crate::models::tokens::TokenFormData;
use crate::services::entries::{
    EntryFormData, SearchEntriesParams, create_entry, delete_entry, list_entries, update_entry,
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
struct EntryTemplate {
    t: TemplateData,
    vault: VaultDto,
    entry: EntryDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

pub async fn entry_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(vault): Extension<VaultDto>,
    Extension(entry): Extension<EntryDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");
    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);

    t.title = format!("Entry - {}", &entry.label);
    t.styles = vec![config.assets.gallery_css.clone()];
    t.scripts = vec![config.assets.gallery_js.clone()];

    let tpl = EntryTemplate {
        t,
        vault,
        entry,
        updated: false,
        can_edit: enforce_policy(actor, Resource::Entry, Action::Update).is_ok(),
        can_delete: enforce_policy(actor, Resource::Entry, Action::Delete).is_ok(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/edit_entry_controls.html")]
struct EditEntryControlsTemplate {
    vault: VaultDto,
    entry: EntryDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

/// Simply re-renders the edit and delete entry controls
pub async fn edit_entry_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(entry): Extension<EntryDto>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Entry, Action::Update)?;

    let tpl = EditEntryControlsTemplate {
        vault,
        entry,
        updated: false,
        can_edit: enforce_policy(actor, Resource::Entry, Action::Update).is_ok(),
        can_delete: enforce_policy(actor, Resource::Entry, Action::Delete).is_ok(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/edit_entry_form.html")]
struct EditEntryFormTemplate {
    payload: EntryFormData,
    vault: VaultDto,
    entry: EntryDto,
    error_message: Option<String>,
}

/// Renders the edit album form
pub async fn edit_entry_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(entry): Extension<EntryDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Entry, Action::Update)?;

    let token = create_csrf_token(&entry.id, &config.jwt_secret)?;

    let tpl = EditEntryFormTemplate {
        vault,
        entry: entry.clone(),
        payload: EntryFormData {
            label: entry.label,
            cipher_username: entry.cipher_username,
            cipher_password: entry.cipher_password,
            cipher_notes: entry.cipher_notes,
            cipher_extra_notes: entry.cipher_extra_notes,
            token,
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

/// Handles the edit entry submission
pub async fn post_edit_entry_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(entry): Extension<EntryDto>,
    State(state): State<AppState>,
    payload: Form<EntryFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let oid = vault.org_id.clone();
    let vid = vault.id.clone();
    let entry_id = entry.id.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Entry, Action::Update)?;

    let token = create_csrf_token(&entry_id, &config.jwt_secret)?;

    let mut tpl = EditEntryFormTemplate {
        vault: vault.clone(),
        entry: entry.clone(),
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

    tpl.payload.label = payload.label.clone();

    let token = ctx.token().expect("token is required");
    let result = update_entry(&config, token, &oid, &vid, &entry_id, &payload).await;
    match result {
        Ok(updated_entry) => {
            // Render the controls again with an out-of-bound swap for title
            let tpl = EditEntryControlsTemplate {
                vault,
                entry: updated_entry,
                updated: true,
                can_edit: enforce_policy(actor, Resource::Entry, Action::Update).is_ok(),
                can_delete: enforce_policy(actor, Resource::Entry, Action::Delete).is_ok(),
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
#[template(path = "widgets/delete_entry_form.html")]
struct DeleteEntryTemplate {
    vault: VaultDto,
    entry: EntryDto,
    payload: TokenFormData,
    error_message: Option<String>,
}

pub async fn get_delete_entry_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(entry): Extension<EntryDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Entry, Action::Delete)?;
    let token = create_csrf_token(&entry.id, &config.jwt_secret)?;

    let tpl = DeleteEntryTemplate {
        vault,
        entry,
        payload: TokenFormData { token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

/// Deletes entry then redirect or show error
pub async fn post_delete_entry_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(vault): Extension<VaultDto>,
    Extension(entry): Extension<EntryDto>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Entry, Action::Delete)?;

    let token = create_csrf_token(&entry.id, &config.jwt_secret)?;

    let auth_token = ctx.token().expect("token is required");

    let result = delete_entry(
        &config,
        auth_token,
        &vault.org_id,
        &vault.id,
        &entry.id,
        &payload.token,
    )
    .await;

    match result {
        Ok(_) => {
            let vid = vault.id.clone();

            // Render same form but trigger a redirect to home
            let tpl = DeleteEntryTemplate {
                vault,
                entry,
                payload: TokenFormData {
                    token: "".to_string(),
                },
                error_message: None,
            };
            Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", format!("/vaults/{}", &vid))
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            let error_message = Some(error_info.message);

            // Just render the form on first load or on error
            let tpl = DeleteEntryTemplate {
                vault,
                entry,
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
