use std::collections::HashSet;

use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use rand::Rng;
use regex::Regex;

use crate::utils::comm::auth::api_key::{
    extract_prefix, generate_key, hash_key, random_string, verify_key, CHARSET,
};

fn random_number(range_start: usize, range_end: usize) -> usize {
    let mut rng = rand::rng();
    rng.random_range(range_start..range_end)
}

// ==================================================================

#[test]
fn test_generate_key_key_length() {
    let (full_key, _) = generate_key();
    assert_eq!(full_key.len(), 42, "Generated key shows invalid format");
}

#[test]
fn test_generate_key_prefix_length() {
    let (_, prefix) = generate_key();
    assert_eq!(prefix.len(), 10, "Generated prefix shows invalid format");
}

#[test]
fn test_generate_key_key_starts_with_prefix() {
    let (full_key, prefix) = generate_key();
    assert!(
        full_key.starts_with(&prefix),
        "Generated key does not start with prefix"
    );
}

#[test]
fn test_generate_key_underscore_count() {
    let (full_key, _) = generate_key();
    assert_eq!(
        full_key.split('_').collect::<Vec<_>>().len(),
        3,
        "Wrong amount of underscores detected"
    );
}

#[test]
fn test_generate_key_start_with_khk() {
    let (full_key, _) = generate_key();
    assert!(full_key.starts_with("khk"), "Key must start with 'khk'");
}

#[test]
fn test_generate_key_uniqueness() {
    let strings: Vec<String> = (0..100)
        .map(|_| {
            let (key, _) = generate_key();
            key
        })
        .collect();

    let unique_count = strings.iter().collect::<HashSet<&String>>().len();
    assert_eq!(unique_count, 100, "Duplicated keys found");
}

// ==================================================================

#[test]
fn test_random_string_length() {
    let l = random_number(1, 100);
    let s = random_string(l);
    assert_eq!(
        s.len(),
        l,
        "Random string has different length than requested"
    );
}

#[test]
fn test_random_string_charset() {
    let allowed = regex::escape(std::str::from_utf8(CHARSET).unwrap());
    let pattern = format!("[^{}]", allowed);
    let re = Regex::new(&pattern).unwrap();

    let l = random_number(1, 100);
    let s = random_string(l);

    assert!(
        !re.is_match(&s),
        "Random string shows chars outside of given charset"
    );
}

#[test]
fn test_random_string_empty() {
    let s = random_string(0);
    assert_eq!(
        s.len(),
        0,
        "Random string has different length than requested"
    );
    assert_eq!(s, "", "Random string with length 0 is not empty");
}

#[test]
fn test_random_string_uniqueness() {
    let l = random_number(1, 100);
    let strings: Vec<String> = (0..100).map(|_| random_string(l)).collect();

    let unique_count = strings.iter().collect::<HashSet<&String>>().len();
    assert_eq!(unique_count, 100, "Duplicated strings found");
}

// ==================================================================
#[test]
fn test_hash_key_returns_ok() {
    let l = random_number(1, 100);
    let string = random_string(l);

    assert!(hash_key(&string).is_ok(), "Hashing string returns error");
}

#[test]
fn test_hash_key_verifiable() {
    let l = random_number(1, 100);
    let string = random_string(l);

    let hash = hash_key(&string).expect("Hashing string returns error");

    let parsed_hash = PasswordHash::new(&hash).expect("Parsing hash for verification failed");
    let argon2 = Argon2::default();
    let res = argon2.verify_password(string.as_bytes(), &parsed_hash);

    assert!(res.is_ok(), "Hash does not match to original string");
}

#[test]
fn test_hash_key_not_original_key() {
    let l = random_number(1, 100);
    let string = random_string(l);

    let hash = hash_key(&string).expect("Hashing string returns error");
    assert_ne!(string, hash, "Hash is identical to original string")
}

#[test]
fn test_hash_key_uniqueness() {
    // Reduced amount of hashes to ensure faster test runs
    // 100 Hashes resulted in 30 secs runtime due to argon2!
    let l = random_number(1, 100);
    let string = random_string(l);
    let hashes: Vec<String> = (0..5)
        .map(|_| hash_key(&string).expect("Hashing string returns error"))
        .collect();

    let unique_count = hashes.iter().collect::<HashSet<&String>>().len();
    assert_eq!(unique_count, 5, "Duplicated hashes found");
}

// ==================================================================

#[test]
fn test_verify_key_valid() {
    let l = random_number(1, 100);
    let string = random_string(l);

    let hash = hash_key(&string).expect("Hashing string returns error");
    let res = verify_key(&string, &hash);
    assert!(
        res.is_ok(),
        "Verification process failed. Internal argon2 error."
    );
    assert!(
        res.unwrap(),
        "Verification failed: Correct key, hash pair is not verified correctly"
    );
}

#[test]
fn test_verify_key_invalid() {
    let l = random_number(1, 100);
    let string = random_string(l);
    let string2 = random_string(l);

    let hash = hash_key(&string).expect("Hashing string returns error");
    let hash2 = hash_key(&string2).expect("Hashing string returns error");

    let res1 = verify_key(&string, &hash2);
    let res2 = verify_key(&string2, &hash);

    assert!(
        res1.is_ok(),
        "Verification process for first invalid pair failed. Internal argon2 error"
    );
    assert!(
        res2.is_ok(),
        "Verification process for second invalid pair failed. Internal argon2 error"
    );

    assert!(!res1.unwrap(), "First invalid pair detected as valid");
    assert!(!res2.unwrap(), "Second invalid pair detected as valid");
}

#[test]
fn test_verify_key_garbage() {
    let l = random_number(1, 100);
    let string = random_string(l);

    let garbage1 = "";
    let garbage2 = string.clone();

    let res1 = verify_key(&string, &garbage1);
    let res2 = verify_key(&string, &garbage2);

    assert!(
        res1.is_err(),
        "Verification with empty string (garbage1) was successful"
    );
    assert!(
        res2.is_err(),
        "Verification with original string (garbage2) was successful"
    );
}

#[test]
fn test_verify_key_empty_against_real() {
    let l = random_number(1, 100);
    let string = random_string(l);
    let hash = hash_key(&string).expect("Hashing string returns error");

    let res = verify_key("", &hash);
    assert!(
        res.is_ok(),
        "Verification process failed. Internal argon2 error"
    );
    assert!(
        !res.unwrap(),
        "Invalid pair (empty string, real hash) was verified successfully"
    );
}

// ==================================================================

#[test]
fn test_extract_prefix_valid() {
    let (full_key, _) = generate_key();
    let ext_prefix = extract_prefix(&full_key);
    assert!(ext_prefix.is_ok(), "Extraction returned error");
}

#[test]
fn test_extract_prefix_starts_with_khk() {
    let (full_key, _) = generate_key();
    let ext_prefix = extract_prefix(&full_key).expect("Extraction returned error");
    assert!(
        ext_prefix.starts_with("khk"),
        "Extracted prefix shows invalid format"
    );
}

#[test]
fn test_extract_prefix_match_generate_key() {
    let (full_key, prefix) = generate_key();
    let ext_prefix = extract_prefix(&full_key).expect("Extraction returned error");
    assert_eq!(
        ext_prefix, prefix,
        "Extracted prefix is not equal generated prefix"
    );
}

#[test]
fn test_extract_prefix_less_underscores() {
    let key = "khk_abc";
    let ext_prefix = extract_prefix(&key);
    assert!(
        ext_prefix.is_err(),
        "Extraction process succeeded for invalid formatted key"
    );
}

#[test]
fn test_extract_prefix_more_underscores() {
    let key = "khk_abc_def_ghi";
    let ext_prefix = extract_prefix(&key);
    assert!(
        ext_prefix.is_err(),
        "Extraction process succeeded for invalid formatted key"
    );
}

#[test]
fn test_extract_prefix_empty() {
    let key = "";
    let ext_prefix = extract_prefix(&key);
    assert!(
        ext_prefix.is_err(),
        "Extraction process succeeded for empty string"
    );
}
