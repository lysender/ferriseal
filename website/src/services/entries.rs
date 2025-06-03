use core::fmt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};
use urlencoding::encode;

use crate::config::Config;
use crate::error::{CsrfTokenSnafu, HttpClientSnafu, HttpResponseParseSnafu};
use crate::services::handle_response_error;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};
use dto::entry::EntryDto;
use dto::pagination::PaginatedDto;

#[derive(Deserialize)]
pub struct SearchEntriesParams {
    pub keyword: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}
impl Default for SearchEntriesParams {
    fn default() -> Self {
        Self {
            keyword: None,
            page: Some(1),
            per_page: Some(10),
        }
    }
}

impl fmt::Display for SearchEntriesParams {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ideally, we want an empty string if all fields are None
        if self.keyword.is_none() && self.page.is_none() && self.per_page.is_none() {
            return write!(f, "");
        }

        let keyword = self.keyword.as_deref().unwrap_or("");
        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(10);

        write!(
            f,
            "page={}&per_page={}&keyword={}",
            page,
            per_page,
            encode(keyword)
        )
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct EntryFormData {
    pub label: String,
    pub cipher_username: Option<String>,
    pub cipher_password: Option<String>,
    pub cipher_notes: Option<String>,
    pub cipher_extra_notes: Option<String>,
    pub token: String,
}

#[derive(Clone, Serialize)]
pub struct EntryData {
    pub label: String,
    pub cipher_username: Option<String>,
    pub cipher_password: Option<String>,
    pub cipher_notes: Option<String>,
    pub cipher_extra_notes: Option<String>,
}

pub async fn list_entries(
    api_url: &str,
    token: &str,
    org_id: &str,
    vault_id: &str,
    params: &SearchEntriesParams,
) -> Result<PaginatedDto<EntryDto>> {
    let url = format!("{}/orgs/{}/vaults/{}/entries", api_url, org_id, vault_id);
    let mut page = "1".to_string();
    let mut per_page = "20".to_string();

    if let Some(p) = params.page {
        page = p.to_string();
    }
    if let Some(pp) = params.per_page {
        per_page = pp.to_string();
    }
    let mut query: Vec<(&str, &str)> = vec![("page", &page), ("per_page", &per_page)];
    if let Some(keyword) = &params.keyword {
        query.push(("keyword", keyword));
    }
    let response = Client::new()
        .get(url)
        .bearer_auth(token)
        .query(&query)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to list entries. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "entries", Error::VaultNotFound).await);
    }

    let entries =
        response
            .json::<PaginatedDto<EntryDto>>()
            .await
            .context(HttpResponseParseSnafu {
                msg: "Unable to parse entries.".to_string(),
            })?;

    Ok(entries)
}

pub async fn create_entry(
    config: &Config,
    token: &str,
    org_id: &str,
    vault_id: &str,
    form: EntryFormData,
) -> Result<EntryDto> {
    let csrf_result = verify_csrf_token(&form.token, &config.jwt_secret)?;
    ensure!(csrf_result == "new_entry", CsrfTokenSnafu);

    let url = format!(
        "{}/orgs/{}/vaults/{}/entries",
        &config.api_url, org_id, vault_id
    );

    let data = EntryData {
        label: form.label,
        cipher_username: form.cipher_username,
        cipher_password: form.cipher_password,
        cipher_notes: form.cipher_notes,
        cipher_extra_notes: form.cipher_extra_notes,
    };
    let response = Client::new()
        .post(url)
        .bearer_auth(token)
        .json(&data)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create entry. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "entries", Error::VaultNotFound).await);
    }

    let entry = response
        .json::<EntryDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse entry information.",
        })?;

    Ok(entry)
}

pub async fn get_entry(
    api_url: &str,
    token: &str,
    org_id: &str,
    vault_id: &str,
    entry_id: &str,
) -> Result<EntryDto> {
    let url = format!(
        "{}/orgs/{}/vaults/{}/entries/{}",
        api_url, org_id, vault_id, entry_id
    );
    let response = Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to get entry. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "entries", Error::EntryNotFound).await);
    }

    let entry = response
        .json::<EntryDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse entry.",
        })?;

    Ok(entry)
}

pub async fn update_entry(
    config: &Config,
    token: &str,
    org_id: &str,
    vault_id: &str,
    entry_id: &str,
    form: &EntryFormData,
) -> Result<EntryDto> {
    let csrf_result = verify_csrf_token(&form.token, &config.jwt_secret)?;
    ensure!(csrf_result == entry_id, CsrfTokenSnafu);

    let url = format!(
        "{}/orgs/{}/vaults/{}/entries/{}",
        &config.api_url, org_id, vault_id, entry_id
    );
    let data = EntryData {
        label: form.label.clone(),
        cipher_username: form.cipher_username.clone(),
        cipher_password: form.cipher_password.clone(),
        cipher_notes: form.cipher_notes.clone(),
        cipher_extra_notes: form.cipher_extra_notes.clone(),
    };
    let response = Client::new()
        .patch(url)
        .bearer_auth(token)
        .json(&data)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update dir. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "dirs", Error::EntryNotFound).await);
    }

    let entry = response
        .json::<EntryDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse entry information.",
        })?;

    Ok(entry)
}

pub async fn delete_entry(
    config: &Config,
    token: &str,
    org_id: &str,
    vault_id: &str,
    entry_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let csrf_result = verify_csrf_token(&csrf_token, &config.jwt_secret)?;
    ensure!(csrf_result == entry_id, CsrfTokenSnafu);
    let url = format!(
        "{}/orgs/{}/vaults/{}/entries/{}",
        &config.api_url, org_id, vault_id, entry_id
    );
    let response = Client::new()
        .delete(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to delete entry. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "entries", Error::EntryNotFound).await);
    }

    Ok(())
}
