use std::error::Error;
use std::fmt;
use std::net::IpAddr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being sent to client (Client <- Manager)
pub enum ClientIncomingRequest {
    JoinedServer { server_id: String, server_name: String, server_ip: String },
    LeftServer,
    GameData {}, // TODO: implement
    Authorized { steamids: Vec<String>, auth_token: String, user: SteamUser },
    // Manual activation for UI side:
    ManagerDisconnected,
    ManagerConnected,
    RegisterTempUi { elem_id: String, expires_seconds: Option<u64>, element: UIElement },
    // Clients will fetch UI if received (with visibility=true)
    UpdateUi { namespace: Option<String>, elem_id: Option<String>, visible: bool, variables: Value },
    ChangeAudioState { source: String, state: u8, volume: Option<f32>, start_time: Option<f32>, repeat: Option<bool> }


}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being received from the client (Client -> Manager)
pub enum ClientOutgoingEvent {
    Action { action: String, namespace: String, elem_id: String }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being sent to server (Server <- Manager)
pub enum ServerIncomingRequest {
    Authorized,
    ManagerDisconnected,
    Action { steamid: String, action: String, namespace: String, elem_id: String }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being received from server. (Server -> Manager)
pub enum ServerOutgoingEvent {
    PlayerJoined { steamid: String },
    PlayerLeft { steamid: String },
    GameState {}, // TODO: implement
    RegisterTempUi { elem_id: String, expires_seconds: Option<u64>, element: UIElement },
    UpdateUi { namespace: Option<String>, elem_id: Option<String>, variables: Value, visible: bool },
    ChangeAudioState { steamids: Vec<String>, source: String, state: u8, volume: Option<f32>, start_time: Option<f32>, repeat: Option<bool> }

}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct UIElement {
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "error")]
pub enum InitConnectionResPayload {
    PendingClientLogin { url: String },
    ClientAuthorized,
    ServerAuthorized,
    InvalidPayload { message: Option<String> },
    AuthError(AuthFailure)
}
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use axum::extract::ws::Message as AxumMessage;
use serde_json::Value;

impl Into<AxumMessage> for InitConnectionResPayload {
    fn into(self) -> AxumMessage {
        AxumMessage::Text(serde_json::to_string(&self).unwrap())
    }
}
impl Into<AxumMessage> for InitConnectionReqPayload {
    fn into(self) -> AxumMessage {
        AxumMessage::Text(serde_json::to_string(&self).unwrap())
    }
}
impl Into<TungsteniteMessage> for InitConnectionResPayload {
    fn into(self) -> TungsteniteMessage {
        TungsteniteMessage::text(serde_json::to_string(&self).unwrap())
    }
}
impl Into<TungsteniteMessage> for InitConnectionReqPayload {
    fn into(self) -> TungsteniteMessage {
        TungsteniteMessage::text(serde_json::to_string(&self).unwrap())
    }
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