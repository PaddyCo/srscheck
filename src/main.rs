use std::{collections::HashMap, env};

use chrono::{DateTime, Local, Locale};
use comfy_table::{
    modifiers::{UTF8_ROUND_CORNERS, UTF8_SOLID_INNER_BORDERS},
    presets::UTF8_FULL,
    Attribute, Cell, Color, ContentArrangement, Table,
};
use config::Config;
use log::info;
use providers::{DataSource, ProviderData};
use serde::Serialize;
use sys_locale::get_locale;

mod providers;
mod settings;

#[derive(Debug, Serialize)]
struct ProviderOutput {
    name: String,
    #[serde(flatten)]
    data: ProviderData,
}

#[derive(Debug, Serialize)]
struct Output {
    providers: Vec<ProviderOutput>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = settings::Settings::new()?;

    let mut data: HashMap<&String, ProviderData> = HashMap::new();

    for (name, provider) in &settings.providers {
        let provider_data = match &provider {
            settings::Provider::WaniKani(provider) => provider.get_data().await?,
            settings::Provider::Bunpro(provider) => provider.get_data().await?,
            settings::Provider::Anki(provider) => provider.get_data().await?,
        };

        data.insert(name, provider_data);
    }

    // TODO: Add support for different output formats
    print_table(data, &settings);
    //print_json(data);

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

fn print_json(data: HashMap<&String, ProviderData>) {
    let mut providers: Vec<ProviderOutput> = Vec::new();

    for (name, provider_data) in data {
        providers.push(ProviderOutput {
            name: name.clone(),
            data: provider_data,
        });
    }

    let output = Output { providers };

    // TODO: Handle Timezones?
    // TODO: Add switch for pretty printing
    //let json = serde_json::to_string_pretty(&output).unwrap();
    let json = serde_json::to_string(&output).unwrap();
    println!("{}", json);
}

fn print_table(data: HashMap<&String, ProviderData>, settings: &settings::Settings) {
    let mut table = Table::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let review_threshold = settings.review_threshold;

    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["System", "Reviews", "Next Review"]);

    let locale: Locale = get_time_locale();

    info!("Using locale: {}", locale);

    for (name, provider) in &data {
        let next_review = match provider.next_review {
            Some(date) => date
                .with_timezone(&Local)
                .format_localized("%x %X", locale)
                .to_string(),
            None => "N/A".to_string(),
        };

        rows.push(vec![
            Cell::new(name),
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
        ]);
    }

    table.add_rows(rows);

    let total_review_count = &data
        .iter()
        .fold(0, |acc, (_, data)| acc + data.review_count);
    let total_review_count_color = match total_review_count {
        0 => Color::Green,
        _ => {
            if total_review_count.clone() < review_threshold {
                Color::Yellow
            } else {
                Color::Red
            }
        }
    };
    // Get the lowest next review date time
    let next_review = match total_review_count {
        0 => match data
            .iter()
            .filter(|(_, data)| data.next_review.is_some())
            .min_by_key(|(_, data)| data.next_review)
        {
            Some((_, provider)) => provider
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
        Cell::new(total_review_count)
            .fg(total_review_count_color)
            .add_attribute(Attribute::Bold),
        Cell::new(next_review).add_attribute(Attribute::Bold),
    ]);

    println!("{}", table);
}
