use std::env;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};
use overlay_common::events::ClientEvent;
use overlay_common::requests::ClientRequest;
use overlay_common::SteamUser;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use reqwest::Url;
use strum_macros::Display;
use tokio::sync::broadcast;
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::{debug, error, info};
use tracing::log::trace;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{client, Error, Message, WebSocket};
use overlay_common::ws::{AuthRequest, WSResponse};
use crate::defs::ServerInfo;

pub struct ClientAuthorized {
    pub steamid2: String,
    pub user: SteamUser,
    pub auth_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents the state of the manager's connection
pub enum ManagerConnStatus {
    /// Manager has been disconnected
    Disconnected { reason: Option<String> },
    /// Manager is connected, either for first time or reconnected.
    /// May or may not be authenticated
    Connected,
    /// Manager is connected, has sent auth details, is waiting for response
    /// ClientIncomingRequest:Authorized sent if authorized
    WaitingForAuth,
}

pub struct WebsocketClient {
    url: Url,
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    connect_attempts: u16,
    authorized: bool,

    state: Arc<Mutex<ManagerState>>
}

pub struct ManagerState {
    connection_status: ManagerConnStatus,
    server: Option<ServerInfo>,
}
impl Default for ManagerState {
    fn default() -> Self {
        Self {
            server: None,
            connection_status: ManagerConnStatus::Disconnected { reason: None },
        }
    }
}

/// one reader (UI), one writer (Inner)
pub type OverlayState = Arc<Mutex<SocketClient>>;
/// read thread owns instance of manager
pub type OverlayInner = Arc<Mutex<SocketClient>>;
/*
TWO CHOICES:
 - return procesing to UI, then it knows the state
 - read thread _is_ manager, and takes in OverlayState

 */
pub type SocketClient = Arc<Mutex<WebsocketClient>>;
impl WebsocketClient {
    pub fn new() -> Self {
        let ws_url = Url::parse(&env::var("MANAGER_WS_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:3011/socket".to_string()))
            .expect("bad MANAGER_WS_URL");

        info!("Using url: {}", ws_url);
        let state = Arc::new(Mutex::new(ManagerState::default()));
        Self {
            socket: None,
            state,
            url: ws_url,
            connect_attempts: 0,
            authorized: false,
        }
    }

    pub fn state(&self) -> Arc<Mutex<ManagerState>> {
        self.state.clone()
    }
    
    pub fn reconnect(&mut self) -> Result<(), String> {
        self.authorized = false;
        let addr = SocketAddr::new(IpAddr::from_str(self.url.host_str().unwrap()).unwrap(), self.url.port().unwrap());
        let stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        // tungesteine makes this a pain for the handshake:
        // stream.set_nonblocking(true).unwrap();
        let stream = MaybeTlsStream::Plain(stream);
        
        client(&self.url, stream).map(|(socket, response)| {
            self.socket = Some(socket);
            self.connect_attempts = 0;
            info!("Connected to manager successfully");
            ()
        }).map_err(|e| {
            error!("Could not connect: {}", e.to_string());
            self.connect_attempts += 1;
            e.to_string()
        })
    }

    // Tries to connect, and on failure returns an increasing delay. If successful, returns None
    pub fn reconnect_delayed(&mut self) -> Option<Duration> {
        if self.reconnect().is_err() {
            let ms = (self.connect_attempts^2) as f32/2.0;
            return Some(Duration::from_secs_f32(ms))
        }
        None
    }

    /// Blocks until connected, while internally trying to connect with increasing delay
    pub fn wait_for_connected(&mut self, max_attempts: Option<u16>) -> bool {
        let mut c = 0;
        while let Some(dur) = self.reconnect_delayed() {
            trace!("reconnect failed, sleeping for {:?}", dur);
            c = c + 1;
            if let Some(max) = max_attempts  {
                if c > max { return false }
            }
            sleep(dur);
        }
        true
    }

    // two auth procedures: first time (get url) or token
    pub fn begin_client(&mut self) -> Result<String, String> {
        match self._authorize(None)? {
            WSResponse::PendingLogin { url } => Ok(url),
            other => Err(format!("manager returned unexpected response"))
        }
    }
    pub fn wait_for_authorized(&mut self) -> Result<ClientAuthorized, String> {
        loop {
            match self.read::<ClientEvent>() {
                Ok(ClientEvent::Authorized {steamid2, auth_token, user}) => {
                    self.authorized = true;
                    return Ok(ClientAuthorized {
                        steamid2,
                        user,
                        auth_token
                    })
                },
                Ok(_) => return Err("manager sent unexpected data".to_string()),
                Err(e) => return Err(e.to_string()),
            }
        }
    }
    // TODO: split Authorized out into own struct
    pub fn authorize_with_token(&mut self, auth_token: String) -> Result<(), String> {
        self.send(AuthRequest::Client { auth_token: Some(auth_token) }.into())?;
        Ok(())
    }
    fn _authorize(&mut self, auth_token: Option<String>) -> Result<WSResponse, String> {
        self.send(AuthRequest::Client { auth_token }.into())?;
        // let start_auth = Instant::now();
        loop {
             let pay = self.read::<WSResponse>()
                 .map_err(|e| e.to_string())?;
            return Ok(pay)
        }
        Err("Authorization timed out".to_string())
    }
    
    fn send(&mut self, msg: WSMessage) -> Result<(), String> {
        if self.socket.is_none() {
            return Err("Not connected to socket".to_string());
        }
        trace!("send: pre");
        let socket = self.socket.as_mut().unwrap();
        socket.send(msg.0).map_err(|e| e.to_string())?;
        trace!("send: done");
        Ok(())
    }

    /// Reads from socket
    pub fn read<T: DeserializeOwned> (&mut self) -> Result<T, ReadError> {
        let socket = self.socket.as_mut().ok_or(ReadError::NotConnected)?;
        // send disconnect
        let msg = socket.read().map_err(|e| ReadError::Socket(e))?;
        let text = msg.into_text().map_err(|e| ReadError::Socket(e))?;
        serde_json::from_str::<T>(&text).map_err(|e| ReadError::InvalidMessage(e.to_string()))
    }

    /// Processes request internally, returning
    pub fn process_request(&mut self, req: &ClientEvent) -> ReadResult {
        // match req {
        //     ClientEvent::JoinedServer { .. } => {}
        //     ClientEvent::LeftServer => {}
        //     ClientEvent::GameData { .. } => {}
        //     ClientEvent::Authorized { .. } => {}
        //     ClientEvent::ManagerConnState(state) => {
        //         // self.connection_status = state;
        //         // TODO:
        //     }
        //     ClientEvent::RegisterTempElement { .. } => {}
        //     ClientEvent::CreateElement { .. } => {}
        //     ClientEvent::UpdateElement { .. } => {}
        //     ClientEvent::ChangeAudioState { .. } => {}
        // }
        ReadResult::Continue
    }
    
    pub fn send_action(&mut self, instance_id: String, namespace: String, command: String, input: Option<String>) -> Result<(), String> {
        // Block until authorized
        let instant = Instant::now();
        while !self.authorized || instant.elapsed().as_secs() > 30 {
            sleep(Duration::from_secs(1));
        }
        if !self.authorized {
            return Err("Authentication time out".to_string());
        }
        self.send(ClientRequest::Action {
            command,
            namespace,
            input: input.unwrap_or("".to_string()),
            instance_id
        }.into())
    }
}

/// Work around not being able to impl Into<Message>
struct WSMessage(Message);
impl Into<WSMessage> for AuthRequest {
    fn into(self) -> WSMessage {
        WSMessage(Message::Text(serde_json::to_string(&self).expect("failed to serialize AuthRequest")))
    }
}
impl Into<WSMessage> for ClientRequest {
    fn into(self) -> WSMessage {
        WSMessage(Message::Text(serde_json::to_string(&self).expect("failed to serialize AuthRequest")))
    }
}

#[derive(PartialEq)]
enum ReadResult {
    /// Continue to send to UI
    Continue,
    /// Do not send to UI
    Handled
}

#[derive(Display)]
enum ReadError {
    Socket(tungstenite::Error),
    InvalidMessage(String),
    NotConnected
}

#[derive(Debug, Clone)]
pub enum SocketMessage {
    ClientEvent(ClientEvent),
    Connection(ManagerConnStatus)
}

fn manager_thread_init(manager: SocketClient, tx: Sender<SocketMessage>) {
    debug!("Starting initial connection to manager...", );

    let mut manager = manager.lock().unwrap();
    manager.wait_for_connected(None);
    tx.send(SocketMessage::Connection(ManagerConnStatus::Connected)).unwrap();

    // Get the auth token, either from storage or new
    let keyring = keyring::Entry::new("game-overlay", "steamid").unwrap();
    if let Some(auth_token) = keyring.get_password().ok() {
        debug!("using stored keyring");
        trace!("{}", auth_token);
        manager.authorize_with_token(auth_token).expect("bad auth token");
    } else {
        debug!("creating auth window");
        let url = manager.begin_client().expect("failed to begin new account");
        webbrowser::open(&url).expect("could not open browser");
    }
    tx.send(SocketMessage::Connection(ManagerConnStatus::WaitingForAuth)).unwrap();

    debug!("waiting for auth");
    match manager.wait_for_authorized() {
        Ok(auth_data) => {
            // TODO: store steamid,user
            debug!("Authorized! {} {}", auth_data.steamid2, auth_data.user.persona_name);
            keyring.set_password(&auth_data.auth_token).unwrap();
            tx.send(SocketMessage::ClientEvent(ClientEvent::Authorized {
                steamid2: auth_data.steamid2,
                auth_token: auth_data.auth_token,
                user: auth_data.user,
            })).unwrap();
        },
        _ => panic!("unexpected manager response")
    }
}

static MANAGER_READ_INTERVAL: Duration = Duration::from_secs(5);
const RX_CHANNEL_BUFFER: usize = 4;
pub fn start_ws_read_thread(ws: SocketClient) -> Receiver<SocketMessage> {
    let (tx, rx) = broadcast::channel::<SocketMessage>(RX_CHANNEL_BUFFER);
    std::thread::Builder::new()
        .name("manager-read-thread".to_string())
        .spawn(move || {
            ws_thread(ws.clone(), tx)
        })
        .expect("failed to create read thread");
    rx
}

fn ws_thread(ws: SocketClient, tx: Sender<SocketMessage>) {
    manager_thread_init(ws.clone(), tx.clone());
    loop {
        let mut manager = ws.lock().unwrap();
        match manager.read::<ClientEvent>() {
            Ok(response) => {
                debug!("data: {:?}", response);
                if manager.process_request(&response) == ReadResult::Continue {
                    tx.send(SocketMessage::ClientEvent(response)).unwrap();
                }
            },
            Err(err) => {
                eprintln!("read error: {}", err);
                if let ReadError::Socket(tungstenite::Error::Io(_)) = err {
                    // Losft connection, attempt to reconnect:
                    // tx.send(ClientIncomingRequest::ManagerConnState(ManagerConnState::Disconnected {
                    //     reason: Some("websocket_error".to_string())
                    // })).unwrap();
                    manager.wait_for_connected(None);
                    // tx.send(ClientIncomingRequest::ManagerConnState(ManagerConnState::Connected)).unwrap();
                }
            }
            _ => {}
        }
        drop(manager);
        sleep(MANAGER_READ_INTERVAL)
    }
}

