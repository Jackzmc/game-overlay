use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use axum::extract::ws::Message;
use jwt::{SignWithKey, VerifyWithKey};
use log::debug;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, Pool};
use steamid_ng::SteamID;
use tokio::sync::mpsc::UnboundedSender;
use overlay_manager::{AuthFailure, ClientOutgoingEvent, ServerOutgoingEvent, ClientIncomingRequest, ServerIncomingRequest};
use crate::client::{ClientInstance};
use crate::{AppError, JWT_SECRET_KEY};
use crate::server::{ServerInstance};
use crate::steam::{SteamClient};

pub type Client = Arc<Mutex<ClientInstance>>;
pub type Server = Arc<Mutex<ServerInstance>>;


#[derive(Debug)]
pub enum RequestError {
    Disconnected,
    RequestNotSerializable,
    InvalidData
}

#[derive(Serialize, Deserialize)]
pub struct ClientTokenClaims {
    #[serde(rename = "sub")]
    pub subject: String,
    #[serde(rename = "iss")]
    pub issuer: String,
    #[serde(rename = "iat")]
    pub issued_at: u64,
    #[serde(rename = "jti")]
    pub jwt_id: Option<String>,
    #[serde(rename = "ip")]
    pub ip_addr: Option<String>
}
#[derive(Serialize, Deserialize)]
pub struct ServerTokenClaims {
    pub namespace: String,
    #[serde(rename = "sub")]
    pub subject: String,
    #[serde(rename = "iss")]
    pub issuer: String,
    #[serde(rename = "iat")]
    pub issued_at: u64,
    #[serde(rename = "ip")]
    pub ip_addr: Option<String>
}
pub struct ManagerInstance {
    clients: HashMap<String, Client>,
    client_steamid_map: HashMap<SteamID, String>, // Maps SteamID to HashMap
    servers: HashMap<String, Server>,
    steam: SteamClient,
}
pub type Manager = Arc<Mutex<ManagerInstance>>;
#[allow(unused)]
// TODO: split into .clients, .servers?
impl ManagerInstance {
    pub fn new(steam: SteamClient) -> Self {
        Self {
            clients: Default::default(),
            client_steamid_map: Default::default(),
            servers: Default::default(),
            steam,
        }
    }

    /// Starts a new client connection
    /// If auth token is not provided, client is temporarily
    pub fn start_client(&mut self, addr: SocketAddr, tx: UnboundedSender<Message>) -> Result<(Client, String), AuthFailure> {
        let id = ClientInstance::next_id();
        let client = ClientInstance::with_id(addr, tx, id.clone());
        let client = Arc::new(Mutex::new(client));
        debug!("start_client: at addr {addr:?}, id={id}");
        self.clients.insert(id.clone(), client.clone());
        Ok((client, id))
    }
    fn _verify_client_token(&self, token: String) -> Result<ClientTokenClaims, String> {
        let claims: ClientTokenClaims = token.verify_with_key(JWT_SECRET_KEY.deref())
            .map_err(|e| e.to_string())?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if claims.issued_at > now {
            return Err("token issued in the future and is invalid".to_string())
        }
        Ok(claims)
    }
    pub async fn authorize_client_token(&mut self, id: &str, auth_token: String) -> Result<(), AuthFailure> {
        let claims: ClientTokenClaims = auth_token.verify_with_key(JWT_SECRET_KEY.deref())
            .map_err(|e| AuthFailure::InvalidAuthToken(Some(e.to_string())))?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if claims.issued_at > now {
            return Err(AuthFailure::InvalidAuthToken(Some("token issued in the future and is invalid".to_string())))
        }
        let steamid = SteamID::from_steam2(&claims.subject).map_err(|e| AuthFailure::InvalidAuthToken(None))?;
        self.mark_client_authorized(id, steamid).await?;
        Ok(())
    }
    /// Marks a client as authorized, storing their steamid & details, and notifies client connection
    pub async fn mark_client_authorized(&mut self, id: &str, steamid: SteamID) -> Result<(), AuthFailure> {
        debug!("authorizing: {}", id);
        let user = self.steam.get_user_details(steamid).await
            .map_err(|e| AuthFailure::General(e.to_string()))?;
        let client = self.clients.get(id).ok_or_else(|| AuthFailure::ObjectNotFound)?;
        let mut client = client.lock().await;
        client._set_steamid(steamid);
        let token = client.generate_auth_token().map_err(|e| AuthFailure::General(e))?;
        client.send_request(&ClientIncomingRequest::Authorized {
            steamid2: steamid.steam2(),
            auth_token: token,
            user
        }).unwrap();
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


    pub async fn try_authorize_server(&mut self, addr: SocketAddr, tx: UnboundedSender<Message>, auth_token: String) -> Result<Server, AuthFailure> {
        let claims: ServerTokenClaims = auth_token.verify_with_key(JWT_SECRET_KEY.deref())
            .map_err(|e| AuthFailure::InvalidAuthToken(Some(e.to_string())))?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if claims.issued_at > now {
            return Err(AuthFailure::InvalidAuthToken(Some("token issued in the future and is invalid".to_string())))
        }
        let server = ServerInstance::with_id(addr, tx, claims.namespace, claims.subject.clone());
        let server = Arc::new(Mutex::new(server));
        self.servers.insert(claims.subject, server.clone());
        Ok(server)
    }

    // pub async fn create_server(&mut self, id: String) -> Result<String, String> {
    //     let id = ServerInstance::next_id().to_string();
    //     let claims = crate::manager::ServerTokenClaims {
    //         namespace: "".to_string(),
    //         subject: id.clone(),
    //         issuer: "manager".to_string(),
    //         issued_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
    //         ip_addr: Some(addr.to_string()),
    //     };
    //     claims.sign_with_key(JWT_SECRET_KEY.deref()).map_err(|e| {
    //         e.to_string()
    //     })
    // }

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
                    let server = server.lock().await;
                    client.send_request(&ClientIncomingRequest::JoinedServer {
                        server_id: server.id().to_string(),
                        server_name: "[not implemented]".to_string(),
                        server_ip: server.addr(),
                    }).unwrap();
                }
            },
            ServerOutgoingEvent::PlayerLeft { steamid} => {
                let steamid = SteamID::from_steam2(steamid).map_err(|_| RequestError::InvalidData)?;
                // If there is no client with that steamid, ignore it, they aren't using overlay
                if let Some(client) = self.find_client_by_steamid(steamid) {
                    let mut client = client.lock().await;
                    client._set_server(None);
                    client.send_request(&ClientIncomingRequest::LeftServer).unwrap();
                }
            },
            ServerOutgoingEvent::RegisterTempUI { elem_id, expires_seconds, element } => {
                let server = server.lock().await;
                for client in server.clients() {
                    let mut client = client.lock().await;
                    client.send_request(&ClientIncomingRequest::RegisterTempUI {
                        elem_id: elem_id.clone(),
                        expires_seconds: expires_seconds.clone(),
                        element: element.clone()
                    }).unwrap();
                }
            },
            ServerOutgoingEvent::UpdateUI { namespace, elem_id, variables, visibility } => {
                let server = server.lock().await;
                for client in server.clients() {
                    let mut client = client.lock().await;
                    client.send_request(&ClientIncomingRequest::UpdateUI {
                        namespace: namespace.clone(),
                        elem_id: elem_id.clone(),
                        variables: variables.clone(),
                        visibility: *visibility
                    }).unwrap();
                }
            },
            // ServerOutgoingEvent::GameState {} => {}
            e => panic!("server event {:?} not supported", e)
        }
        Ok(())
    }
}