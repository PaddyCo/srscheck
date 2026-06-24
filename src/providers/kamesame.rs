use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use cookie::Expiration;
use reqwest::{
    cookie::{Cookie, Jar},
    Url,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};

use crate::cache::{Cache, DATA_KEY};

use super::{DataSource, ProviderData};

fn default_action_url() -> Option<String> {
    Some("https://www.kamesame.com/".to_string())
}

const SESSION_KEY: &str = "session";

#[derive(Debug, Deserialize)]
pub struct KameSameProvider {
    email: String,
    password: String,
    /// URL to open to do reviews
    #[serde(default = "default_action_url")]
    action_url: Option<String>,
    /// How long (in seconds) to cache API results for before fetching fresh data
    #[serde(default = "crate::cache::default_cache_expiry::<300>")]
    cache_expiry: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KameSameCache {
    session: String,
    session_expiry: SystemTime,
}

#[derive(Debug, Serialize)]
struct LoginRequestBody {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponseBody {
    error_messages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    reviews_status: ReviewsStatus,
}

#[derive(Debug, Deserialize)]
struct ReviewsStatus {
    ready_for_review: i32,
    next_hour: i32,
    next_day: i32,
}

impl DataSource for KameSameProvider {
    #[instrument(name = "KameSameProvider::get_data", skip(self, cache))]
    async fn get_data(&self, cache: Cache) -> Result<ProviderData, reqwest::Error> {
        let valid_session = cache
            .read::<KameSameCache>(SESSION_KEY)
            .unwrap_or(None)
            .filter(|cache_data| cache_data.session_expiry > SystemTime::now());
        let has_session = valid_session.is_some();

        let jar = Jar::default();
        if let Some(cache_data) = &valid_session {
            debug!("Adding session cookie to cookie jar...");
            jar.add_cookie_str(
                &format!("_kamesame_session={}", cache_data.session),
                &Url::from_str("https://www.kamesame.com").unwrap(),
            );
        }

        let cookies = Arc::new(jar);
        let client = reqwest::Client::builder()
            .cookie_provider(cookies)
            .build()?;
        info!("Fetching data from KameSame...");

        if !has_session {
            info!("Logging in to KameSame...");
            let body = LoginRequestBody {
                email: self.email.clone(),
                password: self.password.clone(),
            };
            let login = client
                .post("https://www.kamesame.com/api/sessions")
                .json(&body)
                .send()
                .await?;

            let success = login.status().is_success();
            let headers = login.headers().clone();
            let login_response = login.json::<LoginResponseBody>().await?;

            if !success || !login_response.error_messages.is_empty() {
                error!("Login failed: {:?}", &login_response.error_messages);
                return Ok(ProviderData {
                    review_count: 0,
                    next_review: None,
                    action_url: self.action_url.clone(),
                });
            }

            // Cache session
            let cookie =
                cookie::Cookie::parse(headers.get("Set-Cookie").unwrap().to_str().unwrap())
                    .unwrap();
            let cache_data = KameSameCache {
                session: cookie.value().to_string(),
                session_expiry: match cookie.expires() {
                    Some(expiry) => match expiry {
                        Expiration::DateTime(expires) => expires.into(),
                        Expiration::Session => SystemTime::now()
                            .checked_sub(Duration::from_secs(60 * 60 * 24))
                            .unwrap(),
                    },
                    None => SystemTime::now()
                        .checked_sub(Duration::from_secs(60 * 60 * 24))
                        .unwrap(),
                },
            };
            if let Err(err) = cache.write(SESSION_KEY, cache_data) {
                warn!("Failed to write session cache: {}", err);
            }
        }

        let ttl = Duration::from_secs(self.cache_expiry);
        let mut data = cache
            .get_or_fetch(DATA_KEY, ttl, || async move {
                let resp = client
                    .get("https://www.kamesame.com/api/reviews/status")
                    .send()
                    .await?;

                let status = resp.json::<StatusResponse>().await?;

                Ok(ProviderData {
                    review_count: status.reviews_status.ready_for_review,
                    next_review: None,
                    action_url: None,
                })
            })
            .await?;

        data.action_url = self.action_url.clone();
        Ok(data)
    }
}
