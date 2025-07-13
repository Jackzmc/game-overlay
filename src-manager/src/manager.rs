use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use axum::extract::ws::Message;
use futures_util::StreamExt;
use jwt::{SignWithKey, VerifyWithKey};
use log::{debug, trace, warn};
use tokio::sync::{Mutex, MutexGuard};
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, Pool};
use steamid_ng::{SteamID, SteamIDError};
use tokio::sync::mpsc::UnboundedSender;
use overlay_common::events::{ClientEvent, ServerEvent};
use overlay_common::requests::{ClientRequest, ServerRequest};
use overlay_common::TargetPlayer;
use overlay_manager::{AuthFailure};
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
            .map_err(|e| AuthFailure::InvalidAuthToken { message: Some(e.to_string()) })?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if claims.issued_at > now {
            return Err(AuthFailure::InvalidAuthToken { message: Some("token issued in the future and is invalid".to_string())})
        }
        let steamid = SteamID::from_steam2(&claims.subject).map_err(|e| AuthFailure::InvalidAuthToken { message: None })?;
        self.mark_client_authorized(id, steamid).await?;
        Ok(())
    }
    /// Marks a client as authorized, storing their steamid & details, and notifies client connection
    pub async fn mark_client_authorized(&mut self, id: &str, steamid: SteamID) -> Result<(), AuthFailure> {
        debug!("mark_client_authorized: {} [{}]", id, steamid.steam2());
        let user = self.steam.get_user_details(steamid).await
            .map_err(|e| AuthFailure::General { message: e.to_string() })?;
        let client = self.clients.get(id).ok_or_else(|| AuthFailure::ObjectNotFound)?;
        let mut client = client.lock().await;
        client._set_steamid(steamid);
        let token = client.generate_auth_token().map_err(|e| AuthFailure::General { message: e })?;
        client.send_event(&ClientEvent::Authorized {
            steamid2: steamid.steam2(),
            auth_token: token,
            user
        });
        self.client_steamid_map.insert(steamid, client.id());
        Ok(())
    }

    pub fn remove_client(&mut self, id: &str) -> bool {
        self.clients.remove(id).is_some()
    }

    pub fn get_client(&self, id: &str) -> Option<Client> {
        self.clients.get(id).cloned()
    }
    pub fn get_client_from_steamid(&self, steamid: &SteamID) -> Option<Client> {
        if let Some(client_id) = self.client_steamid_map.get(steamid) {
            self.clients.get(client_id).cloned()
        } else {
            None
        }
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
            .map_err(|e| AuthFailure::InvalidAuthToken { message: Some(e.to_string()) })?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if claims.issued_at > now {
            return Err(AuthFailure::InvalidAuthToken { message: Some("token issued in the future and is invalid".to_string()) })
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
    pub async fn on_client_request(&mut self, event: &ClientRequest, client: Client) -> Result<(), RequestError>  {
        match event {
            ClientRequest::Action { command, namespace, input, instance_id } => {
                let client = client.lock().await;
                if let Some(server) = client.connected_server() {
                    let server = server.lock().await;
                    server.send_event(&ServerEvent::Action {
                        steamid2: client.steamid2().unwrap(),
                        command: command.to_string(),
                        namespace: namespace.to_string(),
                        instance_id: instance_id.to_string(),
                        input: input.to_string()
                    })?
                }
            },
            e => panic!("client event {:?} not supported", e)
        }
        Ok(())
    }
    pub async fn on_server_request(&mut self, event: &ServerRequest, server: Server) -> Result<(), RequestError> {
        match event {
            ServerRequest::PlayerJoined { steamid} => {
                let steamid = SteamID::from_steam2(steamid).map_err(|_| RequestError::InvalidData)?;
                // If there is no client with that steamid, ignore it, they aren't using overlay
                if let Some(client) = self.find_client_by_steamid(steamid) {
                    let mut server_inst = server.lock().await;
                    server_inst.add_client(client.clone()).await;
                    let server_info = server_inst.info();
                    drop(server_inst);

                    let mut client = client.lock().await;
                    client._set_server(Some(server.clone()));
                    client.send_event(&ClientEvent::ChangedServer(Some({
                        server_info
                    })))?;
                } else {
                    warn!("PlayerJoined but player was not found: {}", steamid.steam2());
                }
            },
            ServerRequest::PlayerLeft { steamid} => {
                let steamid = SteamID::from_steam2(steamid).map_err(|_| RequestError::InvalidData)?;
                // If there is no client with that steamid, ignore it, they aren't using overlay
                if let Some(client) = self.find_client_by_steamid(steamid) {
                    let mut client = client.lock().await;
                    client._set_server(None);
                    client.send_event(&ClientEvent::ChangedServer(None))?;
                }
                let mut server = server.lock().await;
                server.remove_client(&steamid);
            },
            // ServerOutgoingEvent::RegisterTempUi { selection, elem_id, expires_seconds, element } => {
            //     let server = server.lock().await;
            //     let server_clients = server.client_ids();
            //     drop(server);
            //     for steamid2 in steamids {
            //         if let Ok(steamid) = SteamID::from_steam2(steamid2) {
            //             if server_clients.contains(&steamid) {
            //                 if let Some(client) = self.get_client_from_steamid(&steamid) {
            //                     let mut client = client.lock().await;
            //                     debug!("forwarding register temp ui to client {}", client.id());
            //                     client.send_request(&ClientIncomingRequest::RegisterTempElement {
            //                         elem_id: elem_id.clone(),
            //                         expires_seconds: expires_seconds.clone(),
            //                         element: element.clone()
            //                     }).unwrap();
            //                 }
            //             }
            //         }
            //     }
            //     debug!("forwarded tmp ui to {} clients", steamids.len());
            // },
            ServerRequest::RequestElement { target, elem_id, template_id, state, options } => {
                trace!("{:?}", event);
                self.perform_selection(&server, target, |client| {
                    debug!("forwarding create ui to client {}", client.id());
                    client.send_event(&ClientEvent::RequestElement {
                        elem_id: elem_id.to_string(),
                        template_id: template_id.to_string(),
                        state: state.clone(),
                        options: options.clone()
                    }).unwrap();
                });
                trace!("create element - done");
            },
            ServerRequest::UpdateElement { target, elem_id, state, new_options } => {
                trace!("{:?}", event);
                self.perform_selection(&server, target, |client| {
                    debug!("forwarding update ui to client {}", client.id());
                    client.send_event(&ClientEvent::UpdateElement {
                        elem_id: elem_id.to_string(),
                        state: state.clone(),
                        new_options: new_options.clone()
                    }).unwrap();
                });
                trace!("update element - done");
            },
            // ServerOutgoingEvent::ChangeAudioState {
            //     steamids, source, state, volume, start_time, repeat
            // } => {
            //     panic!("not implemented")
            // },
            // ServerOutgoingEvent::GameState {} => {}
            e => panic!("server event {:?} not supported", e)
        }
        Ok(())
    }

    /// Converts a selection to a list of steamids
    fn selection_to_list(&mut self, all_players: &Vec<SteamID>, selection: &TargetPlayer) -> Result<Vec<SteamID>, SteamIDError> {
        match selection {
            TargetPlayer::Single(steamid) => {
                let steamid = SteamID::from_steam2(steamid)?;
                Ok(vec![steamid])
            }
            TargetPlayer::Many(steamids) => steamids.into_iter().map(|s| SteamID::from_steam2(&s)).collect(),
            TargetPlayer::All => Ok(all_players.clone()),
        }
    }

    async fn perform_selection<F>(&mut self, server: &Server, selection: &TargetPlayer, mut closure: F) -> Result<(), SteamIDError>
        where F: FnMut(MutexGuard<ClientInstance>) -> ()
    {
        let server = server.lock().await;
        let all_players = server.client_ids();
        let mut steamids = self.selection_to_list(&all_players, selection)?;
        drop(server);

        // Filter out clients that aren't connected
        steamids.retain(|id| all_players.contains(id));
        // let client_ids: Result<Vec<SteamID>, SteamIDError> = match selection {
        //     ClientSelection::Steamid(s) => Ok(vec![SteamID::from_steam2(s)?]),
        //     ClientSelection::Steamids(steamids) => steamids.iter().map(|s| SteamID::from_steam2(s)).collect(),
        //     ClientSelection::All => Ok(server_clients.clone())
        // };
        for steamid in &steamids {
            if let Some(client) = self.get_client_from_steamid(&steamid) {
                let mut client = client.lock().await;
                closure(client);
            }
        }
        Ok(())
    }
}