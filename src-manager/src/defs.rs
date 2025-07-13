use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use axum::extract::ws::Message as AxumMessage;
use serde::{Deserialize, Serialize};
pub use overlay_common::AuthFailure;
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
