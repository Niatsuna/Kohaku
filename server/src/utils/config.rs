use std::{env, str::FromStr, sync::Arc};
use tokio::sync::OnceCell;

static CONFIG: OnceCell<Arc<Config>> = OnceCell::const_new();

fn read_env(name: &str, default: Option<&str>) -> String {
    let value = env::var(name);
    if let Some(def) = default {
        value.unwrap_or_else(|_| def.to_string())
    } else {
        value
            .unwrap_or_else(|_| panic!("{} not set!", name))
            .to_string()
    }
}

#[derive(Debug)]
pub struct Config {
    // > Core
    // Server
    pub server_addr: String,
    pub server_port: u16,

    // Logging
    pub logging_level: tracing::Level,

    // Database
    pub database_url: String,

    // Communication
    pub bootstrap_key: String,
    pub encryption_key: Vec<u8>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            server_addr: read_env("SERVER_ADDR", Some("127.0.0.1")),
            server_port: read_env("SERVER_PORT", Some("8080"))
                .parse()
                .expect("SERVER_PORT must be a valid port number"),
            logging_level: tracing::Level::from_str(&read_env(
                "SERVER_LOGGING_LEVEL",
                Some("INFO"),
            ))
            .unwrap(),
            database_url: read_env("DATABASE_URL", None),
            bootstrap_key: read_env("BOOTSTRAP_KEY", None),
            encryption_key: read_env("SERVER_ENCRYPTION_KEY", None).into_bytes(),
        }
    }
}

pub fn init_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(Config::new());
    CONFIG
        .set(config)
        .map_err(|_| "Config already initialized")?;
    Ok(())
}

pub fn get_config() -> Arc<Config> {
    CONFIG
        .get()
        .expect("Config not initialized - call init_config first")
        .clone()
}
