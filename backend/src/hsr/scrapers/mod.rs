use tracing::{error, info};

pub mod item;

pub const BASE_URL: &str = "https://api.hakush.in/hsr";
pub const SCHEDULE: &str = "0 0 12 * * *"; // Every day at 12 PM (UTC)

pub async fn scrape() {
    info!("[Scraper - HSR] Scraping Honkai: Star Rail ...");
    // Run the different scraper-types (Materials, Light Cones, etc.)
    let endpoints = vec![item::run];

    for func in endpoints {
        if let Err(e) = func().await {
            let msg = e.to_string();
            error!(msg);
        }
    }
}
