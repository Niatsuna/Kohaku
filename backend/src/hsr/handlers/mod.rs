use actix_web::{web, HttpResponse};

mod item;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/hsr")
            .route("/items", web::get().to(item::item_list))
            .default_service(
                web::route()
                    .to(|| async { HttpResponse::Ok().json("API development placeholder") }),
            ),
    );
}
