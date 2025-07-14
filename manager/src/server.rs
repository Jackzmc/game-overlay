use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::SystemTime;
use axum::extract::ws::Message;
use handlebars::Template;
use log::debug;
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, MySqlPool, query};
use steamid_ng::SteamID;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
use overlay_common::events::{ClientEvent, ServerEvent};
use overlay_common::game::{ServerInfo, TeamConfig};
use overlay_common::requests::{ClientRequest, ServerRequest};
use overlay_common::ws::InitialServerInfo;
use crate::manager::{Client, RequestError};
use crate::POOL;
use crate::web::websocket::WSMessage;

pub struct ServerInstance {
    tx: UnboundedSender<WSMessage>,
    namespace: String,
    id: String,
    clients: HashMap<SteamID, Client>,
    addr: SocketAddr,

    name: String,
    game_type: u32,
    teams: Vec<TeamConfig>
    // TODO: ugh
    // teams: Vec<TeamConfig>
    // players: Vec<PlayerInfo>
    // db: MySqlPool
}

const ELEMENT_CACHE_TIME: u64 = 90;
impl ServerInstance {
    pub fn next_id() -> Uuid {
        Uuid::new_v4()
    }
    pub fn with_id(addr: SocketAddr, tx: UnboundedSender<WSMessage>, namespace: String, id: String, info: InitialServerInfo) -> Self {
        Self {
            addr,
            tx,
            namespace,
            id,
            clients: HashMap::new(),
            name: info.hostname,
            game_type: info.game_type,
            teams: info.teams,
        }
    }
    pub fn namespace(&self) -> &str { &self.namespace }
    pub fn id(&self) -> &str { &self.id }
    pub fn num_clients(&self) -> usize { self.clients.len() }
    pub fn clients(&self) -> Values<'_, SteamID, Client> {
        self.clients.values().into_iter()
    }
    pub fn client_ids(&self) -> Vec<SteamID> {
        self.clients.keys().map(|s| s.clone()).collect()
    }
    pub fn addr(&self) -> String { self.addr.ip().to_string() }

    pub fn send_event(&self, event: &ServerEvent) -> Result<(), RequestError> {
        let json = serde_json::to_string(event).map_err(|_| RequestError::RequestNotSerializable)?;
        self.tx.send(WSMessage(Message::Text(json))).map_err(|_| ()).map_err(|_| RequestError::Disconnected)
    }

    fn get_client(&self, steamid: SteamID) -> Option<Client> {
        self.clients.get(&steamid).cloned()
    }
    pub async fn add_client(&mut self, client: Client) -> Result<(), ClientNotAuthorized> {
        if let Some(id) = client.lock().await.steamid() {
            debug!("{}: add_client {}", self.id, id.steam2());
            self.clients.insert(id, client.clone());
            Ok(())
        } else {
            Err(ClientNotAuthorized)
        }
    }
    pub fn has_client_steamid(&mut self, id: &SteamID) -> bool {
        self.clients.contains_key(id)
    }
    pub async fn remove_client(&mut self, id: &SteamID) -> bool {
        debug!("{}: remove_client {}", self.id, id.steam2());
        self.clients.remove(id).is_some()
    }

    pub async fn notify_disconnect(&mut self) {
        for client in self.clients() {
            let mut client = client.lock().await;
            client._set_server(None);
            client.send_event(&ClientEvent::LeftServer).unwrap();
        }
    }

    pub fn info(&self) -> ServerInfo {
        ServerInfo {
            id: self.id.clone(),
            // TODO: impl self.name
            name: format!("{} {}", self.id.clone().chars().take(4).collect::<String>(), self.addr),
            ip_addr: self.addr,
            // TODO: impl
            game_type: 0,
            // TODO: impl
            players: vec![],
            // TODO: impl
            teams: vec![],
        }
    }
}

pub struct ClientNotAuthorized;