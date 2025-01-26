use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use once_cell::sync::Lazy;
use std::env;
use std::sync::{Arc, Mutex};

use crate::error::KohakuError;

// Needs to be generated using: diesel print-schema > src/db/schema.rs
pub mod schema;

pub type Pool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;
pub type Connection = PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>;

static DB_POOL: Lazy<Arc<Mutex<Pool>>> =
    Lazy::new(|| Arc::new(Mutex::new(establish_connection_pool())));

/// Established a pool for connections to the given database (See env. DATABASE_URL)
fn establish_connection_pool() -> Pool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let manager = ConnectionManager::<PgConnection>::new(database_url);

    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}

/// Establishes a connection to the given database (See env. DATABASE_URL)
///
/// # Returns
/// `db::Connection` if a valid connection was established or `diesel::r2d2::Error` if an error occured.
pub fn get_connection() -> Result<Connection, KohakuError> {
    let pool = DB_POOL.lock().unwrap();
    pool.get().map_err(KohakuError::ConnectionPoolError)
}
