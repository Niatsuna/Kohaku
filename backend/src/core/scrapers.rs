/// Core functionalities for all scrapers.
/// This includes for example requesting pages / jsons and checking if a scrape is necessary
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use reqwest::{Client, Error, Response};
use tracing::info;

use crate::{
    db::{get_connection, schema::urls},
    error::KohakuError,
    hsr,
};

use super::scheduler::{Scheduler, Task};

/// Representation of a scraped url in the database.
/// Is used to check if a scrape is necessary.
#[derive(Queryable, Debug)]
#[diesel(table_name = urls)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Url {
    pub id: i32,
    pub addr: String,
    pub last_scraped: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = urls)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUrl {
    pub addr: String,
    pub last_scraped: DateTime<Utc>,
}

pub async fn init_scrapers(scheduler: Scheduler) {
    info!("[Scraper] - Setting up scrapers...");
    // Add scrapers to scheduler
    scheduler
        .add_task(Task::new(
            "hsr",
            hsr::scrapers::SCHEDULE,
            hsr::scrapers::scrape,
            false,
        ))
        .await;
    info!("[Scraper] - Done");
}

/// Requests a page for a scraper
pub async fn request_page(url: &str) -> Result<Response, Error> {
    let client = Client::new();
    client.get(url).send().await
}

/// Check if a requested url is present in the database and if an update happened since the last scrape.
///
/// ## Arguments
/// - `url : &str` - requested url
/// - `last_modified : &str` - last_modified value from the response header
///
/// ## Returns
/// - `Ok(True)` - if a scrape is necessary (Url not present in the database or an update happened since last scrape)
/// - `Ok(False)` - if a scrape is unnecessary (Url present and since last scrape no update)
/// - `Err(KohakuError)` - if an error occured
pub async fn check_scrape_necessity(url: &str, last_modified: &str) -> Result<bool, KohakuError> {
    use crate::db::schema::urls::dsl::*;

    // Find url in database
    let mut conn = get_connection()?;
    let record = urls
        .filter(addr.eq(url))
        .first::<Url>(&mut conn)
        .optional()?;

    if let Some(record_) = record {
        // Entry exists > Check date
        let datetime =
            DateTime::parse_from_rfc2822(last_modified).map_err(KohakuError::ParseTimeError)?;
        let utc_time: DateTime<Utc> = datetime.into();
        return Ok(record_.last_scraped < utc_time);
    }

    Ok(true)
}

/// Updates a requested url in the database. If the url is new, it adds it to the database.
/// The timestamp is the current time.
///
/// ## Arguments
/// - `url : &str` - requested url
///
/// ## Returns
/// - `Ok(())` - If the execution was successful without errors
/// - `Err(KohakuError)` - otherwise
pub async fn update_url(url: &str) -> Result<(), KohakuError> {
    use crate::db::schema::urls::dsl::*;

    let mut conn = get_connection()?;

    // New / Updated entry
    let record = NewUrl {
        addr: url.to_string(),
        last_scraped: Utc::now(),
    };

    // Upsert: Update or Insert
    diesel::insert_into(urls)
        .values(&record)
        .on_conflict(addr)
        .do_update()
        .set(last_scraped.eq(record.last_scraped))
        .execute(&mut conn)
        .map_err(KohakuError::QueryResultError)?;

    Ok(())
}
