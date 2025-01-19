use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod anki;
pub mod bunpro;
pub mod wanikani;

#[derive(Debug, Serialize)]
pub struct ProviderData {
    /// Number of reviews available
    pub review_count: i32,
    /// DateTime when the next review is available, returns None if provider does not support it
    pub next_review: Option<DateTime<Utc>>,
}

pub trait DataSource {
    /// Get the data from the provider
    async fn get_data(&self) -> Result<ProviderData, reqwest::Error>;
}
