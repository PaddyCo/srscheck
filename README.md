# srscheck

CLI tool for quickly getting the status of multiple SRS systems.

## Usage

```shell
srscheck
```

TODO: Add more usage information once we have added actual switches to the CLI.

## Example output

```
╭──────────┬─────────┬─────────────────────╮
│ System   ┆ Reviews ┆ Next Review         │
╞══════════╪═════════╪═════════════════════╡
│ Bunpro   ┆ 0       ┆ 2025-01-19 12:00:00 │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Anki     ┆ 0       ┆ N/A                 │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ WaniKani ┆ 0       ┆ 2025-01-19 15:00:00 │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Total    ┆ 0       ┆ 2025-01-19 12:00:00 │
╰──────────┴─────────┴─────────────────────╯
```

## Configuration

By default it will look for the config file `srscheck.toml` the following directories:

```
|Platform | Value                                 | Example                                  |
| ------- | ------------------------------------- | ---------------------------------------- |
| Linux   | `$XDG_CONFIG_HOME` or `$HOME`/.config | /home/alice/.config                      |
| macOS   | `$HOME`/Library/Application Support   | /Users/Alice/Library/Application Support |
| Windows | `{FOLDERID_RoamingAppData}`           | C:\Users\Alice\AppData\Roaming           |
```

TODO: Document how to override the config file path.

## Example configuration

```toml
review_threshold = 100 # The threshold for the review count to be considered high

# Your SRS providers
[providers."MyProvider"]
type = "MyProvider"
api_key = "my-secret-key"

[providers."AnotherProvider"]
type = "AnotherProvider"
url = "http://localhost:1234"
```

For each provider you need to specify the type of the provider, and the fields that are required for that provider.
See the [Supported providers](#supported-providers) below for more information.

## Supported Providers

### Anki (AnkiConnect)

Example config:
```toml
[providers."Anki"] # The name of the provider, this can be any string (but has to be unique)
type = "Anki" # The type of the provider, this has to be "Anki"
url = "http://localhost:8765" # The URL of the AnkiConnect server
api_key = "My secret api key" # OPTIONAL: the API key you set in the AnkiConnect settings
# Deck is the name of the deck you want to check.
# You can target a parent deck, and it will include the counts of all subdecks.
# If you want to target a subdeck, you can use the full path, e.g. "Japanese::Kanji"
deck = "Japanese" 
```

### WaniKani

Example config:
```toml
[providers."WaniKani"] # The name of the provider, this can be any string (but has to be unique)
type = "WaniKani" # The type of the provider, this has to be "WaniKani"
api_key = "your-key" # The API key you get from the WaniKani settings. Read-only access is enough.
```

### Bunpro

Example config:
```toml
[providers."Bunpro"] # The name of the provider, this can be any string (but has to be unique) 
type = "Bunpro" # The type of the provider, this has to be "Bunpro"
api_key = "your-key" # The API key you get from the Bunpro settings.
```

## Example uses

TODO: Add some example use cases.

- TODO: Simple bash script that runs `srscheck` and sends a notification if there are reviews due.

## Development

### Roadmap

- [ ] Add support for KameSame
- [ ] Add support for Anki (SQL) 

### How to add a new provider

1. Create a new file in the `providers` directory with the name of the provider.
2. Look at the existing providers for reference, and implement the "DataSource" trait for your provider.
3. The fields in the Provider struct are the ones that gets parsed from the Config
4. Add your provider to the `Provider` enum in `src/settings.rs`.
5. Add your provider to the match statement in `src/main.rs`
6. Add your provider to the README.md
7. (Optional) Create a PR :)
