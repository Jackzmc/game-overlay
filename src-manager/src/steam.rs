use log::debug;
use reqwest::{Error, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use steamid_ng::SteamID;
use crate::SteamAuthError;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OpenIDPayload {
    #[serde(rename = "openid.mode")]
    pub mode: String,
    #[serde(rename = "openid.claimed_id")]
    pub claimed_id: String,
    #[serde(rename = "openid.identity")]
    pub identity: String,
    #[serde(rename = "openid.return_to")]
    pub return_to: String,
    #[serde(rename = "openid.response_nonce")]
    pub response_nonce: String,
    #[serde(rename = "openid.assoc_handle")]
    pub assoc_handle: String,
    #[serde(rename = "openid.signed")]
    pub signed: String,
    #[serde(rename = "openid.sig")]
    pub sig: String,
    #[serde(rename = "openid.ns")]
    pub ns: String,
    #[serde(rename = "openid.op_endpoint")]
    pub op_endpoint: String,
    #[serde(rename = "openid.error")]
    pub error: Option<String>
}
#[derive(Clone)]
pub struct SteamClient {
    client: reqwest::Client,
    apikey: String
}
impl SteamClient {
    pub fn new(client: reqwest::Client, apikey: String) -> Self {
        Self {
            client,
            apikey
        }
    }

    pub async fn verify_openid(&self, query: &mut OpenIDPayload) -> Result<(),SteamAuthError> {
        if let Some(error) = query.error.as_ref() {
            return Err(SteamAuthError(error.to_string()));
        }
        query.mode = "check_authentication".to_string();
        let query_str = serde_qs::to_string(&query).unwrap();
        debug!("https://steamcommunity.com/openid/login?{}", &query_str);
        let res = self.client.post(format!("https://steamcommunity.com/openid/login?{query_str}"))
            .header("content-length", 0)
            .send().await
            .map_err(|e| SteamAuthError(e.to_string()))?
            .error_for_status().map_err(|e| SteamAuthError(e.to_string()))?;
        let text = res.text().await.unwrap();
        // lazy way to check, might want to properly parse it in future?
        debug!("openid response: {}", text);
        if !text.contains("valid:true") && std::env::var("STEAM_DONT_VALIDATE").is_err(){
            return Err(SteamAuthError("Steam auth verification failed".to_string()));
        }
        Ok(())
    }

    pub async fn get_user_details(&self, steamid: SteamID) -> Result<SteamUser, Error> {
        let response = self.client.get(format!("https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/?key={}&format=json&steamids={}",
            self.apikey,
            u64::from(steamid)
        ))
            .header("content-length", 0)
            .send().await?
            .error_for_status()?
            .json::<SteamResponse<PlayerSummariesResponse>>()
            .await?;
        let mut players = response.response.players;
        Ok(players.pop().unwrap())
    }

}


#[derive(Serialize, Deserialize)]
pub struct SteamResponse<T> {
    response: T,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerSummariesResponse {
    players: Vec<SteamUser>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SteamUser {
    #[serde(rename = "avatar")]
    avatar: String,
    #[serde(rename = "avatarfull")]
    avatar_full: String,
    #[serde(rename = "avatarhash")]
    avatar_hash: String,
    #[serde(rename = "avatarmedium")]
    avatar_medium: String,
    #[serde(rename = "communityvisibilitystate")]
    community_visibility_state: i64,
    #[serde(rename = "lastlogoff")]
    last_log_off: i64,
    #[serde(rename = "loccountrycode")]
    loc_country_code: String,
    #[serde(rename = "locstatecode")]
    loc_state_code: String,
    #[serde(rename = "personaname")]
    persona_name: String,
    #[serde(rename = "personastate")]
    persona_state: i64,
    #[serde(rename = "personastateflags")]
    persona_state_flags: i64,
    #[serde(rename = "primaryclanid")]
    primary_clan_id: String,
    #[serde(rename = "profilestate")]
    profile_state: i64,
    #[serde(rename = "profileurl")]
    profile_url: String,
    #[serde(rename = "steamid")]
    steamid: String,
    #[serde(rename = "timecreated")]
    time_created: i64,
}