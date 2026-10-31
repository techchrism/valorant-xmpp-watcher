# Valorant XMPP Watcher

A tool to connect to Riot XMPP servers to watch for presence updates and chat messages.

Exports XMPP logs in a format compatible with [Valorant XMPP Log Viewer](https://github.com/techchrism/valorant-xmpp-log-viewer)

## Usage

Install rust/cargo and run `cargo run`

||||
|--|--|--|
|Environment Variable|Default|Description|
|`VALORANT_XMPP_WATCHER_SAVED_TOKENS_PATH`|`./saved-tokens.json`|File to read/write saved tokens to. If loading this file fails, new tokens will be generated using the "seed" tokens.|
|`VALORANT_XMPP_WATCHER_SEED_TOKENS_PATH`|`./seed-tokens.json`|File to load initial tokens from.|
|`VALORANT_XMPP_WATCHER_LOGS_PATH`|`./logs`|Directory to put XMPP logs. Created if it doesn't exist.|

`seed-tokens.json` format:
```json
{
    "id_token": "...",
    "refresh_token": "..."
}
```
