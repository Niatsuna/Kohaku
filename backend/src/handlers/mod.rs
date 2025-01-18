use actix_web::{web, HttpResponse};

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(200)
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api").route("/", web::get().to(health_check)));
}
