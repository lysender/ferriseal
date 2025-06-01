use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::{
    Result,
    error::{InvalidAuthTokenSnafu, WhateverSnafu},
};
use dto::actor::ActorPayload;

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    oid: String,
    scope: String,
    exp: usize,
}

// Duration in seconds
const EXP_DURATION: i64 = 60 * 60 * 24; // 1 day

pub fn create_auth_token(actor: &ActorPayload, secret: &str) -> Result<String> {
    let exp = Utc::now() + Duration::seconds(EXP_DURATION);
    let data = actor.clone();

    let claims = Claims {
        sub: data.id,
        oid: data.org_id,
        scope: data.scope,
        exp: exp.timestamp() as usize,
    };

    let Ok(token) = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    ) else {
        return WhateverSnafu {
            msg: "Error creating JWT token".to_string(),
        }
        .fail();
    };

    Ok(token)
}

pub fn verify_auth_token(token: &str, secret: &str) -> Result<ActorPayload> {
    let Ok(decoded) = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) else {
        return InvalidAuthTokenSnafu {}.fail();
    };

    ensure!(decoded.claims.sub.len() > 0, InvalidAuthTokenSnafu {});
    ensure!(decoded.claims.scope.len() > 0, InvalidAuthTokenSnafu {});

    Ok(ActorPayload {
        id: decoded.claims.sub,
        org_id: decoded.claims.oid,
        scope: decoded.claims.scope,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_token() {
        // Generate token
        let actor = ActorPayload {
            id: "thor01".to_string(),
            org_id: "org01".to_string(),
            scope: "auth vault".to_string(),
        };
        let token = create_auth_token(&actor, "secret").unwrap();
        assert!(token.len() > 0);

        // Validate it back
        let actor = verify_auth_token(&token, "secret").unwrap();
        assert_eq!(actor.id, "thor01".to_string());
        assert_eq!(actor.org_id, "org01".to_string());
        assert_eq!(actor.scope, "auth vault".to_string());
    }

    #[test]
    fn test_expired_token() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJqZWN0IjoidGhvcjAxIiwic2NvcGUiOiJhdXRoIGZpbGVzIiwiZXhwIjoxNzE5MDc2MTI2fQ.ep8nXWWHS75MxoOY_yB4m0uoWgxCz1bPNvTPIgourcQ".to_string();
        let result = verify_auth_token(&token, "secret");
        assert!(result.is_err());
    }
}
