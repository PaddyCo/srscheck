use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, span, warn};

use crate::cache::Cache;

use super::{DataSource, ProviderData};

#[derive(Debug, Deserialize)]
pub struct AnkiProvider {
    /// URL to fetch data from
    url: String,
    /// Name of the deck to fetch data from
    deck: String,
    /// AnkiConnect API key
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeckStatsData {
    deck_id: i64,
    name: String,
    learn_count: i32,
    new_count: i32,
    review_count: i32,
    total_in_deck: i32,
}

#[derive(Debug, Deserialize)]
struct DeckStatsResponse {
    error: Option<String>,
    result: Option<HashMap<String, DeckStatsData>>,
}

#[derive(Debug, Serialize)]
struct DeckStatsRequest {
    action: String,
    version: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,

    params: DeckStatsParams,
}

#[derive(Debug, Serialize)]
struct DeckStatsParams {
    decks: Vec<String>,
}

impl DataSource for AnkiProvider {
    #[instrument(name = "AnkiProvider::get_data", skip(self, cache))]
    async fn get_data(&self, cache: Cache) -> Result<ProviderData, reqwest::Error> {
        info!(
            "Fetching data from AnkiConnect for deck \"{}\"...",
            &self.deck
        );

        let client = reqwest::Client::new();
        let request: DeckStatsRequest = DeckStatsRequest {
            action: "getDeckStats".to_string(),
            version: 6,
            key: self.api_key.clone(),
            params: DeckStatsParams {
                decks: vec![self.deck.clone()],
            },
        };

        let resp = client
            .post(&self.url)
            .body(serde_json::to_string(&request).unwrap())
            .send()
            .await?;
        info!(
            "Successfully fetched data from AnkiConnect for deck {}",
            &self.deck
        );

        let response = resp.json::<DeckStatsResponse>().await?;

        match response.result {
            None => {
                error!("AnkiConnect returned an error: {}", response.error.unwrap());
                Ok(ProviderData {
                    review_count: 0,
                    next_review: None,
                })
            }
            Some(result) => {
                let (_, data) = result.iter().next().unwrap();
                Ok(ProviderData {
                    review_count: data.review_count + data.learn_count,
                    next_review: None,
                })
            }
        }
    }
}
