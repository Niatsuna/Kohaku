use actix_web::{App, HttpServer};
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

use crate::utils::{
    config::{get_config, init_config},
    scheduler::scheduler::{get_scheduler, init_scheduler},
};

mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    if let Err(_) = init_config() {
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
    info!("Logging initialized! Initializing scheduler ...");

    // Start scheduler
    if let Err(_) = init_scheduler().await {
        error!("Couldn't initialize scheduler!");
    } else {
        info!("Scheduler initilialized! Starting scheduler ...");
        let scheduler = get_scheduler().await;
        if let Err(_) = scheduler.start().await {
            error!("Couldn't start scheduler!");
        }
        info!("Scheduler started!");
    }

    HttpServer::new(|| App::new())
        .bind((config.server_addr.clone(), config.server_port))?
        .run()
        .await
}
