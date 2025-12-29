use std::{collections::HashSet, time::Duration};

use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use regex::Regex;
use rstest::rstest;

use crate::utils::comm::auth::{
    api_key::{extract_prefix, generate_key, hash_key, random_string, verify_key, CHARSET},
    jwt::{get_jwtservice, init_jwtservice},
    models::{Claims, TokenType},
};

// ========================================= API Keys ========================================== //
// ================================= generate_key
#[test]
fn test_generate_key_format() {
    let (full_key, _) = generate_key();
    assert!(full_key.starts_with("khk_"));
    assert_eq!(full_key.split('_').collect::<Vec<_>>().len(), 3);
    assert_eq!(full_key.len(), 42);
}

#[test]
fn test_generate_key_prefix_fit_key() {
    let (full_key, prefix) = generate_key();
    assert!(full_key.starts_with(&prefix));
}

#[test]
fn test_generate_key_prefix_format() {
    let (_, prefix) = generate_key();
    assert!(prefix.starts_with("khk_"));
    assert_eq!(prefix.split('_').collect::<Vec<_>>().len(), 2);
    assert_eq!(prefix.len(), 10);
}

#[test]
fn test_generate_key_uniqueness() {
    let keys: Vec<String> = (0..100)
        .map(|_| {
            let (key, _) = generate_key();
            key
        })
        .collect();
    // Use HashSet to count unique keys in vector keys
    let unqiue_count = keys.iter().collect::<HashSet<&String>>().len();
    assert_eq!(unqiue_count, keys.len())
}

// ================================= random_string

#[test]
fn test_random_string_correct_length() {
    for i in 0..100 {
        let s = random_string(i);
        assert_eq!(s.len(), i);
    }
}

#[test]
fn test_random_string_guranteed_charset() {
    let allowed = regex::escape(std::str::from_utf8(CHARSET).unwrap());
    let pattern = format!("[^{}]", allowed);
    let re = Regex::new(&pattern).unwrap();

    for i in 0..100 {
        let s = random_string(i);
        assert!(!re.is_match(&s));
    }
}

#[test]
fn test_random_string_randomness() {
    // Collision probability of ~1.32x10^(-15) = 0.00000000000000132
    let rng_str: Vec<String> = (0..100).map(|_| random_string(10)).collect();
    // Use HashSet to count unique keys in vector keys
    let unqiue_count = rng_str.iter().collect::<HashSet<&String>>().len();
    assert_eq!(unqiue_count, rng_str.len())
}

#[test]
fn test_random_string_empty() {
    let s = random_string(0);
    assert_eq!(s, "");
}

// ================================= hash_key

#[test]
fn test_hash_successful() {
    let (key, _) = generate_key();
    let result = hash_key(&key);
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn test_hash_valid_argon2() {
    let (key, _) = generate_key();
    let hash = hash_key(&key).unwrap();
    assert!(hash.starts_with("$argon2"))
}

#[test]
fn test_hash_different_of_same_key() {
    let (key, _) = generate_key();
    let hash1 = hash_key(&key).unwrap();
    let hash2 = hash_key(&key).unwrap();
    assert_ne!(hash1, hash2);
}

#[test]
fn test_hash_different_to_key() {
    let (key, _) = generate_key();
    let hash = hash_key(&key).unwrap();
    assert_ne!(hash, key);
}

// ================================= verify_key

#[test]
fn test_verify_key_valid_pair() {
    let (key, _) = generate_key();
    let hash = hash_key(&key).unwrap();
    let val = verify_key(&key, &hash);
    assert!(val.is_ok());
    assert!(val.unwrap());
}

#[test]
fn test_verify_key_invalid_pair() {
    let (key1, _) = generate_key();
    let (key2, _) = generate_key();

    let hash1 = hash_key(&key1).unwrap();
    let hash2 = hash_key(&key2).unwrap();

    let val = verify_key(&key1, &hash2);
    assert!(val.is_ok());
    assert!(!val.unwrap());

    let val = verify_key(&key2, &hash1);
    assert!(val.is_ok());
    assert!(!val.unwrap());
}

#[test]
fn test_verify_key_corrupted() {
    let (key, _) = generate_key();
    let hash = hash_key(&key).unwrap();
    let malformed = hash.replace("$argon2", "$bygon2");

    let val = verify_key(&key, &malformed);
    assert!(val.is_err());
}

#[test]
fn test_verify_key_empty() {
    let empty_key = "";
    let (key, _) = generate_key();
    let empty_hash = "";
    let hash = hash_key(&key).unwrap();

    let val = verify_key(&empty_key, &hash);
    assert!(val.is_ok());
    assert!(!val.unwrap());

    let val = verify_key(&key, &empty_hash);
    assert!(val.is_err());
}

// ================================= extract_prefix

#[test]
fn test_extract_prefix_format() {
    let (key, prefix) = generate_key();
    let ext_prefix = extract_prefix(&key);
    assert!(ext_prefix.is_ok());
    let ep = ext_prefix.unwrap();
    assert_eq!(
        ep, prefix,
        "Extracted prefix : {}, Generated prefix: {}",
        ep, prefix
    );
}

#[rstest]
#[case("khk_too_many_under_scores_in_this_key")]
#[case("khk_toolittleunderscores")]
#[case("khknounderscores")]
fn test_extract_prefix_illegal_formats(#[case] input: &str) {
    let val = extract_prefix(input);
    assert!(val.is_err());
}

// =========================================== JWT ============================================= //
// ================================= JWTService::create_token

#[rstest]
#[case(-1, vec!["keys:manage"], TokenType::Bootstrap)]
#[case(22, vec!["events:subscribe"], TokenType::Access)]
#[case(487, vec!["events:subscribe", "tests:run"], TokenType::Refresh)]
#[case(3, vec!["events:subscribe"], TokenType::Refresh)]
#[case(127654, vec!["events:subscribe", "tests:run"], TokenType::Access)]
fn test_create_token_valid(
    #[case] key_id: i32,
    #[case] scopes: Vec<&str>,
    #[case] token_type: TokenType,
) {
    let key = "encryption_key".to_string();
    let owner = "test-suite".to_string();

    let _ = init_jwtservice(&key.as_bytes());
    let service = get_jwtservice().unwrap();
    let scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();

    let decoding_key = DecodingKey::from_secret(&key.as_bytes());
    let iat = Utc::now().timestamp() as usize;
    let duration = match token_type {
        TokenType::Bootstrap => 10 * 60,         // 10 Minutes
        TokenType::Access => 15 * 60,            // 15 Minutes
        TokenType::Refresh => 30 * 24 * 60 * 60, // 30 days
    };
    let exp = iat + duration;
    // Should succeed
    let val = service.create_token(owner.clone(), key_id, scopes.clone(), token_type.clone());
    assert!(val.is_ok());

    // Should be decodeable
    let validation = Validation::default();
    let dec = decode::<Claims>(val.unwrap(), &decoding_key, &validation);
    assert!(dec.is_ok());

    // Should be decodeable and return the expected values
    let cl = dec.unwrap().claims;
    assert_eq!(cl.key_id, key_id);
    assert_eq!(cl.owner, owner);
    assert_eq!(cl.scopes, scopes);
    assert_eq!(cl.token_type, token_type);

    assert!(cl.iat - iat < 2);
    assert!(cl.exp - exp < 2);
}

#[rstest]
// Wrong id
#[case(-1, vec!["events:subscribe"], TokenType::Access)]
#[case(-1, vec!["events:subscribe"], TokenType::Refresh)]
#[case(12, vec!["keys:manage"], TokenType::Bootstrap)]
// Wrong scope
#[case(0, vec!["keys:manage"], TokenType::Access)]
#[case(0, vec!["keys:manage"], TokenType::Refresh)]
#[case(-1, vec!["events:subscribe"], TokenType::Bootstrap)]
#[case(0, vec!["keys:manage", "events:subscribe"], TokenType::Access)]
#[case(0, vec!["keys:manage", "events:subscribe"], TokenType::Refresh)]
#[case(-1, vec!["keys:manage", "events:subscribe"], TokenType::Bootstrap)]
// Invalid id
#[case(-2, vec!["keys:manage"], TokenType::Bootstrap)]
#[case(-5, vec!["events:subscribe"], TokenType::Access)]
#[case(-10, vec!["events:subscribe"], TokenType::Refresh)]
fn test_create_token_invalid(
    #[case] key_id: i32,
    #[case] scopes: Vec<&str>,
    #[case] token_type: TokenType,
) {
    let key = "encryption_key".to_string();
    let owner = "test-suite".to_string();

    let _ = init_jwtservice(&key.as_bytes());
    let service = get_jwtservice().unwrap();
    let scopes = scopes.iter().map(|s| s.to_string()).collect();

    // Should fail
    let val = service.create_token(owner, key_id, scopes, token_type);
    assert!(val.is_err());
}

// ================================= JWTService::validate_token
#[rstest]
#[case(0, vec!["events:subscribe"], TokenType::Access, 15 * 60)]
#[case(20, vec!["events:subscribe"], TokenType::Refresh, 30 * 24 * 60 * 60)]
#[case(-1, vec!["keys:manage"], TokenType::Bootstrap, 10 * 60)]
fn test_validate_token_valid(
    #[case] key_id: i32,
    #[case] scopes: Vec<&str>,
    #[case] token_type: TokenType,
    #[case] duration: usize,
) {
    let iat = Utc::now().timestamp() as usize;
    let exp = iat + duration;
    let claims = Claims {
        owner: "test-suite".to_string(),
        key_id,
        scopes: scopes.iter().map(|s| s.to_string()).collect(),
        token_type,
        exp,
        iat,
    };

    let key = "encryption_key".to_string();
    let encoding_key = EncodingKey::from_secret(&key.as_bytes());
    let _ = init_jwtservice(&key.as_bytes());
    let service = get_jwtservice().unwrap();
    let token = encode(&Header::default(), &claims, &encoding_key).unwrap();

    // Check if validating returns same claim
    let val = service.validate_token(&token);
    assert!(val.is_ok());

    let cl = val.unwrap();
    assert_eq!(cl, claims);
}

#[rstest]
#[case(0, vec!["events:subscribe"], TokenType::Access, 15 * 60)]
#[case(20, vec!["events:subscribe"], TokenType::Refresh, 30 * 24 * 60 * 60)]
#[case(-1, vec!["keys:manage"], TokenType::Bootstrap, 10 * 60)]
fn test_validate_token_invalid(
    #[case] key_id: i32,
    #[case] scopes: Vec<&str>,
    #[case] token_type: TokenType,
    #[case] duration: usize,
) {
    let iat = Utc::now().timestamp() as usize;
    let exp = iat + duration;
    let claims = Claims {
        owner: "test-suite".to_string(),
        key_id,
        scopes: scopes.iter().map(|s| s.to_string()).collect(),
        token_type,
        exp,
        iat,
    };

    let key1 = "encryption_key".to_string();
    let key2 = "another_encryption_key".to_string();
    let encoding_key = EncodingKey::from_secret(&key2.as_bytes());
    let _ = init_jwtservice(&key1.as_bytes());
    let service = get_jwtservice().unwrap();
    let token = encode(&Header::default(), &claims, &encoding_key).unwrap();

    // Shouldn't perse fail but the resulting claim should be wrong
    let val = service.validate_token(&token);
    assert!(val.is_err());
}
// ================================= JWTService::blacklist_key
#[tokio::test]
async fn test_blacklist_key() {
    let key_id = 12;

    let key = "encryption_key".to_string();
    let _ = init_jwtservice(&key.as_bytes());
    let service = get_jwtservice().unwrap();

    assert!(service.read_blacklist().await.is_empty());

    let val = service.blacklist_key(key_id, None).await;
    assert!(val.is_ok());
    let blklist = service.read_blacklist().await;
    assert_eq!(blklist.len(), 1);

    // As it is a HashMap it should not increase
    let val = service.blacklist_key(key_id, None).await;
    assert!(val.is_ok());
    let blklist = service.read_blacklist().await;
    assert_eq!(blklist.len(), 1);
}

// ================================= JWTService::is_blacklisted

#[tokio::test]
async fn test_is_blacklisted() {
    let key_id = 13;
    let key_id_no = 455;

    let key = "encryption_key".to_string();
    let _ = init_jwtservice(&key.as_bytes());
    let service = get_jwtservice().unwrap();

    // Not prior blacklisted
    assert!(!service.is_blacklisted(key_id).await);
    assert!(!service.is_blacklisted(key_id_no).await);

    let _ = service.blacklist_key(key_id, Some(2)).await;

    // One is now blacklisted
    assert!(service.is_blacklisted(key_id).await);
    assert!(!service.is_blacklisted(key_id_no).await);

    // Wait for expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Now both should not be blacklisted
    assert!(!service.is_blacklisted(key_id).await);
    assert!(!service.is_blacklisted(key_id_no).await);
}
