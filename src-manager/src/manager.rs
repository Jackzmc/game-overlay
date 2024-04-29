use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use log::debug;
use tokio::sync::Mutex;
use serde::Serialize;
use steamid_ng::SteamID;
use tokio::sync::mpsc::UnboundedSender;
use warp::ws::Message;
use crate::client::{ClientIncomingRequest, ClientInstance, ClientOutgoingEvent};
use crate::server::{ServerInstance, ServerOutgoingEvent};


pub type Client = Arc<Mutex<ClientInstance>>;
pub type Server = Arc<Mutex<ServerInstance>>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "reason")]
pub enum AuthFailure {
    InvalidAuthToken,
    Unknown,
    Timeout,
    ObjectNotFound
}
impl Error for AuthFailure {}

impl fmt::Display for AuthFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthFailure::InvalidAuthToken => write!(f, "auth token is either invalid or unauthorized"),
            AuthFailure::ObjectNotFound => write!(f, "client or server being authorized does not exist"),
            _ => write!(f, "generic authentication failure")
        }
    }
}
#[derive(Debug)]
pub enum RequestError {
    Disconnected,
    RequestNotSerializable,
    InvalidData
}



#[derive(Default)]
pub struct ManagerInstance {
    clients: HashMap<String, Client>,
    client_steamid_map: HashMap<SteamID, String>, // Maps SteamID to HashMap
    servers: HashMap<String, Server>
}
pub type Manager = Arc<tokio::sync::Mutex<ManagerInstance>>;
impl ManagerInstance {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads stored
    pub async fn load(&mut self) {

    }

    pub async fn save(&self) {

    }

    pub fn start_temp_client(&mut self, addr: SocketAddr, tx: UnboundedSender<Message>) -> Result<Client, AuthFailure> {
        let id = ClientInstance::next_id();
        let client = ClientInstance::with_id(addr, tx, id.clone());
        let client = Arc::new(Mutex::new(client));
        debug!("start_temp_client: at addr {addr:?}, id={id}");
        self.clients.insert(id, client.clone());
        Ok(client)
    }

    pub async fn set_client_authorized(&mut self, id: &str, steamid: SteamID) -> Result<(), AuthFailure> {
        let client = self.clients.get(id).ok_or_else(|| AuthFailure::ObjectNotFound)?;
        let mut client = client.lock().await;
        client._set_steamid(steamid);
        self.client_steamid_map.insert(steamid, client.id());
        Ok(())
    }

    pub fn remove_client(&mut self, id: &str) -> bool {
        self.clients.remove(id).is_some()
    }

    pub fn get_client(&self, id: &str) -> Option<Client> {
        self.clients.get(id).cloned()
    }
    pub fn has_client(&self, id: &str) -> bool {
        self.clients.contains_key(id)
    }
    /// Verifies that the client ID exists and the IP matches (ignoring port)
    pub async fn verify_client(&self, id: &str, addr: &SocketAddr) -> bool {
        debug!("verify_client: checking {id} at {addr:?}");
        match self.get_client(id) {
            Some(client) => {
                debug!("verify_client: got client, fetching & validating ip");
                let client = client.lock().await;
                debug!("stored={} incoming={addr}", client.addr());
                client.addr().ip() == addr.ip()
            },
            None => false
        }
    }
    pub fn find_client_by_steamid(&self, steamid: SteamID) -> Option<Client> {
        self.client_steamid_map.get(&steamid).and_then(|id| self.get_client(id))
    }

    pub fn try_authorize_server(&mut self, addr: SocketAddr, tx: UnboundedSender<Message>, auth_token: String) -> Result<Server, AuthFailure> {
        if auth_token.is_empty() {
            return Err(AuthFailure::InvalidAuthToken)
        }
        let id = ServerInstance::next_id();
        let server = ServerInstance::with_id(addr, tx, id.clone());
        let server = Arc::new(Mutex::new(server));
        self.servers.insert(id, server.clone());
        Ok(server)
    }

    pub fn remove_server(&mut self, id: &str) -> bool {
        self.servers.remove(id).is_some()
    }

    pub fn get_server(&self, id: &str) -> Option<Server> {
        self.servers.get(id).cloned()
    }
    pub async fn on_client_event(&mut self, event: &ClientOutgoingEvent, client: Client) -> Result<(), RequestError>  {
        match event {
            e => panic!("client event {:?} not supported", e)
        }
        Ok(())
    }
    pub async fn on_server_event(&mut self, event: &ServerOutgoingEvent, server: Server) -> Result<(), RequestError> {
        match event {
            ServerOutgoingEvent::PlayerJoined { steamid} => {
                let steamid = SteamID::from_steam2(steamid).map_err(|_| RequestError::InvalidData)?;
                // If there is no client with that steamid, ignore it, they aren't using overlay
                if let Some(client) = self.find_client_by_steamid(steamid) {
                    let mut client = client.lock().await;
                    client._set_server(Some(server.clone()));
                    client.send_request(&ClientIncomingRequest::ClientJoined).unwrap();
                }
            },
            ServerOutgoingEvent::PlayerLeft { steamid} => {
                let steamid = SteamID::from_steam2(steamid).map_err(|_| RequestError::InvalidData)?;
                // If there is no client with that steamid, ignore it, they aren't using overlay
                if let Some(client) = self.find_client_by_steamid(steamid) {
                    let mut client = client.lock().await;
                    client._set_server(None);
                    client.send_request(&ClientIncomingRequest::ClientDisconnected).unwrap();
                }
            },
            ServerOutgoingEvent::Disconnecting => {
                let server = server.lock().await;
                for client in server.clients() {
                    let mut client = client.lock().await;
                    client._set_server(None);
                    client.send_request(&ClientIncomingRequest::ClientDisconnected).unwrap();
                }
            },
            // ServerOutgoingEvent::GameState {} => {}
            e => panic!("server event {:?} not supported", e)
        }
        Ok(())
    }
}