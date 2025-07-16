use std::error::Error;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod events;
pub mod requests;
pub mod game;
pub mod ws;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ElementOptions {
    /// If set, after N seconds after element started on client, it will delete itself
    /// Default: None
    pub expires_seconds: Option<u64>,
    /// Should element be visible to client on start?
    /// Default: false
    pub start_visible: Option<bool>
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
    All
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

/// The type of an element's state.
pub type ElementState = Value;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum TargetSelection {
    /// A single player
    SteamID(String),
    SteamIDs(Vec<String>),
    /// All players
    All
}

