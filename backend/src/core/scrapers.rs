/// Core functionalities for all scrapers.
/// This includes for example requesting pages / jsons and checking if a scrape is necessary (TODO)
use reqwest::{Client, Error, Response};
use tracing::info;

use super::scheduler::{Scheduler, Task};

pub async fn init_scrapers(scheduler: Scheduler) {
    info!("[Scraper] - Setting up scrapers...");
    // Add scrapers to scheduler
    // Example:
    //scheduler.add_task(Task::new("scraper1", "* * * * * *", example, false)).await;
    info!("[Scraper] - Done");
}

/// Requests a page for a scraper
pub async fn request_page(url: &str) -> Result<Response, Error> {
    let client = Client::new();
    client.get(url).send().await
}
