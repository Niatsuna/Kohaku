use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::Rng;

use crate::utils::error::KohakuError;

/// Available chars for random string generation
pub const CHARSET: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&*+-/=";

/// Generates an API key and returns the full key as well as the 10-char long prefix of said key.
///
/// # Returns
/// A pair ([`String`], [`String`]) where the first string is the `full_key`, with a length of 42 chars,
/// and the second string is the `prefix`, with a length of 10 chars.
///
/// # Examples
/// ```rust
/// let (key, prefix) = generate_key();
///
/// assert!(key.starts_with(&prefix));
/// ```
pub fn generate_key() -> (String, String) {
    let prefix = format!("khk_{}", random_string(6));
    let secret = random_string(31);

    let full_key = format!("{}_{}", prefix, secret);
    (full_key, prefix)
}

/// Generate a random string of given size and based on the available [`CHARSET`].
///
/// # Parameters
/// - `length` : A `usize`d measurement how many chars the result should have
///
/// # Returns
/// (Pseudo) randomized [`String`] of size `length` with the alphabet [`CHARSET`]
///
/// # Examples
/// ```rust
/// let s = random_string(5);
///
/// assert_eq!(random_string.len(), 5);
/// ```
pub fn random_string(length: usize) -> String {
    let mut rng = rand::rng();

    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Hashes the given key using [`Argon2`].
///
/// # Parameters
/// - `key` : Prior generated API key
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : A hashed [`String`] variant of the given API key
/// - [`Err`] : A [KohakuError::InternalServerError] if [`Argon2`] failed to hash the given API key
///
/// # Examples
/// ```rust
/// let (key, _) = generate_key();
/// let hash = hash_key(&key)?;
/// ```
pub fn hash_key(key: &str) -> Result<String, KohakuError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(key.as_bytes(), &salt)
        .map_err(|e| KohakuError::InternalServerError(e.to_string()))?;
    Ok(hash.to_string())
}

/// Verifies if the given API key matches the given hashed variant using [`Argon2`].
///
/// # Parameters
/// - `key` : Prior generated API key
/// - `hash` : Hashed [`String`] variant of an API key
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : A [`bool`] indicating if the given `key` and `hash` match
/// - [`Err`] : A [`KohakuError::InternalServerError`] if [`Argon2`] failed internally to verify the matching
///
/// # Examples
/// ```rust
/// let (key, _) = generate_key();
/// let hash = hash_key(&key)?;
///
/// let ver = verify_key(&key, &hash);
/// assert!(ver.is_ok())
/// assert!(ver.unwrap())
/// ```
pub fn verify_key(key: &str, hash: &str) -> Result<bool, KohakuError> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| KohakuError::InternalServerError(e.to_string()))?;
    let argon2 = Argon2::default();

    match argon2.verify_password(key.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(KohakuError::InternalServerError(e.to_string())),
    }
}

/// Extracts the prefix from a given API Key.
///
/// The prefix ends at the second `_` char.
///
/// # Parameters
/// - `key` : Prior generated API key
///
/// # Returns
/// A [`String`] up until the second `_` (Default : 10-char long)
pub fn extract_prefix(key: &str) -> String {
    key.split('_').take(2).collect::<Vec<_>>().join("_")
}
