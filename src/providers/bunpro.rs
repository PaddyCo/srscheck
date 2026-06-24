use std::time::Duration;

use chrono::DateTime;
use serde::Deserialize;
use tracing::{info, instrument, warn};

use crate::cache::{Cache, DATA_KEY};

use super::{DataSource, ProviderData};

fn default_action_url() -> Option<String> {
    Some("https://bunpro.jp/".to_string())
}

#[derive(Debug, Deserialize)]
pub struct BunproProvider {
    api_key: String,
    /// URL to open to do reviews
    #[serde(default = "default_action_url")]
    action_url: Option<String>,
    /// How long (in seconds) to cache API results for before fetching fresh data
    #[serde(default = "crate::cache::default_cache_expiry::<300>")]
    cache_expiry: u64,
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
    #[instrument(name = "BunproProvider::get_data", skip(self, cache))]
    async fn get_data(&self, cache: Cache) -> Result<ProviderData, reqwest::Error> {
        let api_key = self.api_key.clone();
        let ttl = Duration::from_secs(self.cache_expiry);

        let mut data = cache
            .get_or_fetch(DATA_KEY, ttl, || async move {
                let client = reqwest::Client::new();
                info!("Fetching data from Bunpro...");

                let url = format!("https://bunpro.jp/api/user/{}/study_queue", api_key);

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
                    action_url: None,
                })
            })
            .await?;

        data.action_url = self.action_url.clone();
        Ok(data)
    }
}
