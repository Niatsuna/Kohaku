use reqwest::{Client, Error, Response};

/// Struct to store information about the scraper
///
/// ### Fields
///  - `name` - Name of the game the data is scraped from (For logging purposes)
///  - `cron` - Cron like string to determine WHEN the scraper should run (tokio-cron-scheduler)
///  - `func` - Async function that get's executed when `Scraper.run()` is called
pub struct Scraper<F>
where
    F: std::future::Future,
{
    name: String,
    cron: String, // * * * * * * | sec min hour day-of-month month day-of-week
    func: fn() -> F,
}

impl<F> Scraper<F>
where
    F: std::future::Future,
{
    async fn run(self) {
        (self.func)().await;
    }
}

/// Requests a page for a scraper
pub async fn request_page(url: &str) -> Result<Response, Error> {
    let client = Client::new();
    client.get(url).send().await
}
