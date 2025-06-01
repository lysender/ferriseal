use axum::extract::FromRef;
use std::sync::Arc;

use crate::{Result, config::Config};
use db::db::{DbMapper, create_db_mapper};

#[derive(Clone, FromRef)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<DbMapper>,
}

pub async fn create_app_state(config: &Config) -> Result<AppState> {
    let db = create_db_mapper(config.db.url.as_str());
    Ok(AppState {
        config: config.clone(),
        db: Arc::new(db),
    })
}

#[cfg(test)]
pub fn create_test_app_state() -> AppState {
    use std::path::PathBuf;

    use crate::config::{CloudConfig, DbConfig, ServerConfig};
    use db::db::create_test_db_mapper;

    let config = Config {
        jwt_secret: "0196d1dbbfd87819b9183f14ac3ed485".to_string(),
        upload_dir: PathBuf::new(),
        cloud: CloudConfig {
            project_id: "test-cloud-project-id".to_string(),
            credentials: "test-credentials-file.json".to_string(),
        },
        server: ServerConfig { port: 43700 },
        db: DbConfig {
            url: "-url".to_string(),
        },
    };

    let db = create_test_db_mapper();

    AppState {
        config,
        db: Arc::new(db),
    }
}
