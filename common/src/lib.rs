use std::error::Error;
use std::fmt;
use std::net::IpAddr;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod events;
pub mod requests;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateElementRegister {
    pub namespace: String,
    pub instance_id: String,
    pub template_id: String,
    pub options: Option<ElementOptions>
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ElementOptions {
    /// If set, after N seconds after element started on client, it will delete itself
    /// Default: None
    pub expires_seconds: Option<u64>,
    /// Should element be visible to client on start?
    /// Default: false
    pub start_visible: Option<bool>
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateElementRegister {
    pub namespace: String,
    pub instance_id: String,
    /// Update visibility, if not set, value remains unchanged
    pub visible: Option<bool>,
    /// Variables. Replaces all variables on client
    pub variables: Value
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum ClientSelection {
    /// Apply to a single steamid
    Steamid { steamid: String },
    /// Apply to multiple steamids
    Steamids { steamids: Vec<String> },
    /// apply to all clients connected to server
    All {  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct UITemplate {
    #[serde(rename = "type")]
    pub ui_type: String,
    #[serde(flatten)]
    pub other_fields: Value
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum InitConnectionReqPayload {
    Client { auth_token: Option<String> },
    Server { auth_token: String }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "reason")]
pub enum AuthFailure {
    InvalidAuthToken { message: Option<String> },
    Unknown,
    General { message: String },
    Timeout,
    ObjectNotFound
}
impl Error for AuthFailure {}
impl fmt::Display for AuthFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthFailure::General { message} => write!(f, "{}", message),
            AuthFailure::InvalidAuthToken { message} => {
                if let Some(msg) = message {
                    write!(f, "{}", msg)
                } else {
                    write!(f, "auth token is either invalid or unauthorized")
                }
            },
            AuthFailure::ObjectNotFound => write!(f, "client or server being authorized does not exist"),
            _ => write!(f, "generic authentication failure")
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SteamUser {
    #[serde(rename = "avatar")]
    pub avatar: String,
    #[serde(rename = "avatarfull")]
    pub avatar_full: String,
    #[serde(rename = "avatarhash")]
    pub avatar_hash: String,
    #[serde(rename = "avatarmedium")]
    pub avatar_medium: String,
    #[serde(rename = "communityvisibilitystate")]
    pub community_visibility_state: i64,
    #[serde(rename = "lastlogoff")]
    pub last_log_off: i64,
    #[serde(rename = "loccountrycode")]
    pub loc_country_code: String,
    #[serde(rename = "locstatecode")]
    pub loc_state_code: String,
    #[serde(rename = "personaname")]
    pub persona_name: String,
    #[serde(rename = "personastate")]
    pub persona_state: i64,
    #[serde(rename = "personastateflags")]
    pub persona_state_flags: i64,
    #[serde(rename = "primaryclanid")]
    pub primary_clan_id: String,
    #[serde(rename = "profilestate")]
    pub profile_state: i64,
    #[serde(rename = "profileurl")]
    pub profile_url: String,
    #[serde(rename = "steamid")]
    pub steamid: String,
    #[serde(rename = "timecreated")]
    pub time_created: i64,
}