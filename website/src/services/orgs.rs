use reqwest::Client;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

use crate::config::Config;
use crate::error::{CsrfTokenSnafu, HttpClientSnafu, HttpResponseParseSnafu};
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};
use dto::org::OrgDto;

use super::handle_response_error;

#[derive(Clone, Deserialize, Serialize)]
pub struct OrgFormSubmitData {
    pub name: String,
    pub token: String,
}

#[derive(Clone, Serialize)]
pub struct OrgSubmitData {
    pub name: String,
}

pub async fn list_orgs(api_url: &str, token: &str) -> Result<Vec<OrgDto>> {
    let url = format!("{}/orgs", api_url);

    let response = Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to list orgs. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    let orgs = response
        .json::<Vec<OrgDto>>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse orgs.".to_string(),
        })?;

    Ok(orgs)
}

pub async fn create_org(config: &Config, token: &str, form: &OrgFormSubmitData) -> Result<OrgDto> {
    let csrf_result = verify_csrf_token(&form.token, &config.jwt_secret)?;
    ensure!(csrf_result == "new_org", CsrfTokenSnafu);

    let url = format!("{}/orgs", &config.api_url);

    let data = OrgSubmitData {
        name: form.name.clone(),
    };
    let response = Client::new()
        .post(url)
        .bearer_auth(token)
        .json(&data)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create org. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    let org = response
        .json::<OrgDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org information.",
        })?;

    Ok(org)
}

pub async fn get_org(api_url: &str, token: &str, org_id: &str) -> Result<OrgDto> {
    let url = format!("{}/orgs/{}", api_url, org_id);
    let response = Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to get org. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    let org = response
        .json::<OrgDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org.",
        })?;

    Ok(org)
}

pub async fn update_org(
    config: &Config,
    token: &str,
    org_id: &str,
    form: &OrgFormSubmitData,
) -> Result<OrgDto> {
    let csrf_result = verify_csrf_token(&form.token, &config.jwt_secret)?;
    ensure!(&csrf_result == org_id, CsrfTokenSnafu);

    let url = format!("{}/orgs/{}", &config.api_url, org_id);
    let data = OrgSubmitData {
        name: form.name.clone(),
    };
    let response = Client::new()
        .patch(url)
        .bearer_auth(token)
        .json(&data)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update org. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    let org = response
        .json::<OrgDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org information.",
        })?;

    Ok(org)
}

pub async fn delete_org(
    config: &Config,
    token: &str,
    org_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let csrf_result = verify_csrf_token(&csrf_token, &config.jwt_secret)?;
    ensure!(csrf_result == org_id, CsrfTokenSnafu);

    let url = format!("{}/orgs/{}", &config.api_url, org_id);
    let response = Client::new()
        .delete(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to delete org. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    Ok(())
}
