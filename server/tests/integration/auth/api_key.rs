use rstest::rstest;

use kohaku::utils::comm::auth::{
    api_key::{generate_key, hash_key, random_string},
    models::{create_apikey, delete_apikey, get_apikey, ApiKey},
};

use crate::helper::{
    db::{seed_api_key, seed_api_key_given, with_test_db},
    utils::vec_str_to_string,
};

#[tokio::test]
async fn test_create_apikey_valid() {
    with_test_db(|conn| {
        Box::pin(async move {
            let (full_key, prefix) = generate_key();
            let hash = hash_key(&full_key).expect("Hashing key returns error");
            let owner = "test-suite";
            let scopes: Vec<String> = vec_str_to_string(vec!["something:valid"]);

            let res = create_apikey(
                conn,
                hash.to_string(),
                prefix.clone(),
                owner.to_string(),
                scopes.clone(),
            )
            .await;

            assert!(res.is_ok());
            let stored = res.unwrap();
            assert_eq!(stored.hashed_key, hash);
            assert_eq!(stored.key_prefix, prefix);
            assert_eq!(stored.scopes, scopes);
            assert_eq!(stored.owner, owner);
        })
    })
    .await;
}

#[tokio::test]
#[rstest]
#[case(vec_str_to_string(vec!["keys:manage"]))]
#[case(vec_str_to_string(vec!["keys:"]))]
#[case(vec_str_to_string(vec!["something:valid", "keys:manage"]))]
#[case(vec_str_to_string(vec!["something:valid", "keys:", "something:valid_again"]))]
async fn test_create_apikey_invalid_scope(#[case] scopes: Vec<String>) {
    with_test_db(|conn| {
        Box::pin(async move {
            let (full_key, prefix) = generate_key();
            let hash = hash_key(&full_key).expect("Hashing key returns error");
            let owner = "test-suite";

            let res = create_apikey(conn, hash, prefix, owner.to_string(), scopes).await;
            assert!(res.is_err());
        })
    })
    .await;
}

#[tokio::test]
async fn test_create_apikey_empty_scope() {
    with_test_db(|conn| {
        Box::pin(async move {
            let (full_key, prefix) = generate_key();
            let hash = hash_key(&full_key).expect("Hashing key returns error");
            let owner = "test-suite";
            let scopes: Vec<String> = vec![];

            let res = create_apikey(
                conn,
                hash.to_string(),
                prefix.clone(),
                owner.to_string(),
                scopes.clone(),
            )
            .await;

            assert!(res.is_ok());
            let stored = res.unwrap();
            assert_eq!(stored.hashed_key, hash);
            assert_eq!(stored.key_prefix, prefix);
            assert_eq!(stored.scopes, scopes);
            assert_eq!(stored.owner, owner);
        })
    })
    .await;
}

// ==================================================================

#[tokio::test]
async fn test_get_apikey_invalid_params() {
    with_test_db(|conn| {
        Box::pin(async move {
            let res = get_apikey(conn, None, None).await;
            assert!(res.is_err());
        })
    })
    .await;
}

#[tokio::test]
async fn test_get_apikey_valid_id() {
    with_test_db(|conn| {
        Box::pin(async move {
            let data = seed_api_key(conn, "test-suite", vec![]);
            let res = get_apikey(conn, Some(data.id), None).await;

            assert!(res.is_ok());
            let stored = res.unwrap();
            assert_eq!(stored.len(), 1);
            let stored_key = stored.get(0).unwrap().to_owned();

            assert_eq!(stored_key, data);
        })
    })
    .await;
}

#[tokio::test]
#[rstest]
#[case(1)]
#[case(5)]
#[case(10)]
async fn test_get_apikey_valid_prefix(#[case] amount_of_same_prefix_keys: usize) {
    with_test_db(|conn| {
        Box::pin(async move {
            let mut keys: Vec<ApiKey> = Vec::new();

            let prefix = random_string(10);
            let pre = prefix.clone();
            for _ in 0..amount_of_same_prefix_keys {
                let pre = pre.clone();
                let full_key = pre.clone() + &random_string(32);
                let hash = hash_key(&full_key).expect("Hashing string returns error");
                let data = seed_api_key_given(conn, hash, pre, "test-suite", vec![]);
                keys.push(data);
            }

            let res = get_apikey(conn, None, Some(prefix)).await;
            assert!(res.is_ok());
            let stored = res.unwrap();
            assert_eq!(stored.len(), amount_of_same_prefix_keys);
            for i in 0..amount_of_same_prefix_keys {
                let stored_data = stored.get(i).unwrap();
                assert!(keys.contains(stored_data));
            }
        })
    })
    .await;
}

#[tokio::test]
#[rstest]
#[case(1)]
#[case(5)]
#[case(10)]
async fn test_get_apikey_valid_intersection(#[case] amount_of_same_prefix_keys: usize) {
    with_test_db(|conn| {
        Box::pin(async move {
            let mut keys: Vec<ApiKey> = Vec::new();

            let prefix = random_string(10);
            let pre = prefix.clone();
            for _ in 0..amount_of_same_prefix_keys {
                let pre = pre.clone();
                let full_key = pre.clone() + &random_string(32);
                let hash = hash_key(&full_key).expect("Hashing string returns error");
                let data = seed_api_key_given(conn, hash, pre, "test-suite", vec![]);
                keys.push(data);
            }

            let target = keys.get(0).unwrap();
            let res = get_apikey(conn, Some(target.id), Some(prefix.clone())).await;
            let res_id = get_apikey(conn, Some(target.id), None).await;
            let res_pre = get_apikey(conn, None, Some(prefix)).await;

            assert!(res.is_ok());
            assert!(res_id.is_ok());
            assert!(res_pre.is_ok());

            let stored = res.unwrap();
            let stored_id = res_id.unwrap();
            let stored_pre = res_pre.unwrap();
            assert_eq!(stored.len(), 1);
            assert_eq!(stored_id.len(), 1);
            assert_eq!(stored_pre.len(), amount_of_same_prefix_keys);

            let mut intersection = stored_pre.clone();
            intersection.retain(|x| stored_id.contains(x));
            assert_eq!(intersection.len(), 1);
            assert_eq!(intersection, stored);
        })
    })
    .await;
}

#[tokio::test]
async fn test_get_apikey_unknown_id() {
    with_test_db(|conn| {
        Box::pin(async move {
            let data = seed_api_key(conn, "test-suite", vec![]);
            let unknown_id = data.id + 1;

            let res = get_apikey(conn, Some(unknown_id), None).await;
            assert!(res.is_ok());
            let stored = res.unwrap();
            assert_eq!(stored.len(), 0);
        })
    })
    .await;
}

#[tokio::test]
async fn test_get_apikey_unknown_prefix() {
    with_test_db(|conn| {
        Box::pin(async move {
            let data = seed_api_key(conn, "test-suite", vec![]);
            let unknown_prefix = random_string(10);
            assert_ne!(data.key_prefix, unknown_prefix);

            let res = get_apikey(conn, None, Some(unknown_prefix)).await;
            assert!(res.is_ok());
            let stored = res.unwrap();
            assert_eq!(stored.len(), 0);
        })
    })
    .await;
}

// ==================================================================

#[tokio::test]
async fn test_delete_apikey_invalid_params() {
    with_test_db(|conn| {
        Box::pin(async move {
            let res = delete_apikey(conn, None, None).await;
            assert!(res.is_err());
        })
    })
    .await;
}

#[tokio::test]
async fn test_delete_apikey_valid_id() {
    with_test_db(|conn| {
        Box::pin(async move {
            let data = seed_api_key(conn, "test-suite", vec![]);
            let other = seed_api_key(conn, "someone", vec![]);

            let res1 = get_apikey(conn, Some(data.id), None).await;
            let res2 = delete_apikey(conn, Some(data.id), None).await;
            let res3 = get_apikey(conn, Some(data.id), None).await;

            assert!(res1.is_ok());
            assert!(res2.is_ok());
            assert!(res3.is_ok());

            let (stored1, stored3) = (res1.unwrap(), res3.unwrap());
            assert_eq!(stored1.len(), 1);
            assert!(stored1.contains(&data));
            assert_eq!(stored3.len(), 0);

            let res4 = get_apikey(conn, Some(other.id), None).await;
            assert!(res4.is_ok());
            let stored4 = res4.unwrap();
            assert_eq!(stored4.len(), 1);
            assert!(stored4.contains(&other));
        })
    })
    .await;
}

#[tokio::test]
#[rstest]
#[case(1)]
#[case(5)]
#[case(10)]
async fn test_delete_apikey_valid_prefix(#[case] amount_of_same_prefix_keys: usize) {
    with_test_db(|conn| {
        Box::pin(async move {
            let mut keys: Vec<ApiKey> = Vec::new();
            let other = seed_api_key(conn, "someone", vec![]);

            let prefix = random_string(10);
            assert_ne!(other.key_prefix, prefix);
            let pre = prefix.clone();
            for _ in 0..amount_of_same_prefix_keys {
                let pre = pre.clone();
                let full_key = pre.clone() + &random_string(32);
                let hash = hash_key(&full_key).expect("Hashing string returns error");
                let data = seed_api_key_given(conn, hash, pre, "test-suite", vec![]);
                keys.push(data);
            }

            let res1 = get_apikey(conn, None, Some(prefix.clone())).await;
            let res2 = delete_apikey(conn, None, Some(prefix.clone())).await;
            let res3 = get_apikey(conn, None, Some(prefix)).await;
            let res4 = get_apikey(conn, None, Some(other.key_prefix.clone())).await;

            assert!(res1.is_ok());
            assert!(res2.is_ok());
            assert!(res3.is_ok());
            assert!(res4.is_ok());

            let (stored1, stored3, stored4) = (res1.unwrap(), res3.unwrap(), res4.unwrap());

            assert_eq!(stored1.len(), amount_of_same_prefix_keys);
            assert_eq!(stored3.len(), 0);
            assert_eq!(stored4.len(), 1);
            assert!(stored4.contains(&other));
        })
    })
    .await;
}

#[tokio::test]
#[rstest]
#[case(1)]
#[case(5)]
#[case(10)]
async fn test_delete_apikey_valid_intersection(#[case] amount_of_same_prefix_keys: usize) {
    with_test_db(|conn| {
        Box::pin(async move {
            let mut keys: Vec<ApiKey> = Vec::new();
            let other = seed_api_key(conn, "someone", vec![]);

            let prefix = random_string(10);
            assert_ne!(other.key_prefix, prefix);
            let pre = prefix.clone();
            for _ in 0..amount_of_same_prefix_keys {
                let pre = pre.clone();
                let full_key = pre.clone() + &random_string(32);
                let hash = hash_key(&full_key).expect("Hashing string returns error");
                let data = seed_api_key_given(conn, hash, pre, "test-suite", vec![]);
                keys.push(data);
            }

            let target = keys.get(0).unwrap();

            let res1 = get_apikey(conn, None, Some(prefix.clone())).await;
            let res2 = delete_apikey(conn, Some(target.id), Some(prefix.clone())).await;
            let res3 = get_apikey(conn, None, Some(prefix.clone())).await;
            let res4 = get_apikey(conn, Some(other.id), None).await;

            assert!(res1.is_ok());
            assert!(res2.is_ok());
            assert!(res3.is_ok());
            assert!(res4.is_ok());

            let (stored1, stored3, stored4) = (res1.unwrap(), res3.unwrap(), res4.unwrap());
            assert_eq!(stored1.len(), amount_of_same_prefix_keys);
            assert_eq!(stored3.len(), amount_of_same_prefix_keys - 1);
            assert_eq!(stored4.len(), 1);
        })
    })
    .await;
}

#[tokio::test]
async fn test_delete_unknown_id() {
    with_test_db(|conn| {
        Box::pin(async move {
            let data = seed_api_key(conn, "test-suite", vec![]);
            let unknown_id = data.id + 1;

            let res1 = get_apikey(conn, Some(unknown_id), None).await;
            let res2 = get_apikey(conn, Some(data.id), None).await;
            let res3 = delete_apikey(conn, Some(unknown_id), None).await;
            let res4 = get_apikey(conn, Some(data.id), None).await;

            assert!(res1.is_ok());
            assert!(res2.is_ok());
            assert!(res3.is_ok());
            assert!(res4.is_ok());

            let (stored1, stored2, stored4) = (res1.unwrap(), res2.unwrap(), res4.unwrap());
            assert_eq!(stored1.len(), 0);
            assert_eq!(stored2.len(), 1);
            assert_eq!(stored4.len(), 1);
        })
    })
    .await;
}

#[tokio::test]
async fn test_delete_unknown_prefix() {
    with_test_db(|conn| {
        Box::pin(async move {
            let data = seed_api_key(conn, "test-suite", vec![]);
            let unknown_pre = random_string(10);

            assert_ne!(unknown_pre, data.key_prefix);

            let res1 = get_apikey(conn, None, Some(unknown_pre.clone())).await;
            let res2 = get_apikey(conn, None, Some(data.key_prefix.clone())).await;
            let res3 = delete_apikey(conn, None, Some(unknown_pre)).await;
            let res4 = get_apikey(conn, None, Some(data.key_prefix)).await;

            assert!(res1.is_ok());
            assert!(res2.is_ok());
            assert!(res3.is_ok());
            assert!(res4.is_ok());

            let (stored1, stored2, stored4) = (res1.unwrap(), res2.unwrap(), res4.unwrap());
            assert_eq!(stored1.len(), 0);
            assert_eq!(stored2.len(), 1);
            assert_eq!(stored4.len(), 1);
        })
    })
    .await;
}

// ==================================================================
