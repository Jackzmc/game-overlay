use std::net::SocketAddr;
use std::ops::Deref;
use std::time::{SystemTime, UNIX_EPOCH};
use axum::extract::ws::Message;
use jwt::SignWithKey;
use serde::{Deserialize, Serialize};
use sha2::digest::KeyInit;
use steamid_ng::SteamID;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
use overlay_manager::ClientIncomingRequest;
use crate::JWT_SECRET_KEY;
use crate::manager::{RequestError, Server};

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
        self.tx.send(Message::Text(json)).map_err(|_| ()).map_err(|_| RequestError::Disconnected)
    }

    pub fn generate_auth_token(&self) -> Result<String, String> {
        if self.steamid.is_none() {
            return Err("Client is not authorized/missing steamid".to_string())
        }
        let claims = crate::manager::ClientTokenClaims {
            subject: self.steamid.unwrap().steam2(),
            issuer: "manager".to_string(),
            issued_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            jwt_id: Some(self.addr.to_string()),
            ip_addr: Some(self.addr.to_string()),
        };
        claims.sign_with_key(JWT_SECRET_KEY.deref()).map_err(|e| {
            e.to_string()
        })
    }

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