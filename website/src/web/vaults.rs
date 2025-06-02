use askama::Template;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use dto::org::OrgDto;
use dto::role::Permission;
use dto::vault::VaultDto;
use snafu::ResultExt;

use crate::models::tokens::TokenFormData;
use crate::services::vaults::{NewVaultFormData, create_vault, delete_vault, list_vaults};
use crate::{
    Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::token::create_csrf_token,
    web::{Action, Resource, enforce_policy},
};

#[derive(Template)]
#[template(path = "pages/vaults.html")]
struct VaultsPageTemplate {
    t: TemplateData,
    org: OrgDto,
    vaults: Vec<VaultDto>,
}

pub async fn vaults_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Vault, Action::Read)?;

    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);
    t.title = String::from("Vaults");

    let token = ctx.token().expect("token is required");
    let vaults = list_vaults(state.config.api_url.as_str(), token, org.id.as_str()).await?;

    let tpl = VaultsPageTemplate { t, org, vaults };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "pages/new_vault.html")]
struct NewVaultTemplate {
    t: TemplateData,
    org: OrgDto,
    payload: NewVaultFormData,
    error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/new_vault_form.html")]
struct NewVaultFormTemplate {
    org: OrgDto,
    payload: NewVaultFormData,
    error_message: Option<String>,
}

pub async fn new_vault_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Vault, Action::Create)?;

    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);
    t.title = String::from("Create New Vault");

    let token = create_csrf_token("new_vault", &config.jwt_secret)?;

    let tpl = NewVaultTemplate {
        t,
        org,
        payload: NewVaultFormData {
            name: "".to_string(),
            test_cipher: "".to_string(),
            token,
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_new_vault_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    payload: Form<NewVaultFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Vault, Action::Create)?;

    let token = create_csrf_token("new_vault", &config.jwt_secret)?;
    let oid = org.id.clone();

    let mut tpl = NewVaultFormTemplate {
        org,
        payload: NewVaultFormData {
            name: "".to_string(),
            test_cipher: "".to_string(),
            token,
        },
        error_message: None,
    };

    let vault = NewVaultFormData {
        name: payload.name.clone(),
        test_cipher: payload.test_cipher.clone(),
        token: payload.token.clone(),
    };

    let token = ctx.token().expect("token is required");
    let result = create_vault(&config, token, oid.as_str(), &vault).await;

    match result {
        Ok(_) => {
            let next_url = format!("/orgs/{}/vaults", oid.as_str());
            // Weird but can't do a redirect here, let htmx handle it
            Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", next_url)
                .body(Body::from("".to_string()))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            tpl.error_message = Some(error_info.message);

            tpl.payload.name = payload.name.clone();
            tpl.payload.test_cipher = payload.test_cipher.clone();

            // Will only arrive here on error
            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "pages/vault.html")]
struct VaultPageTemplate {
    t: TemplateData,
    org: OrgDto,
    vault: VaultDto,
    can_delete: bool,
}

pub async fn vault_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    Extension(vault): Extension<VaultDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");
    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);

    t.title = format!("Vault - {}", &vault.name);

    let tpl = VaultPageTemplate {
        t,
        org,
        vault,
        can_delete: actor.has_permissions(&vec![Permission::VaultsDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/edit_vault_controls.html")]
struct VaultControlsTemplate {
    org: OrgDto,
    vault: VaultDto,
    can_delete: bool,
}

pub async fn vault_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    Extension(vault): Extension<VaultDto>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Vault, Action::Update)?;

    let tpl = VaultControlsTemplate {
        org,
        vault,
        can_delete: actor.has_permissions(&vec![Permission::VaultsDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/delete_vault_form.html")]
struct DeleteVaultFormTemplate {
    org: OrgDto,
    vault: VaultDto,
    payload: TokenFormData,
    error_message: Option<String>,
}

pub async fn delete_vault_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    Extension(vault): Extension<VaultDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Vault, Action::Delete)?;

    let token = create_csrf_token(&vault.id, &config.jwt_secret)?;

    let tpl = DeleteVaultFormTemplate {
        org,
        vault,
        payload: TokenFormData { token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_delete_vault_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    Extension(vault): Extension<VaultDto>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let _ = enforce_policy(actor, Resource::Vault, Action::Delete)?;

    let token = create_csrf_token(&vault.id, &config.jwt_secret)?;

    let mut tpl = DeleteVaultFormTemplate {
        org: org.clone(),
        vault: vault.clone(),
        payload: TokenFormData { token },
        error_message: None,
    };

    let token = ctx.token().expect("token is required");
    let result = delete_vault(&config, token, &org.id, &vault.id, &payload.token).await;

    match result {
        Ok(_) => {
            // Render same form but trigger a redirect to home
            let oid = org.id.clone();
            let tpl = DeleteVaultFormTemplate {
                org,
                vault,
                payload: TokenFormData {
                    token: "".to_string(),
                },
                error_message: None,
            };
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", format!("/orgs/{}/vaults", &oid))
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?);
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
