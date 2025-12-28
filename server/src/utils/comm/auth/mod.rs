use actix_web::{web, HttpRequest, HttpResponse};

use crate::utils::{
    comm::auth::{
        api_key::{extract_prefix, verify_key},
        jwt::get_jwtservice,
        models::{get_apikey, Claims, TokenResponse, TokenType},
    },
    config::get_config,
    error::KohakuError,
};

/*
  TODO:
  Add Endpoints:
    - /api/auth/login                       - Login , Gain JWT
    - /api/auth/refresh                     - Use Refresh-token to gain new access_token
    - /api/auth/manage?owner=XYZ&revoke=ABC - Manage Keys, Revoke = Delete & Blacklist, Owner = Create anew
*/
pub mod api_key;
pub mod jwt;
pub mod models;

/// Configures server so that requests get routed to the correct functions
pub fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/login", web::post().to(login));
}

/// Checks if a given [`HttpRequest`] has a valid token to access this data
///
/// # Params
/// - `req` : [`HttpRequest`] which holds the JWT in its header
/// - `required_scopes` : [`Option<Vec<&str>>`] with required permission scopes in a `category:verb` manner. At [`None`], no permission is required and only the validation of the access token is necessary
///
/// # Returns
/// A [`Result`] which is either:
/// - [`Ok`] : Indicating that the token has valid permissions to access the resource
/// - [`Err`] : A [`KohakuError`] indicating that some validation process failed
pub async fn check_authorization(
    req: &HttpRequest,
    required_scopes: Option<Vec<&str>>,
) -> Result<Claims, KohakuError> {
    // Extract token from header
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(KohakuError::ValidationError("Missing token".to_string()))?;

    // Validate token
    let service = get_jwtservice()?;
    let claims = service.validate_token(token)?;

    // Check if key is blaclisted
    if service.is_blacklisted(claims.key_id).await {
        return Err(KohakuError::Unauthorized(
            "API Key is blacklisted / was revoked!".to_string(),
        ));
    }

    // Check scopes
    let permission = required_scopes.is_none()
        || required_scopes
            .unwrap()
            .iter()
            .all(|scope| claims.scopes.contains(&scope.to_string()));
    if !permission {
        return Err(KohakuError::Unauthorized(
            "API Key has not the required permissions!".to_string(),
        ));
    }
    Ok(claims)
}

/// API Key login endpoint.
///
/// # Parameters
/// - `req` : [`HttpRequest`] body to hold the `X-API-Key` value.
///
/// # Returns
/// A [`Result`] which either is
/// - [`Ok`] : A [`HttpResponse`] with status `200` which holds the [`TokenResponse`]
/// - [`Err`] : A [`KohakuError`] based on failed operations. The [`KohakuError`] gets automatically converted to a [`HttpResponse`]
///
/// # Errors
/// Please see [`KohakuError::details`] for the mapping of [`KohakuError`] to [`actix_web::http::StatusCode`]
async fn login(req: HttpRequest) -> Result<HttpResponse, KohakuError> {
    let api_key = req
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| KohakuError::ValidationError("Missing X-API-Key header".to_string()))?;

    let config = get_config();
    let service = get_jwtservice()?;

    // Check if bootstrap_key
    if api_key == config.bootstrap_key {
        // Return bootstrap JWTs
        let response = service.create_bootstrap_token()?;
        return Ok(HttpResponse::Ok().json(response));
    }
    // Check if API Key can be found in database
    let prefix = extract_prefix(api_key)?;
    let candidates = get_apikey(None, Some(prefix)).await?;

    let mut verified_key = None;
    for candidate in candidates {
        if let Ok(true) = verify_key(api_key, &candidate.hashed_key) {
            verified_key = Some(candidate);
            break;
        }
    }

    if verified_key.is_none() {
        return Err(KohakuError::Unauthorized("Invalid API key".to_string()));
    }
    // Generate tokens
    let verified_key = verified_key.unwrap();
    let scopes = verified_key.scopes.clone();
    let response = service.create_tokens(verified_key.id, &verified_key.owner, scopes)?;

    Ok(HttpResponse::Ok().json(response))
}
