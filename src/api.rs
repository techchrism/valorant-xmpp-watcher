use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: String
}

pub fn reauth(refresh_token: &str, id_token: &str) -> anyhow::Result<AuthTokens> {
    let client = reqwest::blocking::ClientBuilder::new().user_agent("").build()?;

    let response = client.post("https://auth.riotgames.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "client_id=riot-client&grant_type=refresh_token&refresh_token={}&id_token={}&riot_prm=alias%3Achange%20game%3Aplay%20riot%3Ainteract%20purchase%3Acurrency%20chat%3Atext%20chat%3Avoice%20friend%3Aadd%20purchase%3Apersonalize%20content%3Apersonalize%20chat%3Atext%3Asend",
            refresh_token,
            id_token))
        .send()?;
    
    let response_text = response.text()?;
    let response_data: AuthTokens = serde_json::from_str(&response_text).map_err(|err| {
        anyhow::anyhow!("Got error {} from body {}", err, response_text)
    })?;

    Ok(response_data)
}

#[derive(Deserialize)]
struct EntitlementsTokenResponse {
    entitlements_token: String
}

pub fn fetch_entitlements_token(access_token: &str) -> anyhow::Result<String> {
    let client = reqwest::blocking::ClientBuilder::new().user_agent("").build()?;
    
    let response = client.post("https://entitlements.auth.riotgames.com/api/token/v1")
        .bearer_auth(access_token)
        .header("Content-Type", "application/json")
        .send()?;
    
    let response_text = response.text()?;
    let response_data: EntitlementsTokenResponse = serde_json::from_str(&response_text)?;

    Ok(response_data.entitlements_token)
}

pub fn fetch_pas_token(access_token: &str) -> anyhow::Result<String> {
    let client = reqwest::blocking::ClientBuilder::new().user_agent("").build()?;
    
    let response = client.get("https://riot-geo.pas.si.riotgames.com/pas/v1/service/chat")
        .bearer_auth(access_token)
        .send()?;
    
    let response_text = response.text()?;

    Ok(response_text)
}

#[derive(Deserialize)]
pub struct RiotClientConfig {
    #[serde(rename = "chat.affinities")]
    pub chat_affinities: HashMap<String, String>,

    #[serde(rename = "chat.affinity_domains")]
    pub chat_affinity_domains: HashMap<String, String>
}

pub fn fetch_riot_client_config(access_token: &str, entitlements_token: &str) -> anyhow::Result<RiotClientConfig> {
    let client = reqwest::blocking::ClientBuilder::new().user_agent("").build()?;
    
    let response = client.get("https://clientconfig.rpg.riotgames.com/api/v1/config/player?app=Riot%20Client")
        .bearer_auth(access_token)
        .header("X-Riot-Entitlements-JWT", entitlements_token)
        .send()?;
    
    let response_text = response.text()?;
    let response_data: RiotClientConfig = serde_json::from_str(&response_text)?;

    Ok(response_data)
}
