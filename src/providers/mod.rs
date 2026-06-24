use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cache::Cache;

pub mod anki;
pub mod bunpro;
pub mod http;
pub mod kamesame;
pub mod wanikani;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderData {
    /// Number of reviews available
    pub review_count: i32,
    /// DateTime when the next review is available, returns None if provider does not support it
    pub next_review: Option<DateTime<Utc>>,
    /// URL to open to act on the reviews, returns None if provider has no URL configured
    pub action_url: Option<String>,
}

pub trait DataSource {
    /// Get the data from the provider
    async fn get_data(&self, cache: Cache) -> Result<ProviderData, reqwest::Error>;
}
