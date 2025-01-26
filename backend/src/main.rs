use actix_web::{web, App, HttpResponse, HttpServer};
use std::env;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod core;
mod db;
mod hsr;

use core::{scheduler::Scheduler, scrapers};

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
    if let Ok(scheduler) = Scheduler::new().await {
        let _ = scheduler.start().await;
        // Add scheduled task here
        scrapers::init_scrapers(scheduler).await;
        // -----
    }

    // Get Environment variables
    let server_addr: String = env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port: u16 = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("SERVER_PORT must be a valid port number");

    // Start actix web server
    HttpServer::new(|| {
        App::new()
            .service(
                // API
                web::scope("/api").default_service(
                    web::route()
                        .to(|| async { HttpResponse::Ok().json("API development placeholder") }),
                ),
            ) // Other Endpoints
            .default_service(
                web::route().to(|| async { HttpResponse::NotFound().body("404 Not Found") }),
            )
    })
    .bind((server_addr, server_port))?
    .run()
    .await
}
