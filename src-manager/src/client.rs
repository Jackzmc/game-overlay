use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use steamid_ng::SteamID;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
use warp::ws::Message;
use crate::manager::{RequestError, Server};

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Messages that are being sent to client (Client <- Manager)
pub enum ClientIncomingRequest {
    ClientJoined,
    ClientDisconnected,
    GameData {} // TODO: implement
}
#[derive(Serialize, Deserialize, Debug, Clone)]
/// Messages that are being received from the client (Client -> Manager)
pub enum ClientOutgoingEvent {

}
pub struct ClientInstance {
    id: Uuid,
    steamid: Option<SteamID>,
    server: Option<Server>,
    addr: SocketAddr,
    tx: UnboundedSender<Message>,
}
impl ClientInstance {
    pub fn next_id() -> String {
        Uuid::new_v4().to_string()
    }
    pub fn with_id(addr: SocketAddr, tx: UnboundedSender<Message>, id: String) -> Self {
        Self {
            id: id.parse().unwrap(),
            steamid: None,
            server: None,
            addr,
            tx,
        }
    }

    pub fn id(&self) -> String { self.id.to_string() }
    pub fn steamid(&self) -> Option<SteamID> { self.steamid.as_ref().map(|s| s.clone()) }
    pub fn steamid2(&self) -> Option<String> { self.steamid.map(|s| s.steam2()) }
    pub fn addr(&self) -> &SocketAddr { &self.addr }
    pub fn connected_server(&self) -> Option<Server> {
        self.server.as_ref().map(|s| s.clone())
    }
    /// Does the client have a steamid authorized or not (temporary, in process of authorizing)
    pub fn is_temp_client(&self) -> bool {
        self.steamid.is_none()
    }
    /// Is the client authorized with a steamid (not temporary)
    pub fn is_authorized(&self) -> bool {
        self.steamid.is_some()
    }

    pub fn send_request(&self, request: &ClientIncomingRequest) -> Result<(), RequestError> {
        let json = serde_json::to_string(request).map_err(|_| RequestError::RequestNotSerializable)?;
        self.tx.send(Message::text(json)).map_err(|_| ()).map_err(|_| RequestError::Disconnected)
    }

    // pub fn connect_to(&mut self, server: &mut Server) {
    //     // server.add_client(self.)
    // }

    pub fn _set_steamid(&mut self, steamid: SteamID) {
        self.steamid = Some(steamid);
    }
    pub fn _set_server(&mut self, server: Option<Server>) {
        self.server = server;
    }

    pub fn is_connected(&self) -> bool {
        self.server.is_some()
    }
}