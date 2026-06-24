use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Utc};
use jaq_core::load::{Arena, File, Loader};
use jaq_core::{data, unwrap_valr, Compiler, Ctx, Vars};
use jaq_json::{read, Val};
use reqwest::Method;
use serde::Deserialize;
use tracing::{error, info, instrument, warn};

use crate::cache::{Cache, DATA_KEY};

use super::{DataSource, ProviderData};

fn default_method() -> String {
    "GET".to_string()
}

#[derive(Debug, Deserialize)]
pub struct HttpProvider {
    /// URL to send the request to
    url: String,
    /// HTTP method to use for the request (GET, POST, PUT, DELETE, etc.)
    #[serde(default = "default_method")]
    method: String,
    /// Headers to include in the request
    #[serde(default)]
    headers: HashMap<String, String>,
    /// jq filter used to extract the review count from the response, e.g. `.reviews.pending_reviews`
    review_count_path: String,
    /// jq filter used to extract the next review date from the response, e.g. `.reviews.next_review`.
    /// The matched value can be either an RFC 3339 string or a Unix timestamp.
    next_review_path: Option<String>,
    /// URL to open to do reviews
    action_url: Option<String>,
    /// How long (in seconds) to cache API results for before fetching fresh data
    #[serde(default = "crate::cache::default_cache_expiry::<60>")]
    cache_expiry: u64,
}

/// Run a jq filter against a value, returning its first output (if any).
fn run_jq(code: &str, input: &Val) -> Result<Option<Val>, String> {
    let defs = jaq_core::defs()
        .chain(jaq_std::defs())
        .chain(jaq_json::defs());
    let funs = jaq_core::funs()
        .chain(jaq_std::funs())
        .chain(jaq_json::funs());

    let loader = Loader::new(defs);
    let arena = Arena::default();

    let modules = loader
        .load(&arena, File { code, path: () })
        .map_err(|e| format!("{e:?}"))?;

    let filter = Compiler::default()
        .with_funs(funs)
        .compile(modules)
        .map_err(|e| format!("{e:?}"))?;

    let ctx = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]));
    let mut out = filter.id.run((ctx, input.clone())).map(unwrap_valr);

    match out.next() {
        Some(Ok(val)) => Ok(Some(val)),
        Some(Err(e)) => Err(e.to_string()),
        None => Ok(None),
    }
}

fn val_to_review_count(val: &Val) -> Option<i32> {
    match val {
        Val::Num(n) => n.as_isize().map(|i| i as i32),
        _ => None,
    }
}

fn val_to_next_review(val: &Val) -> Option<DateTime<Utc>> {
    match val {
        Val::Num(n) => n
            .as_isize()
            .and_then(|i| DateTime::from_timestamp(i as i64, 0)),
        Val::TStr(b) | Val::BStr(b) => std::str::from_utf8(b)
            .ok()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc)),
        _ => None,
    }
}

impl DataSource for HttpProvider {
    #[instrument(name = "HttpProvider::get_data", skip(self, cache))]
    async fn get_data(&self, cache: Cache) -> Result<ProviderData, reqwest::Error> {
        let url = self.url.clone();
        let method = self.method.clone();
        let headers = self.headers.clone();
        let review_count_path = self.review_count_path.clone();
        let next_review_path = self.next_review_path.clone();
        let ttl = Duration::from_secs(self.cache_expiry);

        let mut data = cache
            .get_or_fetch(DATA_KEY, ttl, || async move {
                let client = reqwest::Client::new();

                let parsed_method = Method::from_bytes(method.as_bytes()).unwrap_or_else(|_| {
                    warn!("Invalid HTTP method \"{}\", defaulting to GET", method);
                    Method::GET
                });

                info!("Fetching data from {}...", url);

                let mut request = client.request(parsed_method, &url);
                for (key, value) in &headers {
                    request = request.header(key, value);
                }

                let resp = request.send().await?;
                let body = resp.bytes().await?;

                let value = match read::parse_single(&body) {
                    Ok(value) => value,
                    Err(e) => {
                        error!("Failed to parse response from {} as JSON: {}", url, e);
                        return Ok(ProviderData {
                            review_count: 0,
                            next_review: None,
                            action_url: None,
                        });
                    }
                };

                let review_count = match run_jq(&review_count_path, &value) {
                    Ok(Some(val)) => val_to_review_count(&val),
                    Ok(None) => {
                        warn!(
                            "review_count_path \"{}\" did not match any value",
                            review_count_path
                        );
                        None
                    }
                    Err(e) => {
                        error!(
                            "Failed to evaluate review_count_path \"{}\": {}",
                            review_count_path, e
                        );
                        None
                    }
                };

                let next_review = match &next_review_path {
                    Some(path) => match run_jq(path, &value) {
                        Ok(Some(val)) => val_to_next_review(&val),
                        Ok(None) => None,
                        Err(e) => {
                            warn!("Failed to evaluate next_review_path \"{}\": {}", path, e);
                            None
                        }
                    },
                    None => None,
                };

                Ok(ProviderData {
                    review_count: review_count.unwrap_or(0),
                    next_review,
                    action_url: None,
                })
            })
            .await?;

        data.action_url = self.action_url.clone();
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Val {
        read::parse_single(
            br#"{"reviews": {"pending_reviews": 120, "next_review": "2023-09-15T12:00:00Z", "epoch": 1694779200}}"#,
        )
        .unwrap()
    }

    #[test]
    fn extracts_review_count() {
        let val = run_jq(".reviews.pending_reviews", &sample())
            .unwrap()
            .unwrap();
        assert_eq!(val_to_review_count(&val), Some(120));
    }

    #[test]
    fn extracts_next_review_from_rfc3339() {
        let val = run_jq(".reviews.next_review", &sample()).unwrap().unwrap();
        assert_eq!(
            val_to_next_review(&val),
            Some(
                DateTime::parse_from_rfc3339("2023-09-15T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc)
            )
        );
    }

    #[test]
    fn extracts_next_review_from_epoch() {
        let val = run_jq(".reviews.epoch", &sample()).unwrap().unwrap();
        assert_eq!(
            val_to_next_review(&val),
            Some(
                DateTime::parse_from_rfc3339("2023-09-15T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc)
            )
        );
    }

    #[test]
    fn missing_path_returns_null_not_error() {
        let val = run_jq(".reviews.missing", &sample()).unwrap().unwrap();
        assert_eq!(val_to_review_count(&val), None);
    }

    #[test]
    fn invalid_filter_syntax_errors() {
        assert!(run_jq(".reviews..bad[", &sample()).is_err());
    }
}
