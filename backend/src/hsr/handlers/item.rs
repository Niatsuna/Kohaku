use actix_web::HttpResponse;
use diesel::{query_dsl::methods::SelectDsl, RunQueryDsl};

use crate::{db::get_connection, error::KohakuError, hsr::models::db::item::Item};

pub async fn item_list() -> HttpResponse {
    use crate::db::schema::hsr_items::dsl::*;
    use diesel::SelectableHelper;

    let mut conn = get_connection().unwrap();
    let result = hsr_items
        .select(Item::as_select())
        .load::<Item>(&mut conn)
        .map_err(KohakuError::from)
        .unwrap();
    HttpResponse::Ok().json(result)
}
