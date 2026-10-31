use std::{env, io::{BufWriter, Write}, path::Path, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

use anyhow::Context;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};

use crate::{api::{fetch_entitlements_token, fetch_pas_token, fetch_riot_client_config, reauth}, xmpp::xmpp_connect};

mod api;
mod xmpp;

const FAST_FAIL_DURATION: Duration = Duration::from_mins(5);
const FAST_FAIL_DELAY: Duration = Duration::from_mins(5);

fn get_jwt_data(jwt: &str) -> anyhow::Result<Vec<u8>> {
    let mut jwt_parts = jwt.split('.');
    let encoded_data = jwt_parts.nth(1).context("data component of JWT was missing")?;
    let data = URL_SAFE_NO_PAD.decode(encoded_data)?;
    Ok(data)
}

#[derive(Deserialize)]
struct JwtWithExpiration {
    exp: u64
}

fn is_jwt_expired(jwt: &str) -> anyhow::Result<bool> {
    let data: JwtWithExpiration = serde_json::from_slice(&get_jwt_data(jwt)?)?;

    let expiration = UNIX_EPOCH + Duration::from_secs(data.exp);
    let now = SystemTime::now();

    return Ok(now > expiration);
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SavedTokens {
    access_token: String,
    id_token: String,
    refresh_token: String,
    entitlements_token: String,
    pas_token: String
}

#[derive(Debug, Deserialize)]
pub struct SeedTokens {
    id_token: String,
    refresh_token: String
}

#[derive(Debug, Deserialize)]
pub struct PasTokenData {
    affinity: String
}

fn connect() -> anyhow::Result<()> {
    let saved_tokens_path = env::var("VALORANT_XMPP_WATCHER_SAVED_TOKENS_PATH").unwrap_or_else(|_| "./saved-tokens.json".to_string());
    let saved_tokens_result: Result<SavedTokens, anyhow::Error> = (|| {
        let file_contents = std::fs::read(&saved_tokens_path)?;
        let saved_tokens = serde_json::from_slice::<SavedTokens>(&file_contents)?;
        Ok(saved_tokens)
    })();

    let mut saved_tokens = match saved_tokens_result {
        Ok(v) => v,
        Err(e) => {
            println!("Error loading saved tokens {} falling back to seed tokens...", e);

            let seed_tokens_path = env::var("VALORANT_XMPP_WATCHER_SEED_TOKENS_PATH").unwrap_or_else(|_| "./seed-tokens.json".to_string());
            let seed_tokens: SeedTokens = serde_json::from_slice(&std::fs::read(seed_tokens_path)?)?;

            SavedTokens {
                access_token: "".into(),
                id_token: seed_tokens.id_token,
                refresh_token: seed_tokens.refresh_token,
                entitlements_token: "".into(),
                pas_token: "".into()
            }
        }
    };

    let mut any_tokens_changed = false;

    if saved_tokens.access_token.is_empty() || saved_tokens.entitlements_token.is_empty() || is_jwt_expired(&saved_tokens.access_token)? {
        println!("Fetching new access token...");
        let new_tokens = reauth(&saved_tokens.refresh_token, &saved_tokens.id_token)?;
        saved_tokens.id_token = new_tokens.id_token;
        saved_tokens.access_token = new_tokens.access_token;
        saved_tokens.refresh_token = new_tokens.refresh_token;

        println!("Fetching new entitlements token...");
        saved_tokens.entitlements_token = fetch_entitlements_token(&saved_tokens.access_token)?;

        any_tokens_changed = true;
    }

    if saved_tokens.pas_token.is_empty() || is_jwt_expired(&saved_tokens.pas_token)? {
        println!("Fetching new pas token...");
        saved_tokens.pas_token = fetch_pas_token(&saved_tokens.access_token)?;
        any_tokens_changed = true;
    }

    if any_tokens_changed {
        println!("Writing saved tokens...");
        std::fs::write(&saved_tokens_path, serde_json::to_string_pretty(&saved_tokens)?)?;
    }

    let pas_token_data: PasTokenData = serde_json::from_slice(&get_jwt_data(&saved_tokens.pas_token)?)?;
    let client_config = fetch_riot_client_config(&saved_tokens.access_token, &saved_tokens.entitlements_token)?;

    let affinity_host = client_config.chat_affinities.get(&pas_token_data.affinity).context("missing affinity host")?;
    let affinity_domain = client_config.chat_affinity_domains.get(&pas_token_data.affinity).context("missing affinity domain")?;

    let logs_dir_value = env::var("VALORANT_XMPP_WATCHER_LOGS_PATH").unwrap_or_else(|_| "./logs".to_string());
    let logs_dir = Path::new(&logs_dir_value);
    std::fs::create_dir_all(logs_dir)?;
    let log_path = logs_dir.join(format!("{}.txt", SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()));
    let output_file = std::fs::File::create_new(log_path)?;
    let mut output_stream = BufWriter::new(output_file);

    output_stream.write_all(b"{\"type\":\"valorant-xmpp-logger\",\"version\":\"1.1.0\"}\n")?;

    xmpp_connect(
        affinity_host,
        5223,
        &mut output_stream,
        affinity_domain,
        &saved_tokens.access_token,
        &saved_tokens.pas_token,
        &saved_tokens.entitlements_token
    )?;

    Ok(())
}

fn main() {
    let mut consecutive_fast_fails = 0;

    loop {
        let started_instant = Instant::now();
        let connection_result = connect();
        let connection_duration = started_instant.elapsed();
        match connection_result {
            Ok(()) => println!("XMPP connection ended with non-error?"),
            Err(e) => println!("XMPP connection ended with error: {}", e)
        };

        if connection_duration < FAST_FAIL_DURATION {
            consecutive_fast_fails += 1;
            let wait_duration = consecutive_fast_fails.min(6) * FAST_FAIL_DELAY;
            println!("Failed quickly {} time(s), waiting {} minutes...", consecutive_fast_fails, wait_duration.as_secs() / 60);
            thread::sleep(wait_duration);
        } else {
            consecutive_fast_fails = 0;
        }
        println!("Reconnecting...");
    }
}
