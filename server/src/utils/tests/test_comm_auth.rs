use rand::Rng;
use regex::Regex;

use crate::utils::comm::auth::api_key::{
    extract_prefix, generate_key, hash_key, random_string, verify_key, CHARSET,
};

#[test]
pub fn test_apikey_generate_key() {
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
pub fn test_apikey_random_string() {
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
pub fn test_apikey_verify_key() {
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
pub fn test_apikey_extract_prefix() {
    let (key, prefix) = generate_key();
    let ext_prefix = extract_prefix(&key);

    assert_eq!(prefix, ext_prefix, "Extracted prefix is not identical to generated prefix of same key. Extracted = {}, Generated = {}", ext_prefix, prefix);
}
