use actix_web::{web, App, HttpServer};
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

use kohaku::{
    db::{get_connection, migrate},
    utils::{
        comm::{self, events},
        config::{get_config, init_config},
        initialize_services,
    },
};

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
        //.with_file(true)
        .with_target(false)
        .with_thread_ids(true)
        .pretty()
        .init();
    info!("Logging initialized!");

    // Setup database
    info!("Running database migration ...");
    let mut conn = get_connection().expect("Failed to connect to database");
    if let Err(e) = migrate(&mut conn) {
        error!("{}", e);
    }

    initialize_services(&config).await;

    HttpServer::new(|| {
        App::new()
            .service(
                web::scope("/api")
                    .service(web::scope("/auth").configure(comm::auth::routes::configure))
                    .service(web::scope("/events").configure(events::configure)),
            )
            .route("/ws", web::get().to(comm::websocket::routes::ws_handler))
    })
    .bind((config.server_addr.clone(), config.server_port))?
    .run()
    .await
}
