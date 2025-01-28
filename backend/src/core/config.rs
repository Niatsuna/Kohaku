use actix_web::{web, HttpResponse};

use crate::hsr;

async fn health_check() -> HttpResponse {
    hsr::scrapers::item::run().await.unwrap();
    HttpResponse::Ok().json("Health Check: 200")
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        // API
        web::scope("/api")
            .route("/", web::get().to(health_check))
            .configure(hsr::handlers::init)
            .default_service(
                web::route()
                    .to(|| async { HttpResponse::Ok().json("API development placeholder") }),
            ),
    ) // Other Endpoints
    .default_service(web::route().to(|| async { HttpResponse::NotFound().body("404 Not Found") }));
}
