use std::{future::Future, pin::Pin};

use diesel::{Connection as _, PgConnection, RunQueryDsl};
use kohaku::{
    db::{migrate, schema},
    utils::comm::auth::{
        api_key::{generate_key, hash_key},
        models::{ApiKey, NewApiKey},
    },
};

use crate::helper::utils::vec_str_to_string;

pub async fn with_test_db<F>(f: F)
where
    for<'a> F: FnOnce(&'a mut PgConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
{
    dotenvy::dotenv().ok();

    let test_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set in .env for integration and e2e tests!");

    let mut conn = PgConnection::establish(&test_url).expect("Failed to connect to database");
    migrate(&mut conn).expect("Failed to migrate test database");
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    f(&mut conn).await;
}

// ==================================================================

pub fn seed_api_key(conn: &mut PgConnection, owner: &str, scopes: Vec<&str>) -> ApiKey {
    let (full_key, prefix) = generate_key();
    let hash = hash_key(&full_key).expect("Hashing should succeed in test setup");

    seed_api_key_given(conn, hash, prefix, owner, scopes)
}

pub fn seed_api_key_given(
    conn: &mut PgConnection,
    hashed_key: String,
    key_prefix: String,
    owner: &str,
    scopes: Vec<&str>,
) -> ApiKey {
    let scopes = vec_str_to_string(scopes);
    let owner = owner.to_string();

    diesel::insert_into(schema::api_keys::table)
        .values(&NewApiKey {
            hashed_key,
            key_prefix,
            owner,
            scopes,
        })
        .get_result(conn)
        .expect("Seeding API key should succeed")
}
