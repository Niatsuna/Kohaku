use actix_web::{web, HttpRequest, HttpResponse};

use crate::utils::{
    comm::auth::{
        api_key::{extract_prefix, verify_key},
        jwt::get_jwtservice,
        models::{get_apikey, TokenResponse},
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

pub fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/login", web::post().to(login));
}

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
        let token = service.create_bootstrap_token()?;
        let response = TokenResponse {
            access_token: token,
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_in: 600, //10 minutes
        };
        return Ok(HttpResponse::Ok().json(response));
    }
    // Check if API Key can be found in database
    let prefix = extract_prefix(api_key);
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
