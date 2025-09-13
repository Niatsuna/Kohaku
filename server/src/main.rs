use actix_web::{App, HttpServer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_thread_ids(true)
        .pretty()
        .init();
    info!("Logging initialized! Starting server ...");

    let server_addr: String = "127.0.0.1".to_string();
    let server_port: u16 = 8080;

    HttpServer::new(|| App::new())
        .bind((server_addr, server_port))?
        .run()
        .await
}
