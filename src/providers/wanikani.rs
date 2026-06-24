use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, warn};

use crate::cache::{Cache, DATA_KEY};

use super::{DataSource, ProviderData};

/// API revision to request, per https://docs.api.wanikani.com/20170710/#information
const WANIKANI_REVISION: &str = "20170710";
const USER_AGENT: &str = concat!("srscheck/", env!("CARGO_PKG_VERSION"));

fn default_action_url() -> Option<String> {
    Some("https://www.wanikani.com/".to_string())
}

#[derive(Debug, Deserialize)]
pub struct WaniKaniProvider {
    api_key: String,
    /// URL to open to do reviews
    #[serde(default = "default_action_url")]
    action_url: Option<String>,
    /// How long (in seconds) to cache API results for before fetching fresh data
    #[serde(default = "crate::cache::default_cache_expiry::<300>")]
    cache_expiry: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewData {
    /// DateTime when the reviews are available (ISO 8601 format)
    available_at: String,
    /// List of subject IDs
    subject_ids: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SummaryData {
    /// Details about subjects available for reviews now and in the next 24 hours by the hour.
    /// (total of 25 objects).
    reviews: Vec<ReviewData>,
    /// DateTime when the next reviews are available (ISO 8601 format)
    next_reviews_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SummaryResponse {
    data: SummaryData,
}

/// Cached summary, alongside the validators returned by WaniKani so we can make
/// conditional requests (`If-None-Match` / `If-Modified-Since`) instead of always
/// re-downloading the payload, per the API's caching guidance.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedSummary {
    summary: SummaryResponse,
    etag: Option<String>,
    last_modified: Option<String>,
    expires_at: DateTime<Utc>,
}

fn build_provider_data(summary: &SummaryResponse) -> ProviderData {
    // Get current review count:
    if summary.data.reviews.is_empty() {
        error!("Reviews returned empty in summary!");
        return ProviderData {
            review_count: 0,
            next_review: None,
            action_url: None,
        };
    }

    let review_count = summary.data.reviews[0].subject_ids.len();

    ProviderData {
        review_count: review_count as i32,
        next_review: Some(
            DateTime::parse_from_rfc3339(&summary.data.next_reviews_at)
                .unwrap()
                .with_timezone(&Utc),
        ),
        action_url: None,
    }
}

impl DataSource for WaniKaniProvider {
    #[instrument(name = "WaniKaniProvider::get_data", skip(self, cache))]
    async fn get_data(&self, cache: Cache) -> Result<ProviderData, reqwest::Error> {
        let ttl = chrono::Duration::from_std(Duration::from_secs(self.cache_expiry))
            .unwrap_or(chrono::Duration::zero());

        let cached = cache.read::<CachedSummary>(DATA_KEY).ok().flatten();

        let summary = match &cached {
            Some(entry) if entry.expires_at > Utc::now() => {
                info!("Using cached data for WaniKani");
                entry.summary.clone()
            }
            _ => {
                let client = reqwest::Client::new();
                let mut req = client
                    .get("https://api.wanikani.com/v2/summary")
                    .bearer_auth(&self.api_key)
                    .header("Wanikani-Revision", WANIKANI_REVISION)
                    .header(reqwest::header::USER_AGENT, USER_AGENT);

                if let Some(entry) = &cached {
                    if let Some(etag) = &entry.etag {
                        req = req.header(reqwest::header::IF_NONE_MATCH, etag);
                    }
                    if let Some(last_modified) = &entry.last_modified {
                        req = req.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
                    }
                }

                info!("Fetching data from WaniKani...");
                let resp = req.send().await?;

                if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
                    info!("WaniKani data not modified, reusing cached summary");
                    let entry = cached.expect("304 response without a previous cache entry");

                    if let Err(err) = cache.write(
                        DATA_KEY,
                        CachedSummary {
                            expires_at: Utc::now() + ttl,
                            ..entry.clone()
                        },
                    ) {
                        warn!("Failed to write cache for key \"{}\": {}", DATA_KEY, err);
                    }

                    entry.summary
                } else {
                    let resp = resp.error_for_status().inspect_err(|err| {
                        error!("WaniKani API returned an error response: {}", err);
                    })?;

                    let etag = resp
                        .headers()
                        .get(reqwest::header::ETAG)
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_string);
                    let last_modified = resp
                        .headers()
                        .get(reqwest::header::LAST_MODIFIED)
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_string);

                    info!("Successfully fetched data from WaniKani");
                    let summary = resp.json::<SummaryResponse>().await?;

                    if let Err(err) = cache.write(
                        DATA_KEY,
                        CachedSummary {
                            summary: summary.clone(),
                            etag,
                            last_modified,
                            expires_at: Utc::now() + ttl,
                        },
                    ) {
                        warn!("Failed to write cache for key \"{}\": {}", DATA_KEY, err);
                    }

                    summary
                }
            }
        };

        let mut data = build_provider_data(&summary);
        data.action_url = self.action_url.clone();
        Ok(data)
    }
}
