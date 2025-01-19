use std::{collections::HashMap, path::PathBuf};

use config::{Config, ConfigError};
use serde::Deserialize;

use crate::providers::{
    anki::AnkiProvider, bunpro::BunproProvider, wanikani::WanikaniProvider, DataSource,
    ProviderData,
};

fn default_cache_path() -> PathBuf {
    match dirs::cache_dir() {
        Some(path) => path.join("srscheck-cache.toml"),
        None => {
            eprintln!("Could not find cache directory!");
            eprintln!("Please specify the cache path in the config file");
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Provider {
    Wanikani(WanikaniProvider),
    Bunpro(BunproProvider),
    Anki(AnkiProvider),
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
    pub fn new() -> Result<Self, ConfigError> {
        // TODO: Allow the user to specify the config path
        let config_path = match dirs::config_dir() {
            Some(path) => path.join("srscheck.toml"),
            None => {
                eprintln!("Could not find config directory");
                std::process::exit(1);
            }
        };

        // Check if config exists
        if !config_path.exists() {
            eprintln!("Config file not found at ~/.config/srscheck.toml");
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
