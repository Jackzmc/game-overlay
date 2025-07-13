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
use overlay_manager::{ClientIncomingRequest, ServerIncomingRequest, UITemplate};
use crate::manager::{Client, RequestError};
use crate::POOL;

pub struct ServerInstance {
    tx: UnboundedSender<Message>,
    namespace: String,
    id: String,
    clients: HashMap<SteamID, Client>,
    addr: SocketAddr,
    template_ids: Vec<String>,
    elements_fetch_time: Option<SystemTime>
    // db: MySqlPool
}

const ELEMENT_CACHE_TIME: u64 = 90;
impl ServerInstance {
    pub fn next_id() -> Uuid {
        Uuid::new_v4()
    }
    pub fn with_id(addr: SocketAddr, tx: UnboundedSender<Message>, namespace: String, id: String) -> Self {
        Self {
            addr,
            tx,
            namespace,
            id,
            clients: HashMap::new(),
            template_ids: Vec::new(),
            elements_fetch_time: None
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

    pub fn send_request(&self, request: &ServerIncomingRequest) -> Result<(), RequestError> {
        let json = serde_json::to_string(request).map_err(|_| RequestError::RequestNotSerializable)?;
        self.tx.send(Message::Text(json)).map_err(|_| ()).map_err(|_| RequestError::Disconnected)
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
    fn _insert_template(&mut self, template: TemplateEntry) -> Result<UITemplate, String> {
        let json: UITemplate =  serde_json::from_str(&template.data).map_err(|e| e.to_string())?;
        self.template_ids.push(template.id.clone());
        Ok(json)
    }

    pub async fn notify_disconnect(&mut self) {
        for client in self.clients() {
            let mut client = client.lock().await;
            client._set_server(None);
            client.send_request(&ClientIncomingRequest::LeftServer).unwrap();
        }
    }
}

#[derive(FromRow)]
pub struct TemplateEntry {
    pub namespace: String,
    pub id: String,
    pub data: String
}

pub struct ClientNotAuthorized;