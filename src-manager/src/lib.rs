use std::error::Error;
use std::fmt;
use serde::{Deserialize, Serialize};

mod steam;
pub use crate::steam::SteamUser;
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being sent to client (Client <- Manager)
pub enum ClientIncomingRequest {
    ClientJoined,
    ClientDisconnected,
    GameData {}, // TODO: implement
    Authorized { steamid2: String, auth_token: String, user: SteamUser },
    ManagerDisconnected
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being received from the client (Client -> Manager)
pub enum ClientOutgoingEvent {

}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being sent to server (Server <- Manager)
pub enum ServerIncomingRequest {
    Authorized,
    ManagerDisconnected
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being received from server. (Server -> Manager)
pub enum ServerOutgoingEvent {
    PlayerJoined { steamid: String },
    PlayerLeft { steamid: String },
    GameState {}, // TODO: implement
    Disconnecting
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
#[serde(tag = "result")]
pub enum InitConnectionResPayload {
    PendingClientLogin { url: String },
    ClientAuthorized,
    ServerAuthorized,
    InvalidPayload { message: Option<String> },
    AuthError(AuthFailure)
}
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use axum::extract::ws::Message as AxumMessage;

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
    InvalidAuthToken(Option<String>),
    Unknown,
    General(String),
    Timeout,
    ObjectNotFound
}
impl Error for AuthFailure {}
impl fmt::Display for AuthFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthFailure::General(msg) => write!(f, "{}", msg),
            AuthFailure::InvalidAuthToken(msg) => {
                if let Some(msg) = msg {
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