use chrono::NaiveDateTime;
use diesel::{prelude::*, query_dsl::methods::FilterDsl};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        self, get_connection,
        schema::{self},
    },
    utils::error::KohakuError,
};

// =========================================== API ============================================= //

#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub owner: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateKeyResponse {
    pub api_key: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeKeyRequest {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

// ========================================= API Keys ========================================== //

/// Representation of database entry of a given ApiKey
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::api_keys)]
pub struct ApiKey {
    /// Serial Primary Key given by the database
    pub id: i32,
    /// Hashed presentation of the actual full key
    pub hashed_key: String,
    /// 10-char long prefix of the actual full key
    pub key_prefix: String,
    /// Identifier which service / user uses this key
    pub owner: String,
    /// Permission scopes given in a `category:verb` manner
    pub scopes: Vec<String>,
    /// Timestamp of creation (Default: Current Time UTC)
    pub created_at: NaiveDateTime,
}

/// Form to create a new [struct@ApiKey].
#[derive(Debug, Insertable, Clone)]
#[diesel(table_name = crate::db::schema::api_keys)]
pub struct NewApiKey {
    pub hashed_key: String,
    pub key_prefix: String,
    pub owner: String,
    pub scopes: Vec<String>,
}

/// Creates an entry for the API key in the database
///
/// # Parameters
/// - `hashed_key` : Hashed [`String`] presentation of the actual full key
/// - `key_prefix` : 10-char long [`String`] prefix of the actual full key
/// - `owner` : [`String`] identifier of the service or user that uses this API key
/// - `scopes`: Vector of [`String`]s that map the actual permissions in a `category:verb` manner
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : A [struct@ApiKey] that represents the now stored API key in the database.
/// - [`Err`] : A [enum@KohakuError] based on the failing operation.
///
pub async fn create_apikey(
    hashed_key: String,
    key_prefix: String,
    owner: String,
    scopes: Vec<String>,
) -> Result<ApiKey, KohakuError> {
    for scp in &scopes {
        if scp.starts_with("keys") {
            return Err(KohakuError::ValidationError("Illegal Argument: Any scope of the category `key` is not allowed for general API keys!".to_string()));
        }
    }

    let mut conn = get_connection()?;

    let new_key = NewApiKey {
        hashed_key,
        key_prefix,
        owner,
        scopes: scopes.clone(),
    };

    diesel::insert_into(schema::api_keys::table)
        .values(&new_key)
        .get_result(&mut conn)
        .map_err(KohakuError::DatabaseError)
}

/// Gets an entry for an identifieable API key in the database
///
/// `id` will be one either 0 or 1 entry, while `key_prefix` is not unique and therefore can result in n entries.
/// # Parameters
/// - `id_` : Serial primary key of the database. Either this or `key_prefix` must be set
/// - `key_prefix_` : 10-char long [`String`] prefix of the actual full key. Either this or `id` must be set
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : The identified [struct@ApiKey]s that matches either `id` and/or `key_prefix` inside a vector
/// - [`Err`] : A [enum@KohakuError] based on the failing operation
pub async fn get_apikey(
    id_: Option<i32>,
    key_prefix_: Option<String>,
) -> Result<Vec<ApiKey>, KohakuError> {
    use db::schema::api_keys::dsl::*;
    if id_.is_none() && key_prefix_.is_none() {
        return Err(KohakuError::ValidationError("Illegal Argument: At least one of the parameters - `id` and/or `key_prefix` must be set!".to_string()));
    }
    let mut conn = get_connection()?;
    let mut query = api_keys.into_boxed();

    if let Some(i) = id_ {
        query = FilterDsl::filter(query, id.eq(i));
    }

    if let Some(kp) = key_prefix_ {
        query = FilterDsl::filter(query, key_prefix.eq(kp));
    }

    query.load(&mut conn).map_err(KohakuError::DatabaseError)
}

/// Removes an entry representing an API key from the database
///
/// # Parameters
/// - `id_` : Serial primary key of the database. Either this or `key_prefix` must be set
/// - `key_prefix_` : 10-char long [`String`] prefix of the actual full key. Either this or `id` must be set
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : The API key was deleted from the database
/// - [`Err`] : A [enum@KohakuError] based on the failing operation
pub async fn delete_apikey(
    id_: Option<i32>,
    key_prefix_: Option<String>,
) -> Result<(), KohakuError> {
    use db::schema::api_keys::dsl::*;
    if id_.is_none() && key_prefix_.is_none() {
        return Err(KohakuError::ValidationError("Illegal Argument: At least one of the parameters - `id` and/or `key_prefix` must be set!".to_string()));
    }
    let mut conn = get_connection()?;
    let mut query = diesel::delete(api_keys).into_boxed();

    if let Some(i) = id_ {
        query = FilterDsl::filter(query, id.eq(i));
    }

    if let Some(kp) = key_prefix_ {
        query = FilterDsl::filter(query, key_prefix.eq(kp));
    }

    query
        .execute(&mut conn)
        .map_err(KohakuError::DatabaseError)?;
    Ok(())
}

// =========================================== JWT ============================================= //

/// JsonWebToken Type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    // Key management JWT
    Bootstrap,
    // Short-lived (15 min)
    Access,
    // Long-lived (30 days)
    Refresh,
}

/// JsonWebToken Claim
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Claims {
    /// Identifier which service / user uses this key
    pub owner: String,
    /// Id of corresponding [struct@ApiKey]
    pub key_id: i32,
    /// Scopes (same as [struct@ApiKey])
    pub scopes: Vec<String>,
    /// Bootstrap, Access or Refresh
    pub token_type: TokenType,
    /// Expiration Timestamp
    pub exp: usize,
    /// Issued-at Timestamp
    pub iat: usize,
}

/// Response of creating a (pair of) token(s)
#[derive(Debug, Serialize, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    /// Expiration in seconds
    pub expires_in: usize,
}
