use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::reactive::spawn_local;
use serde::Deserialize;
use snafu::OptionExt;
use snafu::ResultExt;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;
use web_sys::window;

use crate::Result;
use crate::error::ParseResponseSnafu;
use crate::error::WhateverSnafu;

#[component]
pub fn Container() -> impl IntoView {
    log!("this is from container");
    let config_res = get_server_config();
    log!("{:?}", config_res);

    match config_res {
        Ok(config) => {
            view! {
                <div>
                    <VaultContainer config=config />
                </div>
            }
        }
        Err(e) => {
            view! {
                <div>
                    <article class="message is-danger">
                        <div class="message-header">
                            <p>Error</p>
                        </div>
                        <div class="message-body">{move || e.to_string()}</div>
                    </article>
                </div>
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaultDto {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub test_cipher: String,
    pub created_at: i64,
    pub updated_at: i64,
}

async fn get_vault(url: String, token: Option<String>) -> Result<VaultDto> {
    let mut req = Request::get(&url);
    if let Some(token) = token {
        req = req.header("authorization", &format!("Bearer {}", token));
    }
    match req.send().await {
        Ok(res) => {
            let vault: VaultDto = res.json().await.context(ParseResponseSnafu)?;
            Ok(vault)
        }
        Err(e) => {
            println!("{:?}", e);
            Err("error happened vella".into())
        }
    }
}

#[component]
fn VaultContainer(#[prop(into)] config: VaultConfig) -> impl IntoView {
    let url = format!(
        "{}/orgs/{}/vaults/{}",
        &config.base_url, &config.org_id, &config.vault_id
    );
    let token = config.token.clone();

    let (loading, set_loading) = signal(false);
    let (vault, set_vault) = signal(None);
    let (err, set_err) = signal(None);

    spawn_local({
        let url = url.clone();
        let token = token.clone();

        async move {
            match get_vault(url, token).await {
                Ok(v) => {
                    set_vault.set(Some(v));
                }
                Err(e) => {
                    set_err.set(Some(e.to_string()));
                }
            };
        }
    });

    view! {
        <div>
            <p>This is the vault container</p>
        </div>
    }
}

#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub base_url: String,
    pub token: Option<String>,
    pub org_id: String,
    pub vault_id: String,
}

fn get_server_config() -> Result<VaultConfig> {
    let window = window().context(WhateverSnafu {
        msg: "Unable to read global window var",
    })?;

    let base_url = js_sys::Reflect::get(&window, &JsValue::from_str("API_URL"))
        .ok()
        .context(WhateverSnafu {
            msg: "Unable to read API_URL var",
        })?
        .as_string()
        .context(WhateverSnafu {
            msg: "Unable to parse API_URL var",
        })?;

    let token = js_sys::Reflect::get(&window, &JsValue::from_str("API_TOKEN")).ok();
    let token = match token {
        Some(t) => t.as_string(),
        None => None,
    };

    let org_id = js_sys::Reflect::get(&window, &JsValue::from_str("ORG_ID"))
        .ok()
        .context(WhateverSnafu {
            msg: "Unable to read ORG_ID  var",
        })?
        .as_string()
        .context(WhateverSnafu {
            msg: "Unable to parse ORG_ID var",
        })?;

    let vault_id = js_sys::Reflect::get(&window, &JsValue::from_str("VAULT_ID"))
        .ok()
        .context(WhateverSnafu {
            msg: "Unable to read VAULT_ID  var",
        })?
        .as_string()
        .context(WhateverSnafu {
            msg: "Unable to parse VAULT_ID var",
        })?;

    Ok(VaultConfig {
        base_url,
        token,
        org_id,
        vault_id,
    })
}
