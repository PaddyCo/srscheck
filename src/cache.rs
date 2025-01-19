use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};
use tracing::{debug, info, instrument};

use crate::{
    providers::kamesame::{KameSameCache, KameSameProvider},
    settings::Settings,
};

pub enum ProviderCache {
    WaniKani,
    Bunpro,
    Anki,
    KameSame(KameSameCache),
}

#[derive(Debug, Deserialize)]
pub struct Cache {
    cache_path: PathBuf,
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
            fs::create_dir_all(&cache_dir).unwrap();
            debug!("Created cache directory at \"{:?}\"", cache_dir);
        }

        let cache_path = cache_dir.join(format!("provider.{}.toml", provider));

        Ok(Cache { cache_path })
    }

    #[instrument(name = "read", skip(self))]
    pub fn read<T: DeserializeOwned>(self: &Self) -> Result<Option<T>, Box<dyn std::error::Error>> {
        let cache_file = &self.cache_path;

        // Check if cache file exists:
        if !cache_file.exists() {
            debug!("Cache file not found: \"{:?}\"", cache_file);
            return Ok(None);
        }

        // Read cache file
        let cache = fs::read_to_string(&cache_file).unwrap();
        // Deserialize cache file
        let cache: T = toml::from_str(&cache).unwrap();

        Ok(Some(cache))
    }

    #[instrument(name = "write", skip(self, data))]
    pub fn write<T: Serialize>(self: &Self, data: T) -> Result<(), Box<dyn std::error::Error>> {
        let cache_file = &self.cache_path;

        debug!("Writing data to cache file: \"{:?}\"", cache_file);

        // Serialize data
        let data = toml::to_string(&data).unwrap();
        // Write data to cache file
        fs::write(&cache_file, data).unwrap();

        Ok(())
    }
}
