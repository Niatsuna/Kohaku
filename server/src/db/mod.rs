use std::sync::{Arc, Mutex};

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};

use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use once_cell::sync::Lazy;
use tracing::info;

use crate::utils::config::get_config;
use crate::utils::error::KohakuError;

pub mod schema;

pub type Pool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;
pub type Connection = PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>;

static DB_POLL: Lazy<Arc<Mutex<Pool>>> =
    Lazy::new(|| Arc::new(Mutex::new(establish_connection_pool())));

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/db/migrations");

fn get_database_url() -> String {
    get_config().database_url.clone()
}

fn establish_connection_pool() -> Pool {
    let database_url = get_database_url();
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool!")
}

pub fn get_connection() -> Result<Connection, KohakuError> {
    let pool = DB_POLL.lock().unwrap();
    pool.get().map_err(KohakuError::DatabaseConnectionError)
}

pub fn migrate(conn: &mut PgConnection) -> Result<(), KohakuError> {
    let mig = conn
        .run_pending_migrations(MIGRATIONS)
        .map_err(|e| KohakuError::ExternalServiceError(format!("Migration failed: {}", e)))?;
    info!("Migrations applied! (Count: {})", mig.len());
    Ok(())
}
