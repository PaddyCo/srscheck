# srscheck

CLI tool for quickly getting the status of multiple SRS (Spaced Repetition System) systems.

## Note

- This is a work in progress, and mostly built for personal use. (But should be usable)

- Only tested on Linux at the moment, but should be painless to get running on other platforms as I'm not using any platform-specific code (as far as I know).

- API responses are cached to disk per-provider (in `cache_path`), so running the tool repeatedly within the cache window won't hit the provider's API again. Each provider has a default cache expiration (see [Supported providers](#supported-providers)), which can be overridden per-provider with `cache_expiry` (in seconds).

- If a provider fails to fetch data, it will show `status: Error` in the output (with the error message in JSON mode) and be excluded from the total review count. Other providers are unaffected.


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

╭──────────┬────────┬─────────┬─────────────────────┬───────────────────────────╮
│ System   ┆ Status ┆ Reviews ┆ Next Review         ┆ URL                       │
╞══════════╪════════╪═════════╪═════════════════════╪═══════════════════════════╡
│ WaniKani ┆ OK     ┆ 0       ┆ 2025-01-19 15:00:00 ┆ https://www.wanikani.com/ │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Anki     ┆ OK     ┆ 12      ┆ Now                 ┆                           │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ KameSame ┆ OK     ┆ 42      ┆ Now                 ┆ https://www.kamesame.com/ │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Bunpro   ┆ OK     ┆ 9       ┆ Now                 ┆ https://bunpro.jp/        │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ MySrs    ┆ Error  ┆ N/A     ┆ N/A                 ┆                           │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Total    ┆        ┆ 63      ┆ Now                 ┆                           │
╰──────────┴────────┴─────────┴─────────────────────┴───────────────────────────╯
```

```shell
$ srscheck -o json --pretty

{
  "providers": [
    {
      "status": "OK",
      "name": "WaniKani",
      "review_count": 0,
      "next_review": "2025-01-19T22:00:00Z",
      "action_url": "https://www.wanikani.com/"
    },
    {
      "status": "OK",
      "name": "Bunpro",
      "review_count": 0,
      "next_review": "2025-01-19T16:00:00Z",
      "action_url": "https://bunpro.jp/"
    },
    {
      "status": "OK",
      "name": "KameSame",
      "review_count": 0,
      "next_review": null,
      "action_url": "https://www.kamesame.com/"
    },
    {
      "status": "OK",
      "name": "Anki",
      "review_count": 0,
      "next_review": null,
      "action_url": null
    },
    {
      "status": "Error",
      "name": "MySrs",
      "error": "error sending request for url: dns error: failed to lookup address information: Name or service not known"
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
action_url = "http://localhost:8765" # OPTIONAL: URL opened to do reviews. No default for Anki since it's self-hosted.
cache_expiry = 10 # OPTIONAL: How long (in seconds) to cache API results for. Defaults to 10.
```

### WaniKani

Example config:
```toml
[providers."WaniKani"] # The name of the provider, this can be any string (but has to be unique)
type = "WaniKani" # The type of the provider, this has to be "WaniKani"
api_key = "your-key" # The API key you get from the WaniKani settings. Read-only access is enough.
action_url = "https://www.wanikani.com/" # OPTIONAL: URL opened to do reviews. Defaults to "https://www.wanikani.com/".
cache_expiry = 300 # OPTIONAL: How long (in seconds) to cache API results for. Defaults to 300 (5 minutes).
```

### Bunpro

Example config:
```toml
[providers."Bunpro"] # The name of the provider, this can be any string (but has to be unique) 
type = "Bunpro" # The type of the provider, this has to be "Bunpro"
api_key = "your-key" # The API key you get from the Bunpro settings.
action_url = "https://bunpro.jp/" # OPTIONAL: URL opened to do reviews. Defaults to "https://bunpro.jp/".
cache_expiry = 300 # OPTIONAL: How long (in seconds) to cache API results for. Defaults to 300 (5 minutes).
```

### KameSame

Example config:

```toml
[providers."KameSame"] # The name of the provider, this can be any string (but has to be unique)
type = "KameSame" # The type of the provider, this has to
email = "your@email.com"
password = "your-password"
action_url = "https://www.kamesame.com/" # OPTIONAL: URL opened to do reviews. Defaults to "https://www.kamesame.com/".
cache_expiry = 300 # OPTIONAL: How long (in seconds) to cache API results for. Defaults to 300 (5 minutes).
```

### NativShark

Example config:

```toml
[providers."NativShark"] # The name of the provider, this can be any string (but has to be unique)
type = "NativShark" # The type of the provider, this has to be "NativShark"
email = "your@email.com"
password = "your-password"
action_url = "https://app.nativshark.com/" # OPTIONAL: URL opened to do reviews. Defaults to "https://app.nativshark.com/".
cache_expiry = 300 # OPTIONAL: How long (in seconds) to cache API results for. Defaults to 300 (5 minutes).
```

NativShark's API uses a login token instead of an API key. `srscheck` logs in with your email/password
and caches the resulting token (it's valid for about a month), automatically logging in again once it expires
or is rejected by the API.

### Http (custom provider)

Use this to hook up any SRS that exposes a JSON HTTP API but isn't natively supported.
`review_count_path` and `next_review_path` are [jq](https://jqlang.github.io/jq/manual/) filters
(evaluated with [jaq](https://github.com/01mf02/jaq)) run against the JSON response to pick out the
fields you need.

Given a response like:
```json
{
  "reviews": {
    "pending_reviews": 120,
    "next_review": "2023-09-15T12:00:00Z"
  }
}
```

Example config:
```toml
[providers."MySrs"] # The name of the provider, this can be any string (but has to be unique)
type = "Http" # The type of the provider, this has to be "Http"
url = "https://example.com/api/reviews" # The URL to send the request to
method = "GET" # OPTIONAL: HTTP method to use (GET, POST, PUT, DELETE, etc.). Defaults to "GET".
headers = { Authorization = "Bearer my-token" } # OPTIONAL: headers to include in the request
review_count_path = ".reviews.pending_reviews" # jq filter used to extract the review count
next_review_path = ".reviews.next_review" # OPTIONAL: jq filter used to extract the next review date.
# The matched value can be either an RFC 3339 string (like above) or a Unix timestamp, e.g. ".reviews.next_review_epoch"
action_url = "https://example.com/" # OPTIONAL: URL opened to do reviews. No default since it's a custom provider.
cache_expiry = 60 # OPTIONAL: How long (in seconds) to cache API results for. Defaults to 60 (1 minute).
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
