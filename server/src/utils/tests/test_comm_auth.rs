use std::sync::Arc;

use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use regex::Regex;

use crate::utils::comm::auth::{
    api_key::{extract_prefix, generate_key, hash_key, random_string, verify_key, CHARSET},
    jwt::{get_jwtservice, init_jwtservice, JWTService},
    models::Claims,
};

// ========================================= API Keys ========================================== //

#[test]
fn test_apikey_generate_key() {
    let (full_key, prefix) = generate_key();

    assert_eq!(
        full_key.len(),
        42,
        "Full keys length was expected to be 42 chars long, but is {}",
        full_key.len()
    );
    assert_eq!(
        prefix.len(),
        10,
        "Prefix lentgh was expected to be 10 chars long, but is {}",
        prefix.len()
    );
    assert!(
        full_key.starts_with(&prefix),
        "Full key is suppose to start with the generated prefix, but does not."
    );
    assert!(
        prefix.starts_with("khk_"),
        "Prefix is suppose to start with `khk_`, but does not"
    );

    let splits = full_key.split('_').collect::<Vec<_>>().len();
    assert_eq!(
        splits, 3,
        "Full key should have two `_` to split into three parts! Can be split into {} parts",
        splits
    );
}

#[test]
fn test_apikey_random_string() {
    let mut rng = rand::rng();
    let len1 = rng.random_range(2..100);
    let len2 = rng.random_range(2..100);

    let str1 = random_string(len1);
    let str2 = random_string(len1);
    let str3 = random_string(len2);

    assert_eq!(
        str1.len(),
        len1,
        "Random string should have the given length of {} but is {} chars long [#1]",
        len1,
        str1.len()
    );
    assert_eq!(
        str2.len(),
        len1,
        "Random string should have the given length of {} but is {} chars long [#2]",
        len1,
        str2.len()
    );
    assert_eq!(
        str3.len(),
        len2,
        "Random string should have the given length of {} but is {} chars long [#3]",
        len2,
        str3.len()
    );

    // Chance of them actual being identical is miniscule and goes against 0% (Len1 = 2 => ~0.02%)
    assert_ne!(str1, str2);

    // These would not even be less likely because of the different length parameter
    assert_ne!(str2, str3);
    assert_ne!(str1, str3);

    // Check charset
    let allowed = regex::escape(std::str::from_utf8(CHARSET).unwrap());
    let pattern = format!("[^{}]", allowed);
    let re = Regex::new(&pattern).unwrap();

    assert!(
        !re.is_match(&str1),
        "Random string has chars outside of available CHARSET [#1]"
    );
    assert!(
        !re.is_match(&str2),
        "Random string has chars outside of available CHARSET [#2]"
    );
    assert!(
        !re.is_match(&str3),
        "Random string has chars outside of available CHARSET [#3]"
    );
}

#[test]
fn test_apikey_verify_key() {
    let (key, _) = generate_key();
    let hash = hash_key(&key);
    assert!(hash.is_ok(), "Hash generation failed! [#1]");
    let h = hash.unwrap();

    let (key2, _) = generate_key();
    let hash2 = hash_key(&key2);
    assert!(hash2.is_ok(), "Hash generation failed! [#2]");
    let h2 = hash2.unwrap();

    // Case #1: Correct key hash pairs
    let val = verify_key(&key, &h);
    assert!(val.is_ok());
    assert!(val.unwrap(), "Correct key-hash pair was not verified [#1]");

    let val = verify_key(&key2, &h2);
    assert!(val.is_ok());
    assert!(val.unwrap(), "Correct key-hash pair was not verified [#2]");

    // Case #2: Incorrect key hash pairs
    let val = verify_key(&key, &h2);
    assert!(val.is_ok());
    assert!(!val.unwrap(), "Incorrect key-hash pair was verified [#1]");

    let val = verify_key(&key2, &h);
    assert!(val.is_ok());
    assert!(!val.unwrap(), "Incorrect key-hash pair was verified [#2]");
}

#[test]
fn test_apikey_extract_prefix() {
    let (key, prefix) = generate_key();
    let ext_prefix = extract_prefix(&key);

    assert_eq!(prefix, ext_prefix, "Extracted prefix is not identical to generated prefix of same key. Extracted = {}, Generated = {}", ext_prefix, prefix);
}

// =========================================== JWT ============================================= //
fn setup_service() -> (JWTService, EncodingKey, DecodingKey) {
    // Generate a random encryption key
    let s = random_string(100);
    let key: &[u8] = s.as_bytes();

    let encoding_key = EncodingKey::from_secret(key);
    let decoding_key = DecodingKey::from_secret(key);

    let service = JWTService::new(key);

    (service, encoding_key, decoding_key)
}

#[test]
fn test_jwt_create_bootstrap_token() {
    let (service, encoding_key, decoding_key) = setup_service();

    let now = Utc::now().timestamp() as usize;
    let exp = now + 10 * 60;
    let token = service.create_bootstrap_token();
    assert!(token.is_ok());

    // Check if decoding results in predefined claim
    let t = token.unwrap();
    let t2 = t.clone();
    let validation = Validation::default();
    let claims = decode::<Claims>(t, &decoding_key, &validation);

    assert!(claims.is_ok());
    let cl = claims.unwrap().claims;
    assert_eq!(cl.owner, "system".to_string());
    assert_eq!(cl.key_id, -1);
    assert_eq!(cl.scopes, vec!["keys:manage".to_string()]);
    // now < iat, exp < cl.exp
    assert!(cl.iat - now <= 1);
    assert!(cl.exp - exp <= 1);

    // Check if encoding the same claim with the same key gets the same token
    let tkn = encode(&Header::default(), &cl, &encoding_key);
    assert!(tkn.is_ok());
    assert_eq!(tkn.unwrap(), t2);
}

#[test]
fn test_jwt_create_tokens() {
    let (service, encoding_key, decoding_key) = setup_service();

    let id = 0;
    let owner = "test-suite";
    let scopes = vec!["test:test".to_string()];
    let scopes_ = scopes.clone();

    let now = Utc::now().timestamp() as usize;

    let tokens = service.create_tokens(id, &owner, scopes);
    assert!(tokens.is_ok());
    let tokens_ = tokens.unwrap().clone();

    // Check if decoding results in predefined claim
    let access_token = tokens_.access_token;
    let at2 = access_token.clone();
    let refresh_token = tokens_.refresh_token.unwrap();
    let rf2 = refresh_token.clone();
    let validation = Validation::default();

    let at_claims = decode(access_token, &decoding_key, &validation);
    assert!(at_claims.is_ok());
    let at_claim: Claims = at_claims.unwrap().claims;
    assert_eq!(at_claim.owner, owner.to_string());
    assert_eq!(at_claim.key_id, id);
    assert_eq!(at_claim.scopes, scopes_);

    // + 15 min
    let exp = now + 15 * 60;
    assert!(at_claim.iat - now <= 1);
    assert!(at_claim.exp - exp <= 1);

    let rf_claims = decode(refresh_token, &decoding_key, &validation);
    assert!(rf_claims.is_ok());
    let rf_claim: Claims = rf_claims.unwrap().claims;
    assert_eq!(rf_claim.owner, owner.to_string());
    assert_eq!(rf_claim.key_id, id);
    assert_eq!(rf_claim.scopes, scopes_);

    // + 30 days
    let exp = now + 30 * 24 * 60 * 60;
    assert!(rf_claim.iat - now <= 1);
    assert!(rf_claim.exp - exp <= 1);

    // Check if encoding the same claim with same key gets the same token
    let at_tkn = encode(&Header::default(), &at_claim, &encoding_key);
    assert!(at_tkn.is_ok());
    assert_eq!(at_tkn.unwrap(), at2);

    let rf_tkn = encode(&Header::default(), &rf_claim, &encoding_key);
    assert!(rf_tkn.is_ok());
    assert_eq!(rf_tkn.unwrap(), rf2);
}

#[test]
fn test_jwt_validate_token() {
    let (service, _, _) = setup_service();
    let id = 0;
    let owner = "test-suite";
    let scopes = vec!["test:test".to_string()];
    let scopes_ = scopes.clone();

    let tokens = service.create_tokens(id, owner, scopes);
    assert!(
        tokens.is_ok(),
        "Token creation failed: {}",
        tokens.err().unwrap()
    );
    let tokens_ = tokens.unwrap().clone();

    let access_token = &tokens_.access_token;
    let refresh_token = &tokens_.refresh_token.unwrap();

    let val = service.validate_token(&access_token);
    assert!(
        val.is_ok(),
        "Validation for access token failed: {}",
        val.err().unwrap()
    );
    let cl = val.unwrap();
    assert_eq!(cl.key_id, id);
    assert_eq!(cl.owner, owner);
    assert_eq!(cl.scopes, scopes_);

    let val = service.validate_token(&refresh_token);
    assert!(
        val.is_ok(),
        "Validation for refresh token failed: {}",
        val.err().unwrap()
    );
    let cl = val.unwrap();
    assert_eq!(cl.key_id, id);
    assert_eq!(cl.owner, owner);
    assert_eq!(cl.scopes, scopes_);
}

#[test]
fn test_jwt_service_not_initialized_before_get() {
    assert!(get_jwtservice().is_err())
}

#[test]
fn test_jwt_service_singleton() {
    let s = random_string(100);
    let key = s.as_bytes();

    // #1 Test that first intialization succeeds and second fails
    assert!(init_jwtservice(key).is_ok());
    assert!(init_jwtservice(key).is_err());

    // #2 Test if getter returns same instance
    let s1 = get_jwtservice();
    let s2 = get_jwtservice();
    assert!(s1.is_ok());
    assert!(s2.is_ok());
    assert!(Arc::ptr_eq(&s1.unwrap(), &s2.unwrap()));
}
