use actix_web::{web, HttpResponse, Result};
use diesel::prelude::*;
use serde::Deserialize;
use strsim::jaro_winkler;
use validator::Validate;

use crate::{
    db::get_connection,
    error::KohakuError,
    hsr::models::{
        api::item::DirectItem,
        db::item::{Item, ShortItem},
    },
};

#[derive(Debug, Deserialize, Validate)]
pub struct ListParams {
    name: Option<String>,
    #[validate(range(min = 2, max = 5))]
    rarity: Option<i32>,
    #[validate(range(min = 1, max = 500))]
    limit: Option<i64>,
}

/// API Endpoint: `/api/hsr/items?limit=...&name=...&rarity=...`.
/// Returns a list of stored items. Can be used to search for items.
/// ## Arguments (Optional)
/// - `limit : Option<i32>` - limits the given output. (range: [1..500]) (Default: None)
/// - `name : Option<String>` - given item name to look for (Default: None)
/// - `rarity : Option<i32>` - given item rarity to look for. (range: [2..5]) (Default: None)
///
/// ## Returns
/// - `200` with a list of items in the json body fitting the given arguments
/// - `400` if the given arguments were invalid
/// - `404` if no item data was found
///
/// ## Note
/// Default results in a list of all items.
/// Given a name items are filtered and sorted by similarity score (Jaro Winkler). Threshold: 80%
pub async fn item_list(params: web::Query<ListParams>) -> Result<HttpResponse> {
    use crate::db::schema::hsr_items::dsl::*;

    // Check Arguments
    params.validate().map_err(KohakuError::from)?;

    // Build Query (without name)
    let mut query = hsr_items.into_boxed();
    if let Some(rarity_) = &params.rarity {
        query = query.filter(rarity.eq(rarity_));
    }

    if let Some(limit) = &params.limit {
        query = query.limit(limit.to_owned());
    }

    // Get all items
    let mut conn = get_connection()?;
    let mut result: Vec<ShortItem> = query
        .select(ShortItem::as_select())
        .load(&mut conn)
        .map_err(KohakuError::from)?;

    // If name was set, get similarity scores for each item, filter and sort
    if let Some(nm) = &params.name {
        // Similarity (Scored & Filtered)
        let mut scored_result = result
            .into_iter()
            .map(|item| {
                let score = jaro_winkler(&item.name.to_lowercase(), &nm.to_lowercase());
                (item, score)
            })
            .filter(|(_, score)| *score >= 0.8)
            .collect::<Vec<(ShortItem, f64)>>();

        // Sort : Descending order (Highest similarity first)
        scored_result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        result = scored_result
            .into_iter()
            .map(|(item, _)| item)
            .collect::<Vec<ShortItem>>();
    }

    if result.is_empty() {
        return Ok(HttpResponse::NotFound().json("No item found (404)"));
    }

    Ok(HttpResponse::Ok().json(result))
}

/// API Endpoint: `/api/hsr/items/{id}`
/// Returns an direct item
/// ## Arguments
/// - `id : i32` - The direct id of said item in the database
///
/// ## Returns
/// - `200` with the items data in the json body if the item was found
/// - `404` if the item was not found
///
pub async fn item_direct(path: web::Path<i32>) -> Result<HttpResponse> {
    use crate::db::schema::hsr_items::dsl::*;
    let item_id = path.into_inner();

    let mut conn = get_connection()?;

    let result = hsr_items
        .filter(id.eq(item_id))
        .first::<Item>(&mut conn)
        .optional()
        .map_err(KohakuError::from)?;
    if let Some(res) = result {
        // Found item
        Ok(HttpResponse::Ok().json(DirectItem { db: res }))
    } else {
        // Item not found
        Ok(HttpResponse::NotFound().json("404 Not Found"))
    }
}
