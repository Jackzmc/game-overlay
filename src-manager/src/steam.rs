use log::debug;
use reqwest::{Error, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
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
        let res = self.client.post(format!("https://steamcommunity.com/openid/login?{query_str}")).send().await
            .map_err(|e| SteamAuthError(e.to_string()))?;
        let text = res.text().await.unwrap();
        // lazy way to check, might want to properly parse it in future?
        if !text.contains("valid:true") {
            return Err(SteamAuthError("Steam auth verification failed".to_string()));
        }

        debug!("steam response:\n{}", text);
        Ok(())
    }

    pub async fn get_user_details(&self, steamid: SteamID) -> Result<SteamUser, Error> {
        let response = self.client.post(format!("https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/?key=${}&format=json&steamids={}",
            self.apikey,
            u64::from(steamid)
        )).send().await?;
        let response = response.error_for_status()?;
        response.json().await
    }

}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SteamUser {
    pub steamid: String,
    pub communityvisibilitystate: usize,
    pub profilestate: usize,
    pub personaname: String,
    pub profileurl: String,
    pub avatar: String,
    pub avatarmedium: String,
    pub avatarfull: String,
    pub avatarhash: String,
    pub lastlogoff: usize,
    pub personastate: usize,
    pub primaryclanid: String,
    pub timecreated: usize,
    pub personastateflags: usize,
    pub loccountrycode: String,
    pub locstatecode: String,
}