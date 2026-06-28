use std::{collections::BTreeMap, env, path::PathBuf};

use chrono::{DateTime, Local, Locale};
use clap::{command, Parser, ValueEnum};
use clap_verbosity_flag::Verbosity;
use comfy_table::{
    modifiers::{UTF8_ROUND_CORNERS, UTF8_SOLID_INNER_BORDERS},
    presets::UTF8_FULL,
    Attribute, Cell, Color, ContentArrangement, Table,
};
use config::Config;
use providers::{DataSource, ProviderData};
use serde::Serialize;
use sys_locale::get_locale;
use tracing::{info, warn};

mod cache;
mod providers;
mod settings;

#[derive(Debug, Serialize)]
#[serde(tag = "status")]
enum ProviderOutput {
    #[serde(rename = "OK")]
    Ok {
        name: String,
        #[serde(flatten)]
        data: ProviderData,
    },
    #[serde(rename = "Error")]
    Error {
        name: String,
        error: String,
    },
}

#[derive(Debug, Serialize)]
struct Output {
    providers: Vec<ProviderOutput>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputType {
    Json,
    Table,
}

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[command(flatten)]
    verbosity: Verbosity,

    #[arg(short, long, value_enum, default_value = "table")]
    output: Option<OutputType>,

    #[arg(long, help = "Pretty print JSON output")]
    pretty: bool,

    #[arg(long, help = "Path to the config file")]
    config_path: Option<PathBuf>,

    #[arg(
        long,
        help = "Ignore our cache expiry and force fetching fresh data from providers"
    )]
    force_fetch: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.verbosity)
        .init();

    let settings = match args.config_path {
        Some(path) => settings::Settings::new(path)?,
        None => settings::Settings::from_default_path()?,
    };

    let mut data: BTreeMap<&String, Result<ProviderData, String>> = BTreeMap::new();

    for (name, provider) in &settings.providers {
        let cache = cache::Cache::new(name, &settings, args.force_fetch)?;
        // TODO: Run in parallel
        let result = match &provider {
            settings::Provider::WaniKani(provider) => provider.get_data(cache).await.map_err(|e| e.to_string()),
            settings::Provider::Bunpro(provider) => provider.get_data(cache).await.map_err(|e| e.to_string()),
            settings::Provider::Anki(provider) => provider.get_data(cache).await.map_err(|e| e.to_string()),
            settings::Provider::KameSame(provider) => provider.get_data(cache).await.map_err(|e| e.to_string()),
            settings::Provider::NativShark(provider) => provider.get_data(cache).await.map_err(|e| e.to_string()),
            settings::Provider::Http(provider) => provider.get_data(cache).await.map_err(|e| e.to_string()),
        };

        if let Err(ref e) = result {
            warn!("Provider {} failed: {}", name, e);
        }

        data.insert(name, result);
    }

    match args.output {
        Some(OutputType::Json) => print_json(data, args.pretty),
        Some(OutputType::Table) | None => print_table(data, &settings),
    }


    Ok(())
}

fn get_time_locale() -> Locale {
    let locale = match env::var("LC_TIME") {
        Ok(val) => Some(val.split('.').next().unwrap().to_string()),
        Err(_) => get_locale(),
    };

    match locale {
        Some(locale) => locale.as_str().try_into().unwrap_or(Locale::POSIX),
        None => Locale::POSIX,
    }
}

fn print_json(data: BTreeMap<&String, Result<ProviderData, String>>, pretty: bool) {
    let mut providers: Vec<ProviderOutput> = Vec::new();

    for (name, result) in data {
        providers.push(match result {
            Ok(provider_data) => ProviderOutput::Ok {
                name: name.clone(),
                data: provider_data,
            },
            Err(error) => ProviderOutput::Error {
                name: name.clone(),
                error,
            },
        });
    }

    let output = Output { providers };

    // TODO: Handle Timezones?
    let json = match pretty {
        true => serde_json::to_string_pretty(&output).unwrap(),
        false => serde_json::to_string(&output).unwrap(),
    };
    println!("{}", json);
}

fn print_table(data: BTreeMap<&String, Result<ProviderData, String>>, settings: &settings::Settings) {
    let mut table = Table::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let review_threshold = settings.review_threshold;

    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["System", "Status", "Reviews", "Next Review", "URL"]);

    let locale: Locale = get_time_locale();

    info!("Using locale: {}", locale);

    for (name, result) in &data {
        match result {
            Ok(provider) => {
                let next_review = match provider.next_review {
                    Some(date) => date
                        .with_timezone(&Local)
                        .format_localized("%x %X", locale)
                        .to_string(),
                    None => "N/A".to_string(),
                };

                rows.push(vec![
                    Cell::new(name),
                    Cell::new("OK").fg(Color::Green),
                    Cell::new(provider.review_count.to_string()).fg(match provider.review_count {
                        0 => Color::Green,
                        _ => {
                            if provider.review_count < review_threshold {
                                Color::Yellow
                            } else {
                                Color::Red
                            }
                        }
                    }),
                    Cell::new(match provider.review_count {
                        0 => next_review,
                        _ => "Now".to_string(),
                    }),
                    Cell::new(provider.action_url.clone().unwrap_or_default()),
                ]);
            }
            Err(_) => {
                rows.push(vec![
                    Cell::new(name),
                    Cell::new("Error").fg(Color::Red),
                    Cell::new("N/A"),
                    Cell::new("N/A"),
                    Cell::new(""),
                ]);
            }
        }
    }

    table.add_rows(rows);

    let total_review_count = data
        .iter()
        .fold(0, |acc, (_, result)| acc + result.as_ref().map(|d| d.review_count).unwrap_or(0));
    let total_review_count_color = match total_review_count {
        0 => Color::Green,
        _ => {
            if total_review_count < review_threshold {
                Color::Yellow
            } else {
                Color::Red
            }
        }
    };
    // Get the lowest next review date time across successful providers
    let next_review = match total_review_count {
        0 => match data
            .iter()
            .filter_map(|(_, result)| result.as_ref().ok())
            .filter(|d| d.next_review.is_some())
            .min_by_key(|d| d.next_review)
        {
            Some(provider) => provider
                .next_review
                .as_ref()
                .unwrap()
                .with_timezone(&Local)
                .format_localized("%x %X", locale)
                .to_string(),
            None => "N/A".to_string(),
        },
        _ => "Now".to_string(),
    };

    table.add_row(vec![
        Cell::new("Total").add_attribute(Attribute::Bold),
        Cell::new("").add_attribute(Attribute::Bold),
        Cell::new(total_review_count)
            .fg(total_review_count_color)
            .add_attribute(Attribute::Bold),
        Cell::new(next_review).add_attribute(Attribute::Bold),
        Cell::new(""),
    ]);

    println!("{}", table);
}
