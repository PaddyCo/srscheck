use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, warn};

use crate::cache::{Cache, DATA_KEY};

use super::{DataSource, ProviderData};

const API_URL: &str = "https://api.nativshark.com/api";
const TOKEN_KEY: &str = "token";
/// NativShark tokens are valid for about a month; refresh a bit early to be safe.
const TOKEN_LIFETIME: Duration = Duration::from_secs(28 * 24 * 60 * 60);

fn default_action_url() -> Option<String> {
    Some("https://app.nativshark.com/".to_string())
}

#[derive(Debug, Deserialize)]
pub struct NativSharkProvider {
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
struct TokenCache {
    token: String,
    expiry: SystemTime,
}

#[derive(Debug, Deserialize)]
struct GraphQlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    query: &'a str,
    variables: LoginVariables<'a>,
}

#[derive(Debug, Serialize)]
struct LoginVariables<'a> {
    #[serde(rename = "signIn")]
    sign_in: SignInInput<'a>,
}

#[derive(Debug, Serialize)]
struct SignInInput<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Debug, Deserialize)]
struct SignInData {
    #[serde(rename = "signIn")]
    sign_in: Option<SignInPayload>,
}

#[derive(Debug, Deserialize)]
struct SignInPayload {
    errors: Option<Vec<GraphQlError>>,
    result: Option<SignInResult>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
enum SignInResult {
    User { token: String },
    MfaRequired {},
}

const SIGN_IN_QUERY: &str = r#"
mutation ($signIn: SignInInput!) {
  signIn(input: $signIn) {
    errors {
      key
      message
    }
    result {
      __typename
      ... on User {
        token
      }
    }
  }
}
"#;

#[derive(Debug, Serialize)]
struct StudiesRequest<'a> {
    query: &'a str,
}

const STUDIES_QUERY: &str = r#"
{
  getTodaysStudies {
    systemReviews {
      remaining {
        lesson
        kanji
        vocabulary
      }
    }
  }
}
"#;

#[derive(Debug, Deserialize)]
struct StudiesData {
    #[serde(rename = "getTodaysStudies")]
    get_todays_studies: GetTodaysStudies,
}

#[derive(Debug, Deserialize)]
struct GetTodaysStudies {
    #[serde(rename = "systemReviews")]
    system_reviews: SystemReviews,
}

#[derive(Debug, Deserialize)]
struct SystemReviews {
    remaining: ReviewCounts,
}

#[derive(Debug, Deserialize)]
struct ReviewCounts {
    lesson: i32,
    kanji: i32,
    vocabulary: i32,
}

fn empty_data(action_url: Option<String>) -> ProviderData {
    ProviderData {
        review_count: 0,
        next_review: None,
        action_url,
    }
}

fn store_token(cache: &Cache, token: &str) {
    let cache_data = TokenCache {
        token: token.to_string(),
        expiry: SystemTime::now() + TOKEN_LIFETIME,
    };
    if let Err(err) = cache.write(TOKEN_KEY, cache_data) {
        warn!("Failed to write token cache: {}", err);
    }
}

async fn login(
    client: &reqwest::Client,
    email: &str,
    password: &str,
) -> Result<Option<String>, reqwest::Error> {
    let body = LoginRequest {
        query: SIGN_IN_QUERY,
        variables: LoginVariables {
            sign_in: SignInInput { email, password },
        },
    };

    let resp: GraphQlResponse<SignInData> = client
        .post(API_URL)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if let Some(errors) = &resp.errors {
        error!(
            "NativShark login failed: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        return Ok(None);
    }

    let sign_in = match resp.data.and_then(|d| d.sign_in) {
        Some(sign_in) => sign_in,
        None => {
            error!("NativShark login returned no data");
            return Ok(None);
        }
    };

    if let Some(errors) = &sign_in.errors {
        if !errors.is_empty() {
            error!(
                "NativShark login failed: {:?}",
                errors.iter().map(|e| &e.message).collect::<Vec<_>>()
            );
            return Ok(None);
        }
    }

    match sign_in.result {
        Some(SignInResult::User { token }) => Ok(Some(token)),
        Some(SignInResult::MfaRequired {}) => {
            error!("NativShark account requires MFA, which is not supported");
            Ok(None)
        }
        None => {
            error!("NativShark login returned no result");
            Ok(None)
        }
    }
}

async fn fetch_studies(
    client: &reqwest::Client,
    token: &str,
) -> Result<Option<StudiesData>, reqwest::Error> {
    let body = StudiesRequest {
        query: STUDIES_QUERY,
    };

    let resp: GraphQlResponse<StudiesData> = client
        .post(API_URL)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if let Some(errors) = &resp.errors {
        warn!(
            "NativShark API returned errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        return Ok(None);
    }

    Ok(resp.data)
}

impl DataSource for NativSharkProvider {
    #[instrument(name = "NativSharkProvider::get_data", skip(self, cache))]
    async fn get_data(&self, cache: Cache) -> Result<ProviderData, reqwest::Error> {
        let client = reqwest::Client::new();

        let token = match cache
            .read::<TokenCache>(TOKEN_KEY)
            .unwrap_or(None)
            .filter(|cached| cached.expiry > SystemTime::now())
            .map(|cached| cached.token)
        {
            Some(token) => token,
            None => {
                info!("Logging in to NativShark...");
                match login(&client, &self.email, &self.password).await? {
                    Some(token) => {
                        store_token(&cache, &token);
                        token
                    }
                    None => return Ok(empty_data(self.action_url.clone())),
                }
            }
        };

        let ttl = Duration::from_secs(self.cache_expiry);
        let cache_ref = &cache;
        let mut data = cache
            .get_or_fetch(DATA_KEY, ttl, move || async move {
                info!("Fetching data from NativShark...");
                let mut studies = fetch_studies(&client, &token).await?;

                if studies.is_none() {
                    info!("NativShark token rejected, logging in again...");
                    if let Some(new_token) = login(&client, &self.email, &self.password).await? {
                        store_token(cache_ref, &new_token);
                        studies = fetch_studies(&client, &new_token).await?;
                    }
                }

                let remaining = studies
                    .map(|s| s.get_todays_studies.system_reviews.remaining)
                    .unwrap_or(ReviewCounts {
                        lesson: 0,
                        kanji: 0,
                        vocabulary: 0,
                    });

                Ok(ProviderData {
                    review_count: remaining.lesson + remaining.kanji + remaining.vocabulary,
                    next_review: None,
                    action_url: None,
                })
            })
            .await?;

        data.action_url = self.action_url.clone();
        Ok(data)
    }
}
