use actix_web::{web, HttpRequest, HttpResponse};
use tracing::info;

use crate::utils::{
    comm::auth::{
        api_key::{extract_prefix, generate_key, hash_key, verify_key},
        check_authorization,
        jwt::get_jwtservice,
        models::{
            create_apikey, delete_apikey, get_apikey, CreateKeyRequest, CreateKeyResponse,
            RevokeKeyRequest, TokenResponse, TokenType,
        },
    },
    config::get_config,
    error::KohakuError,
};

/// Configures server so that requests get routed to the correct functions
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/login", web::post().to(login))
        .route("/manage/refresh", web::post().to(refresh))
        .route("/manage/create", web::post().to(create))
        .route("/manage/revoke", web::post().to(revoke));
}

/// API Key login endpoint.
///
/// # Parameters
/// - `req` : [`HttpRequest`] header to hold the `X-API-Key` value.
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
    } else if service
        .is_blacklisted(verified_key.clone().unwrap().id)
        .await
    {
        return Err(KohakuError::Unauthorized(
            "API key previously revoked. Please request a new API key!".to_string(),
        ));
    }
    // Generate tokens
    let verified_key = verified_key.unwrap();
    let scopes = verified_key.scopes.clone();
    let response = service.create_tokens(verified_key.id, &verified_key.owner, scopes)?;

    Ok(HttpResponse::Ok().json(response))
}

/// API Key refresh endpoint.
///
/// # Parameters
/// - `req` : [`HttpRequest`] header to hold the `Authorization` with the refresh JWT
///
/// # Returns
/// A [`Result`] which either is
/// - [`Ok`] : A [`HttpResponse`] with status `200` which holds the [`TokenResponse`]
/// - [`Err`] : A [`KohakuError`] based on failed operations. The [`KohakuError`] gets automatically converted to a [`HttpResponse`]
///
/// # Errors
/// Please see [`KohakuError::details`] for the mapping of [`KohakuError`] to [`actix_web::http::StatusCode`]
async fn refresh(req: HttpRequest) -> Result<HttpResponse, KohakuError> {
    let claims = check_authorization(&req, None).await?;
    // Check if token is a refresh token
    if claims.token_type != TokenType::Refresh {
        return Err(KohakuError::ValidationError(
            "Invalid token type".to_string(),
        ));
    }

    // Valid, not blacklisted refresh token => Create new access token
    let service = get_jwtservice()?;
    let token = service.create_token(
        claims.owner,
        claims.key_id,
        claims.scopes,
        TokenType::Access,
    )?;
    let response = TokenResponse {
        access_token: token,
        refresh_token: None,
        token_type: "Bearer".to_string(),
        expires_in: 900,
    };
    info!("[Authentication] - Refreshed token.");
    Ok(HttpResponse::Ok().json(response))
}

/// API Key creation endpoint.
///
/// Will create a new API Key if the user uses an access token linked to the bootstrap key.
///
/// # Parameters
/// - `req` : [`HttpRequest`] header to hold the `Authorization` via JWT access token.
/// - `body` : [`CreateKeyRequest`] in a JSON Format to hold the necessary data for creation
///
/// # Returns
/// A [`Result`] which either is
/// - [`Ok`] : A [`HttpResponse`] with status `200` which holds the [`CreateKeyResponse`]
/// - [`Err`] : A [`KohakuError`] based on failed operations. The [`KohakuError`] gets automatically converted to a [`HttpResponse`]
///
/// # Errors
/// Please see [`KohakuError::details`] for the mapping of [`KohakuError`] to [`actix_web::http::StatusCode`]
async fn create(
    req: HttpRequest,
    body: web::Json<CreateKeyRequest>,
) -> Result<HttpResponse, KohakuError> {
    let _ = check_authorization(&req, Some(vec!["keys:manage"])).await?;
    if body.scopes.contains(&"keys:manage".to_string()) {
        return Err(KohakuError::ValidationError(
            "Invalid key scope: keys:manage is bootstrap key exclusive!".to_string(),
        ));
    }

    let (key, prefix) = generate_key();
    let hashed_key = hash_key(&key)?;
    let _ = create_apikey(
        hashed_key,
        prefix.clone(),
        body.owner.clone(),
        body.scopes.clone(),
    )
    .await?;
    info!(
        "[Authentication] - New API Key with prefix {} created!",
        prefix
    );

    let response = CreateKeyResponse {
        api_key: key,
        scopes: body.scopes.clone(),
    };
    Ok(HttpResponse::Ok().json(response))
}

/// API Key revokation endpoint.
///
/// Will revoke an API Key if the user uses an access token linked to the bootstrap key.
///
/// # Parameters
/// - `req` : [`HttpRequest`] header to hold the `Authorization` via JWT access token.
/// - `body` : [`RevokeKeyRequest`] in a JSON Format to hold the necessary data for revokation
///
/// # Returns
/// A [`Result`] which either is
/// - [`Ok`] : A [`HttpResponse`] with status `200`
/// - [`Err`] : A [`KohakuError`] based on failed operations. The [`KohakuError`] gets automatically converted to a [`HttpResponse`]
///
/// # Errors
/// Please see [`KohakuError::details`] for the mapping of [`KohakuError`] to [`actix_web::http::StatusCode`]
async fn revoke(
    req: HttpRequest,
    body: web::Json<RevokeKeyRequest>,
) -> Result<HttpResponse, KohakuError> {
    let _ = check_authorization(&req, Some(vec!["keys:manage"])).await?;
    let service = get_jwtservice()?;

    // Check if such a key actually exists
    let key = body.api_key.clone();

    let prefix = extract_prefix(&key)?;
    let candidates = get_apikey(None, Some(prefix.clone())).await?;
    for candidate in candidates {
        if let Ok(true) = verify_key(&key, &candidate.hashed_key) {
            // Found key: Remove it from database and blacklist it
            let key_id = candidate.id;
            delete_apikey(Some(key_id), None).await?;
            let _ = service.blacklist_key(key_id, None).await?;
            info!("[Authentication] - API Key with prefix {} revoked!", prefix);
            return Ok(HttpResponse::Ok().finish());
        }
    }
    Err(KohakuError::NotFound(
        "API key could not be found!".to_string(),
    ))
}
