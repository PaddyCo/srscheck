use std::{cell::RefCell, collections::HashMap, fs, future::Future, path::PathBuf, time::Duration};

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::{debug, instrument, warn};

use crate::settings::Settings;

/// Cache key used to store a provider's fetched API data.
pub const DATA_KEY: &str = "data";

/// Generic serde default-value helper for `cache_expiry` fields, e.g.
/// `#[serde(default = "crate::cache::default_cache_expiry::<300>")]`.
pub fn default_cache_expiry<const SECS: u64>() -> u64 {
    SECS
}

#[derive(Debug, Deserialize)]
pub struct Cache {
    cache_path: PathBuf,
    /// In-memory copy of the cache file, populated on first access so that
    /// repeated reads/writes within one provider call don't re-read and
    /// re-parse the file from disk each time.
    #[serde(skip)]
    loaded: RefCell<Option<HashMap<String, toml::Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedEntry<T> {
    data: T,
    expires_at: DateTime<Utc>,
}

impl Cache {
    #[instrument(name = "new", skip(settings))]
    pub fn new(provider: &str, settings: &Settings) -> Result<Self, Box<dyn std::error::Error>> {
        let cache_dir = &settings.cache_path;
        debug!("Cache path: \"{:?}\"...", cache_dir);

        // Check if cache directory exists
        if !cache_dir.exists() {
            // Create cache directory
            debug!("Cache directory not found, creating it...");
            fs::create_dir_all(cache_dir)?;
            debug!("Created cache directory at \"{:?}\"", cache_dir);
        }

        let cache_path = cache_dir.join(format!("provider.{}.toml", provider));

        Ok(Cache {
            cache_path,
            loaded: RefCell::new(None),
        })
    }

    /// Load the full cache file, mapping cache keys to their raw stored value.
    /// Each provider can store multiple independent entries (e.g. a login session
    /// alongside cached API results) in the same file, keyed by name. The parsed
    /// map is cached in memory after the first call.
    fn load(&self) -> HashMap<String, toml::Value> {
        if let Some(map) = self.loaded.borrow().as_ref() {
            return map.clone();
        }

        let map = if !self.cache_path.exists() {
            debug!("Cache file not found: \"{:?}\"", self.cache_path);
            HashMap::new()
        } else {
            match fs::read_to_string(&self.cache_path) {
                Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
                Err(err) => {
                    warn!("Failed to read cache file \"{:?}\": {}", self.cache_path, err);
                    HashMap::new()
                }
            }
        };

        *self.loaded.borrow_mut() = Some(map.clone());
        map
    }

    #[instrument(name = "read", skip(self))]
    pub fn read<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Box<dyn std::error::Error>> {
        match self.load().remove(key) {
            Some(value) => Ok(Some(value.try_into()?)),
            None => Ok(None),
        }
    }

    #[instrument(name = "write", skip(self, data))]
    pub fn write<T: Serialize>(
        &self,
        key: &str,
        data: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut map = self.load();
        map.insert(key.to_string(), toml::Value::try_from(data)?);

        debug!("Writing data to cache file: \"{:?}\"", self.cache_path);
        let contents = toml::to_string(&map)?;
        fs::write(&self.cache_path, contents)?;

        *self.loaded.borrow_mut() = Some(map);

        Ok(())
    }

    /// Return the cached value for `key` if present and not yet expired. Otherwise call
    /// `fetch`, cache its result with an expiration of `ttl` from now, and return it.
    #[instrument(name = "get_or_fetch", skip(self, fetch))]
    pub async fn get_or_fetch<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        fetch: F,
    ) -> Result<T, reqwest::Error>
    where
        T: Clone + Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, reqwest::Error>>,
    {
        if let Ok(Some(entry)) = self.read::<CachedEntry<T>>(key) {
            if entry.expires_at > Utc::now() {
                debug!("Using cached data for key \"{}\"", key);
                return Ok(entry.data);
            }
            debug!("Cached data for key \"{}\" expired", key);
        }

        let data = fetch().await?;

        let entry = CachedEntry {
            data: data.clone(),
            expires_at: Utc::now()
                + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::zero()),
        };
        if let Err(err) = self.write(key, entry) {
            warn!("Failed to write cache for key \"{}\": {}", key, err);
        }

        Ok(data)
    }
}
