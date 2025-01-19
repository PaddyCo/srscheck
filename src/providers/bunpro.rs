use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::{info, instrument, warn};

use crate::cache::Cache;

use super::{DataSource, ProviderData};

#[derive(Debug, Deserialize)]
pub struct BunproProvider {
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct StudyQueueData {
    /// Number of reviews available
    reviews_available: i32,
    /// Number of reviews available in the next hour
    reviews_available_next_hour: i32,
    /// Number of reviews available in the next day
    reviews_available_next_day: i32,
    /// DateTime when the next review is available (in seconds since Unix epoch)
    next_review_date: i64,
}

#[derive(Debug, Deserialize)]
struct StudyQueueResponse {
    requested_information: StudyQueueData,
}

impl DataSource for BunproProvider {
    #[instrument(name = "BunproProvider::get_data", skip(self, _cache))]
    async fn get_data(&self, _cache: Cache) -> Result<ProviderData, reqwest::Error> {
        let client = reqwest::Client::new();
        info!("Fetching data from Bunpro...");

        let url = format!("https://bunpro.jp/api/user/{}/study_queue", self.api_key);

        let resp = client.get(url).send().await?;
        info!("Successfully fetched data from Bunpro");

        let study_queue = resp.json::<StudyQueueResponse>().await?;

        let data = study_queue.requested_information;

        Ok(ProviderData {
            review_count: data.reviews_available,
            next_review: Some(
                DateTime::from_timestamp(data.next_review_date, 0)
                    .unwrap()
                    .to_utc(),
            ),
        })
    }
}
