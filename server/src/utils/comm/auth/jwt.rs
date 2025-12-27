use chrono::{Duration, NaiveDateTime, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[allow(unused_imports)] // ApiKey is linked in the documentation
use crate::utils::{
    comm::auth::models::{ApiKey, Claims, TokenResponse, TokenType},
    config::get_config,
    error::KohakuError,
};

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

    /// Creates JWT for the bootstrap key.
    ///
    /// # Returns
    /// A [`Result`] which is either
    /// - [`Ok`] : [`String`] representation of JWT [`Claims`]
    /// - [`Err`]: A [KohakuError::ValidationError] when the encoding fails
    pub fn create_bootstrap_token(&self) -> Result<String, KohakuError> {
        let now = Utc::now().timestamp() as usize;
        let exp = now + 10 * 60;

        let claims = Claims {
            owner: "system".to_lowercase().to_string(),
            key_id: -1,
            scopes: vec!["keys:manage".to_string()],
            token_type: TokenType::Bootstrap,
            exp,
            iat: now,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| KohakuError::ValidationError(e.to_string()))
    }

    /// Creates JWT for general API keys.
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
        if scopes.contains(&"key:manage".to_string()) {
            return Err(KohakuError::Unauthorized("No general tokens with the scope `keys:manage` can be created! Please refer to the bootstrap key!".to_string()));
        }
        let now = Utc::now().timestamp() as usize;

        // Access Token (15 min)
        let access_exp = now + 15 * 60;
        let access_claims = Claims {
            owner: owner.to_string(),
            key_id,
            scopes: scopes.clone(),
            token_type: TokenType::Access,
            exp: access_exp,
            iat: now,
        };
        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|e| KohakuError::InternalServerError(e.to_string()))?;

        // Refresh Token (30 days)
        let refresh_exp = now + 30 * 24 * 60 * 60;
        let refresh_claims = Claims {
            owner: owner.to_string(),
            key_id,
            scopes: scopes.clone(),
            token_type: TokenType::Refresh,
            exp: refresh_exp,
            iat: now,
        };
        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|e| KohakuError::InternalServerError(e.to_string()))?;

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
