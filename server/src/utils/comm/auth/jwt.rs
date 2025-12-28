use chrono::{Duration, NaiveDateTime, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{OnceCell, RwLock};

#[allow(unused_imports)] // ApiKey is linked in the documentation
use crate::utils::{
    comm::auth::models::{ApiKey, Claims, TokenResponse, TokenType},
    config::get_config,
    error::KohakuError,
};

static JWT_SERVICE: OnceCell<Arc<JWTService>> = OnceCell::const_new();

/// JsonWebToken Service for generating, verifying and managing JWTs
pub struct JWTService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    // Blacklist for API Key revokation to ensure early denying of still active JWTs
    blacklist: RwLock<HashMap<i32, NaiveDateTime>>,
}

impl JWTService {
    pub fn new(encryption_key: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(encryption_key),
            decoding_key: DecodingKey::from_secret(encryption_key),
            blacklist: RwLock::new(HashMap::new()),
        }
    }

    /// Create one token for the given API key and scopes.
    ///
    /// Bootstrap and access tokens are short-lived with 10 and 15 minutes respectively.
    /// Refresh tokens live for 30 days.
    ///
    /// # Parameters
    /// - `owner` : [`String`] based identifier which service / user uses this key
    /// - `key_id`: Identifier of API key in the database
    /// - `scopes`: [`String`] based vector that grants permissions in a `category:verb` manner
    ///
    /// # Returns
    /// A [`Result`] which is either
    /// - [`Ok`] : A [`String`] representation of the token
    /// - [`Err`] : A [`KohakuError`] if some operation fails or the input is invalid
    pub fn create_token(
        &self,
        owner: String,
        key_id: i32,
        scopes: Vec<String>,
        token_type: TokenType,
    ) -> Result<String, KohakuError> {
        let management_scope = scopes.contains(&"key:manage".to_string());
        let is_bootstrap = token_type == TokenType::Bootstrap;

        // Check if given Arguments are valid (`keys:manage` exlcusively and uniquely for bootstrap key)
        if management_scope && !is_bootstrap {
            return Err(KohakuError::Unauthorized("No general tokens with the scope `keys:manage` can be created! Please refer to the bootstrap key!".to_string()));
        } else if !management_scope && is_bootstrap {
            return Err(KohakuError::Unauthorized(
                "Bootstrap Key must have `keys:manage` and no other permission scopes!".to_string(),
            ));
        }

        // Create claim
        let now = Utc::now().timestamp() as usize;
        let duration = match token_type {
            TokenType::Bootstrap => 10 * 60,         // 10 Minutes
            TokenType::Access => 15 * 60,            // 15 Minutes
            TokenType::Refresh => 30 * 24 * 60 * 60, // 30 days
        };

        let claims = Claims {
            owner,
            key_id,
            scopes: scopes.clone(),
            token_type,
            exp: now + duration,
            iat: now,
        };

        // Create token
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| KohakuError::InternalServerError(e.to_string()))
    }

    /// Helper function to generate the bootstrap token. Calls [`JWTService::create_token`].
    ///
    /// Bootstrap token lives for 10 minutes.
    ///
    /// # Returns
    /// A [`Result`] which is either
    /// - [`Ok`] : [`String`] representation of JWT [`Claims`]
    /// - [`Err`]: A [KohakuError::ValidationError] when the encoding fails
    pub fn create_bootstrap_token(&self) -> Result<String, KohakuError> {
        let owner = "system".to_string();
        let key_id = -1;
        let scopes = vec!["keys:manage".to_string()];
        let token_type = TokenType::Bootstrap;

        self.create_token(owner, key_id, scopes, token_type)
    }

    /// Helper function to generate both, access and refresh token, at once. Calls [`JWTService::create_token`].
    ///
    /// Access tokens are short-lived with only 15 minutes, while refresh tokens are valid up until 30 days.
    ///
    /// # Parameters
    /// - `key_id` : Identifier of the underlying [`ApiKey`] inside the database
    /// - `owner` : Identifier which service / user uses this key
    /// - `scopes` : Permission scopes given in a `category:verb` manner
    ///
    /// # Returns
    /// A [`Result`] which is either
    /// - [`Ok`] : A [`TokenResponse`] holding the access and refresh token
    /// - [`Err`] : Either [KohakuError::InternalServerError] when the encoding fails, or [KohakuError::Unauthorized]
    ///             when the scope contains the `key` category which is exclusive to the bootstrap key
    pub fn create_tokens(
        &self,
        key_id: i32,
        owner: &str,
        scopes: Vec<String>,
    ) -> Result<TokenResponse, KohakuError> {
        let access_token =
            self.create_token(owner.to_string(), key_id, scopes.clone(), TokenType::Access)?;
        let refresh_token = self.create_token(
            owner.to_string(),
            key_id,
            scopes.clone(),
            TokenType::Refresh,
        )?;

        Ok(TokenResponse {
            access_token,
            refresh_token: Some(refresh_token),
            token_type: "Bearer".to_string(),
            expires_in: 900,
        })
    }

    /// Validates a given token.
    ///
    /// # Parameters
    /// - `token` - A [`String`] representation reference of the underlying JWT
    ///
    /// # Returns
    /// A [`Result`] which is either
    /// - [`Ok`] : The [`Claims`] of the given token
    /// - [`Err`]: A [`KohakuError::ValidationError`] when the validation fails
    pub fn validate_token(&self, token: &str) -> Result<Claims, KohakuError> {
        let validation = Validation::default();
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| KohakuError::ValidationError(e.to_string()))?;
        Ok(token_data.claims)
    }

    /// Blacklist an API key on revokation.
    ///
    /// This feature is used when an API key gets revoked to ensure that still active JWTs get denied.
    ///
    /// Expiration time is currently: Time of blacklisting + 30 minutes
    /// At the current implementation every JWT access token will expire regardless.
    /// # Parameters
    /// - `key_id` : Identifier of the underlying [`ApiKey`] inside the database
    pub async fn blacklist_key(&self, key_id: i32) -> Result<(), KohakuError> {
        let expiry = Utc::now().naive_utc() + Duration::minutes(30);
        self.blacklist.write().await.insert(key_id, expiry);

        Ok(())
    }

    /// Checks if a specific API key is currently blacklisted.
    ///
    /// The function will call [JWTService::cleanup_expired] first, to clean up any expired listings.
    /// # Parameters
    /// - `key_id` : Identifier of the underlying [`ApiKey`] inside the database
    ///
    /// # Returns
    /// A [`bool`] which indicates if the stated API key is on the list or not
    pub async fn is_blacklisted(&self, key_id: i32) -> bool {
        self.cleanup_expired().await;
        let blklist = self.blacklist.read().await;

        blklist.contains_key(&key_id)
    }

    /// Cleans up the blacklist of expired revoked API keys.
    pub async fn cleanup_expired(&self) {
        let now = Utc::now().naive_utc();
        let mut blklist = self.blacklist.write().await;

        blklist.retain(|_, &mut expiry| expiry >= now);
    }
}

/// Initializes a globally unqiue and accessible [`JWTService`] instance.
///
/// # Parameters
/// - `encryption_key` : A [`String`]-based key for JWT encryption. Can be found in the configuration and is loaded as a env
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : [`JWTService`] is now accessible via [get_jwtservice]
/// - [`Err`] : A [KohakuError::InternalServerError] if the [`JWTService`] is already initialized
pub fn init_jwtservice(encryption_key: &[u8]) -> Result<(), KohakuError> {
    let service = Arc::new(JWTService::new(encryption_key));
    JWT_SERVICE.set(service).map_err(|_| {
        KohakuError::InternalServerError("JWTService already initialized".to_string())
    })?;
    Ok(())
}

/// Get current [`JWTService`] instance.
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : A [`Arc<JWTService>`] to gain access to the functionalities of the [`JWTService`]
/// - [`Err`] : A [KohakuError::InternalServerError] if the [`JWTService`] was not prior initialized via [`init_jwtservice`]
pub fn get_jwtservice() -> Result<Arc<JWTService>, KohakuError> {
    let service = JWT_SERVICE.get();
    if service.is_none() {
        return Err(KohakuError::InternalServerError(
            "JWTService not initialized - call init_jwtservice first!".to_string(),
        ));
    }
    Ok(service.unwrap().clone())
}
