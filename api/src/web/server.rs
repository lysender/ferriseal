use axum::{Router, body::Body, middleware, response::Response};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{Level, error, info};

use crate::Result;
use crate::config::Config;
use crate::error::{ErrorInfo, ErrorResponse};
use crate::state::create_app_state;
use crate::web::routes::all_routes;

#[cfg(test)]
use axum_test::TestServer;

pub async fn run_web_server(config: &Config) -> Result<()> {
    let port = config.server.port;
    let state = create_app_state(config).await?;

    let routes_all = Router::new()
        .merge(all_routes(state))
        .layer(middleware::map_response(response_mapper))
        .layer(
            ServiceBuilder::new().layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            ),
        );

    // Setup the server
    let ip = "127.0.0.1";
    let addr = format!("{}:{}", ip, port);
    info!("HTTP server running on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, routes_all.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    info!("HTTP server stopped");

    Ok(())
}

async fn response_mapper(res: Response) -> Response {
    let error = res.extensions().get::<ErrorInfo>();
    if let Some(e) = error {
        if e.status_code.is_server_error() {
            // Build the error response
            error!("{}", e.message);
            if let Some(bt) = &e.backtrace {
                error!("{}", bt);
            }
        }

        let body = ErrorResponse {
            status_code: e.status_code.as_u16(),
            message: e.message.as_str(),
            error: e.status_code.canonical_reason().unwrap(),
        };

        return Response::builder()
            .status(e.status_code)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
    }
    res
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use db::{
        org::{TEST_ADMIN_ORG_ID, TEST_ORG_ID},
        user::{TEST_ADMIN_USER_ID, TEST_USER_ID},
        vault::TEST_VAULT_ID,
    };

    use super::*;
    use dto::{
        entry::EntryDto, org::OrgDto, pagination::PaginatedDto, user::UserDto, vault::VaultDto,
    };
    use serde_json::json;

    fn create_test_app() -> TestServer {
        use crate::state::create_test_app_state;

        let state = create_test_app_state();

        let app = Router::new()
            .merge(all_routes(state))
            .layer(middleware::map_response(response_mapper));

        TestServer::builder()
            .save_cookies()
            .default_content_type("application/json")
            .expect_success_by_default()
            .mock_transport()
            .build(app)
            .unwrap()
    }

    fn create_test_user_auth_token() -> Result<String> {
        use crate::state::create_test_app_state;
        use crate::token::create_auth_token;
        use db::org::TEST_ORG_ID;
        use db::user::TEST_USER_ID;
        use dto::actor::ActorPayload;

        let state = create_test_app_state();

        let actor = ActorPayload {
            id: TEST_USER_ID.to_string(),
            org_id: TEST_ORG_ID.to_string(),
            scope: "auth vault".to_string(),
        };

        create_auth_token(&actor, &state.config.jwt_secret)
    }

    fn create_test_admin_auth_token() -> Result<String> {
        use crate::state::create_test_app_state;
        use crate::token::create_auth_token;
        use db::org::TEST_ADMIN_ORG_ID;
        use db::user::TEST_ADMIN_USER_ID;
        use dto::actor::ActorPayload;

        let state = create_test_app_state();

        let actor = ActorPayload {
            id: TEST_ADMIN_USER_ID.to_string(),
            org_id: TEST_ADMIN_ORG_ID.to_string(),
            scope: "auth vault".to_string(),
        };

        create_auth_token(&actor, &state.config.jwt_secret)
    }

    #[tokio::test]
    async fn test_home_page() {
        let server = create_test_app();
        let response = server.get("/").await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_health_live() {
        let server = create_test_app();
        let response = server.get("/health/liveness").await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_login_invalid() {
        let server = create_test_app();
        let response = server
            .post("/auth/token")
            .json(&json!({
                "username": "pythagoras",
                "password": "not-a-strong-password",
            }))
            .expect_failure()
            .await;

        response.assert_status_unauthorized();
    }

    #[tokio::test]
    async fn test_login_admin() {
        let server = create_test_app();
        let response = server
            .post("/auth/token")
            .json(&json!({
                "username": "admin",
                "password": "secret-password",
            }))
            .await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_login_user() {
        let server = create_test_app();
        let response = server
            .post("/auth/token")
            .json(&json!({
                "username": "user",
                "password": "secret-password",
            }))
            .await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_list_orgs_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let orgs: Vec<OrgDto> = server
            .get("/orgs")
            .authorization_bearer(token.as_str())
            .await
            .json();

        // Should only see its own
        assert_eq!(orgs.len(), 1);

        let org = orgs.first().unwrap();
        assert_eq!(org.id.as_str(), TEST_ORG_ID);
    }

    #[tokio::test]
    async fn test_list_orgs_as_admin() {
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let orgs: Vec<OrgDto> = server
            .get("/orgs")
            .authorization_bearer(token.as_str())
            .await
            .json();

        // Should see all orgs
        assert_eq!(orgs.len(), 2);
    }

    #[tokio::test]
    async fn test_user_profile_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let user: UserDto = server
            .get("/user")
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(user.id.as_str(), TEST_USER_ID);
    }

    #[tokio::test]
    async fn test_user_profile_as_admin() {
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let user: UserDto = server
            .get("/user")
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(user.id.as_str(), TEST_ADMIN_USER_ID);
    }

    #[tokio::test]
    async fn test_get_user_org_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let url = format!("/orgs/{}", TEST_ORG_ID);
        let org: OrgDto = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(org.id.as_str(), TEST_ORG_ID);
    }

    #[tokio::test]
    async fn test_get_admin_org_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let url = format!("/orgs/{}", TEST_ADMIN_ORG_ID);
        let response = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .expect_failure()
            .await;

        response.assert_status_not_found();
    }

    #[tokio::test]
    async fn test_get_user_org_as_admin() {
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let url = format!("/orgs/{}", TEST_ORG_ID);
        let org: OrgDto = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(org.id.as_str(), TEST_ORG_ID);
    }

    #[tokio::test]
    async fn test_get_admin_org_as_admin() {
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let url = format!("/orgs/{}", TEST_ADMIN_ORG_ID);
        let org: OrgDto = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(org.id.as_str(), TEST_ADMIN_ORG_ID);
    }

    #[tokio::test]
    async fn test_get_org_not_found_as_admin() {
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let url = "/orgs/0196d27b10c47e1abb9aae6cf3eea36a";
        let response = server
            .get(url)
            .authorization_bearer(token.as_str())
            .expect_failure()
            .await;

        response.assert_status_not_found();
    }

    #[tokio::test]
    async fn test_list_user_vaults_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let url = format!("/orgs/{}/vaults", TEST_ORG_ID);
        let vaults: Vec<VaultDto> = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(vaults.len(), 1);
        let vault = vaults.first().unwrap();
        assert_eq!(vault.id.as_str(), TEST_VAULT_ID);
    }

    #[tokio::test]
    async fn test_list_admin_vaults_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let url = format!("/orgs/{}/vaults", TEST_ADMIN_ORG_ID);
        let response = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .expect_failure()
            .await;

        response.assert_status_not_found();
    }

    #[tokio::test]
    async fn test_list_user_vaults_as_admin() {
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let url = format!("/orgs/{}/vaults", TEST_ORG_ID);
        let vaults: Vec<VaultDto> = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(vaults.len(), 1);
        let vault = vaults.first().unwrap();
        assert_eq!(vault.id.as_str(), TEST_VAULT_ID);
    }

    #[tokio::test]
    async fn test_list_admin_vaults_as_admin() {
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let url = format!("/orgs/{}/vaults", TEST_ADMIN_ORG_ID);
        let vaults: Vec<VaultDto> = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(vaults.len(), 0);
    }

    #[tokio::test]
    async fn test_get_user_vault_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let url = format!("/orgs/{}/vaults/{}", TEST_ORG_ID, TEST_VAULT_ID);
        let vault: VaultDto = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(vault.id.as_str(), TEST_VAULT_ID);
    }

    #[tokio::test]
    async fn test_get_user_vault_as_admin() {
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let url = format!("/orgs/{}/vaults/{}", TEST_ORG_ID, TEST_VAULT_ID);
        let vault: VaultDto = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(vault.id.as_str(), TEST_VAULT_ID);
    }

    #[tokio::test]
    async fn test_get_user_vault_not_found_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let url = format!(
            "/orgs/{}/vaults/0196d277ffc47800ba5e7ffb6a557f31",
            TEST_ORG_ID
        );
        let response = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .expect_failure()
            .await;

        response.assert_status_not_found();
    }

    #[tokio::test]
    async fn test_list_user_users_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let url = format!("/orgs/{}/users", TEST_ORG_ID);
        let users: Vec<UserDto> = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(users.len(), 1);
        let user = users.first().unwrap();
        assert_eq!(user.id.as_str(), TEST_USER_ID);
    }

    #[tokio::test]
    async fn test_list_user_users_as_admin() {
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let url = format!("/orgs/{}/users", TEST_ORG_ID);
        let users: Vec<UserDto> = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(users.len(), 1);
        let user = users.first().unwrap();
        assert_eq!(user.id.as_str(), TEST_USER_ID);
    }

    #[tokio::test]
    async fn test_list_user_entries_as_user() {
        let server = create_test_app();
        let token = create_test_user_auth_token().unwrap();
        let url = format!("/orgs/{}/vaults/{}/entries", TEST_ORG_ID, TEST_VAULT_ID,);
        let listing: PaginatedDto<EntryDto> = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .await
            .json();

        assert_eq!(listing.meta.total_records, 0);
    }

    #[tokio::test]
    async fn test_list_user_entries_as_admin() {
        // System Admins cannot view vault entries
        let server = create_test_app();
        let token = create_test_admin_auth_token().unwrap();
        let url = format!("/orgs/{}/vaults/{}/entries", TEST_ORG_ID, TEST_VAULT_ID,);
        let response = server
            .get(url.as_str())
            .authorization_bearer(token.as_str())
            .expect_failure()
            .await;

        response.assert_status_forbidden();
    }
}
