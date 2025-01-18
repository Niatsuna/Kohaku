use actix_web::{App, HttpServer};
use std::env;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod db;
mod handlers;
mod models;
mod scrapers;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    // Logging
    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_thread_ids(true)
        //.with_thread_names(true)
        .pretty()
        .init();
    info!("Starting server ...");

    // Scheduler
    //TODO: Implement

    // Start actix server
    let server_addr: String = env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port: u16 = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("SERVER_PORT must be a valid port number");

    HttpServer::new(|| App::new().configure(handlers::init))
        .bind((server_addr, server_port))?
        .run()
        .await
}
