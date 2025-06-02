use askama::Template;
use axum::{
    Extension,
    body::Body,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use snafu::ResultExt;

use crate::{
    Result,
    ctx::Ctx,
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::TemplateData,
    services::vaults::list_vaults,
};
use crate::{models::Pref, run::AppState};
use dto::vault::VaultDto;

use super::{Action, Resource, enforce_policy};

#[derive(Template)]
#[template(path = "pages/index.html")]
struct IndexTemplate {
    t: TemplateData,
    vaults: Vec<VaultDto>,
}

pub async fn index_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");
    let _ = enforce_policy(actor, Resource::Vault, Action::Read)?;

    if actor.is_system_admin() {
        // Redirect to orgs page
        return Ok(Redirect::to("/orgs").into_response());
    }

    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);
    t.title = String::from("Home");

    let token = ctx.token().expect("token is required");
    let vaults = list_vaults(&state.config.api_url, token, &actor.org_id).await?;

    let tpl = IndexTemplate { t, vaults };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}
