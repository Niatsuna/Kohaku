use std::{future::Future, pin::Pin};

use diesel::{Connection as _, PgConnection};
use kohaku::db::migrate;

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
