use actix_web::{web, App, HttpServer};
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

use crate::{
    db::migrate,
    utils::{
        comm::auth::{configure_auth_routes, jwt::init_jwtservice},
        config::{get_config, init_config},
        scheduler::{get_scheduler, init_scheduler},
    },
};

mod db;
mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    if init_config().is_err() {
        error!("Couldn't initialize config!");
    }
    let config = get_config();

    FmtSubscriber::builder()
        .with_max_level(config.logging_level)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_thread_ids(true)
        .pretty()
        .init();
    info!("Logging initialized!");

    // Setup database
    info!("Running database migration ...");
    if let Err(e) = migrate() {
        error!("{}", e);
    }

    // Start scheduler
    info!("Setting up scheduler ...");
    if init_scheduler().await.is_err() {
        error!("Couldn't initialize scheduler!");
    } else {
        info!("Scheduler initilialized! Starting scheduler ...");
        let scheduler = get_scheduler().await;
        if scheduler.start().await.is_err() {
            error!("Couldn't start scheduler!");
        }
        info!("Scheduler started!");
    }

    // Start JWT Service
    info!("Setting up JWTService ...");
    if init_jwtservice(&config.encryption_key).is_ok() {
        info!("JWTService started!");
    } else {
        error!("Couldn't initialize JWTService! Protected endpoints will return an error!");
    }

    // Start websocket
    // TODO:

    HttpServer::new(|| {
        App::new().service(
            web::scope("/api").service(web::scope("/auth").configure(configure_auth_routes)),
        )
    })
    .bind((config.server_addr.clone(), config.server_port))?
    .run()
    .await
}
