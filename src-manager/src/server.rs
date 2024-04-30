use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::net::SocketAddr;
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use steamid_ng::SteamID;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
use crate::client::ClientIncomingRequest;
use crate::manager::{Client, RequestError};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
/// Messages that are being sent to server (Server <- Manager)
pub enum ServerIncomingRequest {

}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
/// Messages that are being received from server. (Server -> Manager)
pub enum ServerOutgoingEvent {
    PlayerJoined { steamid: String },
    PlayerLeft { steamid: String },
    GameState {}, // TODO: implement
    Disconnecting
}
pub struct ServerInstance {
    tx: UnboundedSender<Message>,
    id: Uuid,
    clients: HashMap<SteamID, Client>,
    addr: SocketAddr
}
impl ServerInstance {
    pub fn next_id() -> String {
        Uuid::new_v4().to_string()
    }
    pub fn with_id(addr: SocketAddr, tx: UnboundedSender<Message>, id: String) -> Self {
        Self {
            addr,
            tx,
            id: id.parse().unwrap(),
            clients: HashMap::new()
        }
    }

    pub fn id(&self) -> String { self.id.to_string() }
    pub fn num_clients(&self) -> usize { self.clients.len() }
    pub fn clients(&self) -> Values<'_, SteamID, Client> {
        self.clients.values().into_iter()
    }

    pub fn send_request(&self, request: &ServerIncomingRequest) -> Result<(), RequestError> {
        let json = serde_json::to_string(request).map_err(|_| RequestError::RequestNotSerializable)?;
        self.tx.send(Message::Text(json)).map_err(|_| ()).map_err(|_| RequestError::Disconnected)
    }

    fn get_client(&self, steamid: SteamID) -> Option<Client> {
        self.clients.get(&steamid).cloned()
    }
    pub async fn add_client(&mut self, client: Client) -> Result<(), ClientNotAuthorized> {
        if let Some(id) = client.lock().await.steamid() {
            self.clients.insert(id, client.clone());
            Ok(())
        } else {
            Err(ClientNotAuthorized)
        }
    }
}

pub struct ClientNotAuthorized;