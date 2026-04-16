use std::{future::Future, pin::Pin};

use diesel::{Connection as _, RunQueryDsl};
use kohaku::{
    db::{get_connection, migrate, schema, Connection},
    utils::comm::auth::{
        api_key::{generate_key, hash_key},
        models::{ApiKey, NewApiKey},
    },
};

pub async fn with_test_db<F>(f: F)
where
    for<'a> F: FnOnce(&'a mut Connection) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
{
    dotenvy::dotenv().ok();

    let test_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set in .env for integration and e2e tests!");
    std::env::set_var("DATABASE_URL", test_url);

    let mut conn = get_connection().expect("Failed to get test DB connection");
    migrate().expect("Failed to migrate test database");
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    f(&mut conn).await;
}
