use askama::Template;
use axum::http::StatusCode;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use snafu::{ResultExt, ensure};

use crate::error::ForbiddenSnafu;
use crate::models::tokens::TokenFormData;
use crate::services::orgs::{create_org, delete_org, update_org};
use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::{
        orgs::{OrgFormSubmitData, list_orgs},
        token::create_csrf_token,
    },
    web::{Action, Resource, enforce_policy},
};
use dto::org::OrgDto;
use dto::role::Permission;

#[derive(Template)]
#[template(path = "widgets/orgs.html")]
struct OrgsTemplate {
    error_message: Option<String>,
    orgs: Vec<OrgDto>,
}

#[derive(Template)]
#[template(path = "pages/orgs.html")]
struct OrgsPageTemplate {
    t: TemplateData,
}

pub async fn orgs_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Org, Action::Read)?;

    ensure!(
        actor.is_system_admin(),
        ForbiddenSnafu {
            msg: "Orgs page require system admin privileges."
        }
    );

    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);
    t.title = String::from("Orgs");

    let tpl = OrgsPageTemplate { t };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn orgs_listing_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let mut tpl = OrgsTemplate {
        error_message: None,
        orgs: Vec::new(),
    };

    let token = ctx.token().expect("token is required");
    match list_orgs(&config.api_url, token).await {
        Ok(orgs) => {
            tpl.orgs = orgs;
            build_response(tpl)
        }
        Err(err) => build_error_response(tpl, err),
    }
}

#[derive(Template)]
#[template(path = "pages/new_org.html")]
struct NewOrgTemplate {
    t: TemplateData,
    action: String,
    payload: OrgFormSubmitData,
    error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/new_org_form.html")]
struct OrgFormTemplate {
    action: String,
    payload: OrgFormSubmitData,
    error_message: Option<String>,
}

pub async fn new_org_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Org, Action::Create)?;

    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);
    t.title = String::from("Create New Org");

    let token = create_csrf_token("new_org", &config.jwt_secret)?;

    let tpl = NewOrgTemplate {
        t,
        action: "/orgs/new".to_string(),
        payload: OrgFormSubmitData {
            name: "".to_string(),
            token,
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_new_org_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    payload: Form<OrgFormSubmitData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Org, Action::Create)?;

    let token = create_csrf_token("new_org", &config.jwt_secret)?;

    let mut tpl = OrgFormTemplate {
        action: "/orgs/new".to_string(),
        payload: ClientFormSubmitData {
            name: "".to_string(),
            token,
        },
        error_message: None,
    };

    let status: StatusCode;

    let payload = OrgFormSubmitData {
        name: payload.name.clone(),
        token: payload.token.clone(),
    };

    let token = ctx.token().expect("token is required");
    let result = create_org(&config, token, &payload).await;

    match result {
        Ok(_) => {
            let next_url = "/orgs".to_string();
            // Weird but can't do a redirect here, let htmx handle it
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", next_url)
                .body(Body::from("".to_string()))
                .context(ResponseBuilderSnafu)?);
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            status = error_info.status_code;
            tpl.error_message = Some(error_info.message);
        }
    }

    tpl.payload.name = payload.name.clone();

    // Will only arrive here on error
    Ok(Response::builder()
        .status(status)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "pages/org.html")]
struct OrgPageTemplate {
    t: TemplateData,
    org: OrgDto,
    can_edit: bool,
    can_delete: bool,
    updated: bool,
}

pub async fn org_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");
    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);

    t.title = format!("Org - {}", &org.name);

    let tpl = OrgPageTemplate {
        t,
        org,
        can_edit: actor.has_permissions(&vec![Permission::OrgsEdit]),
        can_delete: actor.has_permissions(&vec![Permission::OrgsDelete]),
        updated: false,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/edit_org_form.html")]
struct EditOrgFormTemplate {
    org: OrgDto,
    payload: OrgFormSubmitData,
    error_message: Option<String>,
}

pub async fn edit_org_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Org, Action::Update)?;

    let token = create_csrf_token(&org.id, &config.jwt_secret)?;

    let tpl = EditOrgFormTemplate {
        org: org.clone(),
        payload: OrgFormSubmitData {
            name: org.name,
            token,
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_edit_org_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    payload: Form<OrgFormSubmitData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Org, Action::Update)?;

    let token = create_csrf_token(&org.id, &config.jwt_secret)?;

    let mut tpl = EditOrgFormTemplate {
        org: org.clone(),
        payload: OrgFormSubmitData {
            name: "".to_string(),
            token,
        },
        error_message: None,
    };

    let status: StatusCode;

    let payload = OrgFormSubmitData {
        name: payload.name.clone(),
        token: payload.token.clone(),
    };

    let token = ctx.token().expect("token is required");
    let result = update_org(&config, token, &org.id, &payload).await;

    match result {
        Ok(updated_org) => {
            // Render the controls back
            let tpl = EditOrgsControlsTemplate {
                org: updated_org,
                updated: true,
                can_edit: actor.has_permissions(&vec![Permission::OrgsEdit]),
                can_delete: actor.has_permissions(&vec![Permission::OrgsDelete]),
            };

            Ok(Response::builder()
                .status(200)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            status = error_info.status_code;
            tpl.error_message = Some(error_info.message);

            tpl.payload.name = payload.name.clone();

            Ok(Response::builder()
                .status(status)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/edit_org_controls.html")]
struct EditOrgsControlsTemplate {
    org: OrgDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

pub async fn edit_org_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Org, Action::Update)?;

    let tpl = EditOrgsControlsTemplate {
        org,
        updated: false,
        can_edit: actor.has_permissions(&vec![Permission::OrgsEdit]),
        can_delete: actor.has_permissions(&vec![Permission::OrgsDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/delete_org_form.html")]
struct DeleteOrgFormTemplate {
    org: OrgDto,
    payload: TokenFormData,
    error_message: Option<String>,
}

pub async fn delete_org_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Org, Action::Delete)?;

    let token = create_csrf_token(&org.id, &config.jwt_secret)?;

    let tpl = DeleteOrgFormTemplate {
        org: org.clone(),
        payload: TokenFormData { token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_delete_org_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Org, Action::Delete)?;

    let token = create_csrf_token(&org.id, &config.jwt_secret)?;

    let mut tpl = DeleteOrgFormTemplate {
        org: org.clone(),
        payload: TokenFormData { token },
        error_message: None,
    };

    let status: StatusCode;

    let token = ctx.token().expect("token is required");
    let result = delete_org(&config, token, &org.id, &payload.token).await;

    match result {
        Ok(_) => {
            // Render same form but trigger a redirect to home
            let tpl = DeleteOrgFormTemplate {
                org,
                payload: TokenFormData {
                    token: "".to_string(),
                },
                error_message: None,
            };
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", "/orgs")
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?);
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            status = error_info.status_code;
            tpl.error_message = Some(error_info.message);

            Ok(Response::builder()
                .status(status)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

fn build_response(tpl: OrgsTemplate) -> Result<Response<Body>> {
    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

fn build_error_response(mut tpl: OrgsTemplate, error: Error) -> Result<Response<Body>> {
    let error_info = ErrorInfo::from(&error);
    tpl.error_message = Some(error_info.message);

    Ok(Response::builder()
        .status(error_info.status_code)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}
