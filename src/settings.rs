use std::{collections::HashMap, path::PathBuf};

use config::{Config, ConfigError};
use serde::Deserialize;
use tracing::error;

use crate::providers::{
    anki::AnkiProvider, bunpro::BunproProvider, kamesame::KameSameProvider,
    wanikani::WaniKaniProvider, DataSource, ProviderData,
};

fn default_cache_path() -> PathBuf {
    match dirs::cache_dir() {
        Some(path) => path.join("srscheck"),
        None => {
            error!("Could not find cache directory!");
            error!("Please specify the cache directory path in the config file (`cache_path`)");
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Provider {
    WaniKani(WaniKaniProvider),
    Bunpro(BunproProvider),
    Anki(AnkiProvider),
    KameSame(KameSameProvider),
}

fn default_review_threshold() -> i32 {
    100
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default = "default_cache_path")]
    pub cache_path: PathBuf,
    #[serde(default = "default_review_threshold")]
    pub review_threshold: i32,

    /// Data sources to fetch data from
    pub providers: HashMap<String, Provider>,
}

impl Settings {
    pub fn from_default_path() -> Result<Self, ConfigError> {
        let config_path = match dirs::config_dir() {
            Some(path) => path.join("srscheck.toml"),
            None => {
                eprintln!("Could not find config directory");
                std::process::exit(1);
            }
        };
        Settings::new(config_path)
    }

    pub fn new(config_path: PathBuf) -> Result<Self, ConfigError> {
        // Check if config exists
        if !config_path.exists() {
            error!("Config file not found at {}", config_path.display());
            std::process::exit(1);
        }

        let settings = Config::builder()
            .add_source(config::File::from(config_path))
            .add_source(config::Environment::with_prefix("SRSCHECK"))
            .build()
            .unwrap();

        settings.try_deserialize()
    }
}
