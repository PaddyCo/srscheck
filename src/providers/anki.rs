use std::{collections::HashMap, time::Duration};

use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, span, warn};

use crate::cache::{Cache, DATA_KEY};

use super::{DataSource, ProviderData};

#[derive(Debug, Deserialize)]
pub struct AnkiProvider {
    /// URL to fetch data from
    url: String,
    /// Name of the deck to fetch data from
    deck: String,
    /// AnkiConnect API key
    api_key: Option<String>,
    /// URL to open to do reviews
    action_url: Option<String>,
    /// How long (in seconds) to cache API results for before fetching fresh data
    #[serde(default = "crate::cache::default_cache_expiry::<10>")]
    cache_expiry: u64,
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
        let url = self.url.clone();
        let deck = self.deck.clone();
        let api_key = self.api_key.clone();
        let ttl = Duration::from_secs(self.cache_expiry);

        let mut data = cache
            .get_or_fetch(DATA_KEY, ttl, || async move {
                info!("Fetching data from AnkiConnect for deck \"{}\"...", &deck);

                let client = reqwest::Client::new();
                let request: DeckStatsRequest = DeckStatsRequest {
                    action: "getDeckStats".to_string(),
                    version: 6,
                    key: api_key,
                    params: DeckStatsParams {
                        decks: vec![deck.clone()],
                    },
                };

                let resp = client
                    .post(&url)
                    .body(serde_json::to_string(&request).unwrap())
                    .send()
                    .await?;
                info!("Successfully fetched data from AnkiConnect for deck {}", &deck);

                let response = resp.json::<DeckStatsResponse>().await?;

                let review_count = match response.result {
                    None => {
                        error!("AnkiConnect returned an error: {}", response.error.unwrap());
                        0
                    }
                    Some(result) => {
                        let (_, data) = result.iter().next().unwrap();
                        data.review_count + data.learn_count
                    }
                };

                Ok(ProviderData {
                    review_count,
                    next_review: None,
                    action_url: None,
                })
            })
            .await?;

        data.action_url = self.action_url.clone();
        Ok(data)
    }
}
