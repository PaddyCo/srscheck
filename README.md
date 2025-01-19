# srscheck

CLI tool for quickly getting the review counts from multiple SRS systems.

## Note

- This is a work in progress, and mostly built for personal use. (But should be usable)

- Only tested on Linux at the moment, but should be painless to get running on other platforms as I'm not using any platform-specific code (as far as I know).

- There is minimal caching, so it will make a request to each provider every time it is run, so be careful with the rate limits of the APIs you are using.

- Error handling is minimal at the moment, so if something goes wrong, a provider might silently fail and return a review count of 0. (It will log a warning to the console, but that's not helpful if you are running this in a script for example)


## Usage

```shell
$ srscheck --help

Usage: srscheck [OPTIONS]

Options:
  -v, --verbose...                 Increase logging verbosity
  -q, --quiet...                   Decrease logging verbosity
  -o, --output <OUTPUT>            [default: table] [possible values: json, table]
      --pretty                     Pretty print JSON output
      --config-path <CONFIG_PATH>  Path to the config file
  -h, --help                       Print help
  -V, --version                    Print version
```

Increase the verbosity with additional `-v` flags. (e.g. `-vv`, `-vvv`, etc)


## Example output

```
$ srscheck

╭──────────┬─────────┬─────────────────────╮
│ System   ┆ Reviews ┆ Next Review         │
╞══════════╪═════════╪═════════════════════╡
│ WaniKani ┆ 0       ┆ 2025-01-19 15:00:00 │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Anki     ┆ 12      ┆ Now                 │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ KameSame ┆ 42      ┆ Now                 │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Bunpro   ┆ 9       ┆ Now                 │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Total    ┆ 63      ┆ Now                 │
╰──────────┴─────────┴─────────────────────╯
```

```shell
$ srscheck -o json --pretty

{
  "providers": [
    {
      "name": "WaniKani",
      "review_count": 0,
      "next_review": "2025-01-19T22:00:00Z"
    },
    {
      "name": "Bunpro",
      "review_count": 0,
      "next_review": "2025-01-19T16:00:00Z"
    },
    {
      "name": "KameSame",
      "review_count": 0,
      "next_review": null
    },
    {
      "name": "Anki",
      "review_count": 0,
      "next_review": null
    }
  ]
}
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
review_threshold = 100 # (Optional) The threshold for the review count to be considered high

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

### KameSame

Example config:

```toml
[providers."KameSame"] # The name of the provider, this can be any string (but has to be unique)
type = "KameSame" # The type of the provider, this has to
email = "your@email.com"
password = "your-password"
```


## Example uses

TODO: Add some example use cases.

- TODO: Simple bash script that runs `srscheck` and sends a notification if there are reviews due.

## Development

### Roadmap

- [ ] Add build pipeline
- [ ] Better error handling for when a provider is not configured correctly or not responding. At the moment we simply return a review count of 0.
- [ ] More data
- [ ] Add support for Anki (SQL) 

### How to add a new provider

1. Create a new file in the `providers` directory with the name of the provider.
2. Look at the existing providers for reference, and implement the "DataSource" trait for your provider.
3. The fields in the Provider struct are the ones that gets parsed from the Config
4. Add `pub mod my_provider;` to `src/providers/mod.rs`
5. Add your provider to the `Provider` enum in `src/settings.rs`.
6. Add your provider to the match statement in `src/main.rs`
7. Add your provider to the README.md
8. (Optional) Create a PR :)
