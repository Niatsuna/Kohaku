use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::RunQueryDsl;
use tracing::{error, info};

use super::BASE_URL;
use crate::{
    core::scrapers::{check_scrape_necessity, request_page, update_url},
    db::get_connection,
    error::KohakuError,
    hsr::{
        format::format_item,
        models::parse::item::{Item as ParseItem, ListItem},
    },
};

pub const API_ENDPOINT: &str = "data/en/item";
pub const WIKI_URL: &str = "https://hsr18.hakush.in/item";

async fn scrape_item(key: &str) -> Result<(), KohakuError> {
    use crate::db::schema::hsr_items::dsl::*;
    // Get API data
    let url = format!("{BASE_URL}/{API_ENDPOINT}/{key}.json");
    let resp = request_page(&url).await?;
    let data: ParseItem = resp.json().await.map_err(KohakuError::from)?;

    // Format parsed data to database Item
    let item = format_item(key, data)?;

    // Save Item in database
    let mut conn = get_connection()?;
    diesel::insert_into(hsr_items)
        .values(&item)
        .on_conflict(name)
        .do_update()
        .set(&item)
        .execute(&mut conn)
        .map_err(KohakuError::from)?;

    Ok(())
}

pub async fn run() -> Result<(), KohakuError> {
    let url = format!("{BASE_URL}/{API_ENDPOINT}.json");
    let resp = request_page(&url).await?;

    // Get last-modified
    match resp.headers().get("last-modified") {
        Some(lm) => {
            let last_modified = lm.to_str().map_err(KohakuError::from)?;

            // Check if scraping is necessary
            if check_scrape_necessity(&url, last_modified).await? {
                info!("[Scraper - HSR] Items: Start scraping ...");
                // Get list of items
                let mut data: HashMap<String, ListItem> =
                    resp.json().await.map_err(KohakuError::from)?;

                // Filter out unnecessary data
                data = data
                    .into_iter()
                    .filter(|(_, item)| item.purpose_type <= 13 && item.purpose_type != 10)
                    .collect();

                // Scrape individual items
                let start: DateTime<Utc> = Utc::now();
                let mut count = 0;
                for (key, _) in data {
                    if let Err(e) = scrape_item(&key).await {
                        error!("[Scraper - HSR] Failed to scrape item with api id '{key}' : {e}")
                    } else {
                        count = count + 1;
                    }
                }
                let end: DateTime<Utc> = Utc::now();
                let time = (end - start).num_seconds();
                info!(
                    "[Scraper - HSR] Items: Scraped {} items! ({} sec)",
                    count, time
                );
                update_url(&url).await?;
            } else {
                info!("[Scraper - HSR] Items: Skipping!");
            }

            Ok(())
        }
        None => Err(KohakuError::CustomError(
            "Couldn't get last-modified from response header".to_string(),
        )),
    }
}
