use reqwest::Client;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

use crate::config::Config;
use crate::error::{CsrfTokenSnafu, HttpClientSnafu, HttpResponseParseSnafu};
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};
use dto::vault::VaultDto;

use super::handle_response_error;

#[derive(Clone, Deserialize, Serialize)]
pub struct NewVaultFormData {
    pub name: String,
    pub test_cipher: String,
    pub token: String,
}

#[derive(Clone, Serialize)]
pub struct NewVaultData {
    pub name: String,
    pub test_cipher: String,
}

pub async fn list_vaults(api_url: &str, token: &str, org_id: &str) -> Result<Vec<VaultDto>> {
    let url = format!("{}/orgs/{}/vaults", api_url, org_id);

    let response = Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to list vaults. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "vaults", Error::OrgNotFound).await);
    }

    let vaults = response
        .json::<Vec<VaultDto>>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse vaults.".to_string(),
        })?;

    Ok(vaults)
}

pub async fn create_vault(
    config: &Config,
    token: &str,
    org_id: &str,
    form: &NewVaultFormData,
) -> Result<VaultDto> {
    let csrf_result = verify_csrf_token(&form.token, &config.jwt_secret)?;
    ensure!(csrf_result == "new_vault", CsrfTokenSnafu);

    let url = format!("{}/orgs/{}/vaults", &config.api_url, org_id);

    let data = NewVaultData {
        name: form.name.clone(),
        test_cipher: form.test_cipher.clone(),
    };

    let response = Client::new()
        .post(url)
        .bearer_auth(token)
        .json(&data)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create vault. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "vaults", Error::OrgNotFound).await);
    }

    let vault = response
        .json::<VaultDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse vault information.",
        })?;

    Ok(vault)
}

pub async fn get_vault(
    api_url: &str,
    token: &str,
    org_id: &str,
    vault_id: &str,
) -> Result<VaultDto> {
    let url = format!("{}/orgs/{}/vaults/{}", api_url, org_id, vault_id);
    let response = Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to get vault. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "vaults", Error::VaultNotFound).await);
    }

    let vault = response
        .json::<VaultDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse vault.",
        })?;

    Ok(vault)
}

pub async fn delete_vault(
    config: &Config,
    token: &str,
    org_id: &str,
    vault_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let csrf_result = verify_csrf_token(&csrf_token, &config.jwt_secret)?;
    ensure!(csrf_result == vault_id, CsrfTokenSnafu);
    let url = format!("{}/orgs/{}/vaults/{}", &config.api_url, org_id, vault_id);
    let response = Client::new()
        .delete(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to delete vault. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "vaults", Error::VaultNotFound).await);
    }

    Ok(())
}
