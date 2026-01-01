use actix_web::HttpRequest;

use crate::utils::{
    comm::auth::{
        api_key::{extract_prefix, verify_key},
        jwt::get_jwtservice,
        models::{get_apikey, ApiKey, Claims, TokenType},
    },
    error::KohakuError,
};

pub mod api_key;
pub mod jwt;
pub mod models;
pub mod routes;

/// Helper: Quick lookup for token type duration (seconds)
pub fn token_duration(token_type: &TokenType) -> usize {
    match token_type {
        TokenType::Bootstrap => 10 * 60,         // 10 minutes
        TokenType::Access => 15 * 60,            // 15 minutes
        TokenType::Refresh => 30 * 24 * 60 * 60, // 30 days
    }
}

/// Checks if the given key is valid
///
/// # Parameters
/// - `key` - Prior generated API Key
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : [`ApiKey`] entry from the database indicating that this key is valid
/// - [`Err`]: A [`KohakuError`] which indicates that ANY operation failed, the key is invalid
pub async fn check_authorization_key(key: &str) -> Result<ApiKey, KohakuError> {
    // Check if the key is valid
    let prefix = extract_prefix(key)?;
    let candidates = get_apikey(None, Some(prefix)).await?;

    let mut verified_key = None;
    for candidate in candidates {
        if verify_key(key, &candidate.hashed_key)? {
            verified_key = Some(candidate);
            break;
        }
    }
    if verified_key.is_none() {
        return Err(KohakuError::Unauthorized("Invalid API key".to_string()));
    }

    // Note: If the implementation changes and blacklisting doesn't mean deletion in the
    // database, a blacklist check must be implemented here

    Ok(verified_key.unwrap())
}

/// Checks if the given token is valid and its corresponding key is not blacklisted
///
/// # Parameters
/// - `token` : [`String`] representation of the token
/// - `required_scopes` : Optional required token scopes for permission handling. If [`None`] not further permissions needed.
pub async fn check_authorization_token(
    req: &HttpRequest,
    required_scopes: Option<Vec<&str>>,
) -> Result<Claims, KohakuError> {
    let token = extract_token(req);
    if token.is_none() {
        return Err(KohakuError::Unauthorized("Missing token".to_string()));
    }
    let token = token.unwrap();

    // Validate token
    let service = get_jwtservice()?;
    let claims = service.validate_token(token.as_str())?;

    // Check if key is blacklisted
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

/// Extracts the api key under `X-API-Key` from the header
///
/// # Parameters
/// - `req` : [`HttpRequest`] given by the endpoint
///
/// # Returns
/// An [`Option`] which is either
/// - [`Some`] : A `&str` with the full delivered key
/// - [`None`] : If no key was found in the header
pub fn extract_key(req: &HttpRequest) -> Option<&str> {
    req.headers().get("X-API-Key").and_then(|h| h.to_str().ok())
}

/// Extracts the token under `Authorization:` from the header
///
/// # Parameters
/// - `req` : [`HttpRequest`] given by the endpoint
///
/// # Returns
/// An [`Option`] which is either
/// - [`Some`] : A String representation of the delivered JWT
/// - [`None`] : If no JWT was found under `Authorization:` inside the header
fn extract_token(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|h| h.to_string())
}
