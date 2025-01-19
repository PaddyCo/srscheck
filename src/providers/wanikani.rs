use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::{error, info, instrument, warn};

use crate::cache::Cache;

use super::{DataSource, ProviderData};

#[derive(Debug, Deserialize)]
pub struct WaniKaniProvider {
    api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewData {
    /// DateTime when the reviews are available (ISO 8601 format)
    available_at: String,
    /// List of subject IDs
    subject_ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
struct SummaryData {
    /// Details about subjects available for reviews now and in the next 24 hours by the hour.
    /// (total of 25 objects).
    reviews: Vec<ReviewData>,
    /// DateTime when the next reviews are available (ISO 8601 format)
    next_reviews_at: String,
}

#[derive(Debug, Deserialize)]
struct SummaryResponse {
    data: SummaryData,
}

impl DataSource for WaniKaniProvider {
    #[instrument(name = "WaniKaniProvider::get_data", skip(self, _cache))]
    async fn get_data(&self, _cache: Cache) -> Result<ProviderData, reqwest::Error> {
        let client = reqwest::Client::new();
        info!("Fetching data from WaniKani...");
        let resp = client
            .get("https://api.wanikani.com/v2/summary")
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        info!("Successfully fetched data from WaniKani");

        let summary = resp.json::<SummaryResponse>().await?;
        // Get current review count:
        if summary.data.reviews.is_empty() {
            error!("Reviews returned empty in summary!");
            return Ok(ProviderData {
                review_count: 0,
                next_review: None,
            });
        }

        let review_count = match summary.data.reviews.len() {
            0 => 0,
            _ => summary.data.reviews[0].subject_ids.len(),
        };

        Ok(ProviderData {
            review_count: review_count as i32,
            next_review: Some(
                DateTime::parse_from_rfc3339(&summary.data.next_reviews_at)
                    .unwrap()
                    .with_timezone(&Utc),
            ),
        })
    }
}
