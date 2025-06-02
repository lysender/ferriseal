use askama::Template;
use axum::extract::Query;
use axum::{Extension, body::Body, extract::State, response::Response};
use dto::vault::VaultDto;
use snafu::ResultExt;

use crate::models::ListDirsParams;
use crate::{
    Result,
    ctx::Ctx,
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
};

#[derive(Template)]
#[template(path = "pages/my_vault.html")]
struct MyVaultPageTemplate {
    t: TemplateData,
    vault: VaultDto,
    query_params: String,
}

pub async fn my_vault_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(vault): Extension<VaultDto>,
    State(state): State<AppState>,
    Query(query): Query<ListDirsParams>,
) -> Result<Response<Body>> {
    let actor = ctx.actor().expect("actor is required");
    let mut t = TemplateData::new(&state, Some(actor.clone()), &pref);

    t.title = format!("Vault - {}", &vault.name);

    let tpl = MyVaultPageTemplate {
        t,
        vault,
        query_params: query.to_string(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}
