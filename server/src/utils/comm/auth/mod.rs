use actix_web::HttpRequest;

use crate::utils::{
    comm::auth::{jwt::get_jwtservice, models::Claims},
    error::KohakuError,
};

pub mod api_key;
pub mod jwt;
pub mod models;
pub mod routes;

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
