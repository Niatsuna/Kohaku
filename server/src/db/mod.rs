use std::sync::{Arc, Mutex};

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};

use once_cell::sync::Lazy;

use crate::utils::config::get_config;
use crate::utils::error::KohakuError;

pub mod schema;

pub type Pool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;
pub type Connection = PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>;

static DB_POLL: Lazy<Arc<Mutex<Pool>>> =
    Lazy::new(|| Arc::new(Mutex::new(establish_connection_pool())));

fn establish_connection_pool() -> Pool {
    let database_url = get_config().database_url.clone();
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool!")
}

pub fn get_connection() -> Result<Connection, KohakuError> {
    let pool = DB_POLL.lock().unwrap();
    pool.get().map_err(KohakuError::DatabaseConnectionError)
}
