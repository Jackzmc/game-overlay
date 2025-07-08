use std::env;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use tungstenite::stream::{MaybeTlsStream, NoDelay};
use tungstenite::{client, connect, Error, HandshakeError, Message, WebSocket};
use tungstenite::handshake::client::Response;
use tungstenite::util::NonBlockingError;
use overlay_manager;
use overlay_manager::{ClientIncomingRequest, ClientOutgoingEvent, ManagerConnState};
use reqwest::Url;
use tokio::sync::broadcast;
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::{debug, error, info};
use tracing::log::trace;

pub struct ClientAuthorized {
    pub steamid2: String,
    pub user: overlay_manager::SteamUser,
    pub auth_token: String,
}


pub struct OverlayManagerInstance {
    url: Url,
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    connect_attempts: u16,
    authorized: bool,

    connection_state: ManagerConnState
}

/// one reader (UI), one writer (Inner)
pub type OverlayState = Arc<Mutex<OverlayManager>>;
/// read thread owns instance of manager
pub type OverlayInner = Arc<Mutex<OverlayManager>>;
/*
TWO CHOICES:
 - return procesing to UI, then it knows the state
 - read thread _is_ manager, and takes in OverlayState

 */
pub type OverlayManager = Arc<Mutex<OverlayManagerInstance>>;
impl OverlayManagerInstance {
    pub fn new() -> Self {
        let ws_url = Url::parse(&env::var("MANAGER_WS_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:3011/socket".to_string()))
            .expect("bad MANAGER_WS_URL");

        info!("Using url: {}", ws_url);
        Self {
            socket: None,
            url: ws_url,
            connect_attempts: 0,
            authorized: false,

            connection_state: ManagerConnState::Disconnected { reason: None }
        }
    }

    pub fn conn_state(&self) -> &ManagerConnState {
        &self.connection_state
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
            overlay_manager::InitConnectionResPayload::PendingClientLogin { url } => Ok(url),
            other => Err(format!("manager returned unexpected response"))
        }
    }
    pub fn wait_for_authorized(&mut self) -> Result<ClientAuthorized, String> {
        loop {
            match self.read::<overlay_manager::ClientIncomingRequest>() {
                Ok(Some(overlay_manager::ClientIncomingRequest::Authorized {steamid2, auth_token, user})) => {
                    self.authorized = true;
                    return Ok(ClientAuthorized {
                        steamid2,
                        user,
                        auth_token
                    })
                },
                Ok(Some(_)) => return Err("manager sent unexpected data".to_string()),
                Err(e) => return Err(e.to_string()),
                Ok(None) => {}
            }
        }
    }
    // TODO: split Authorized out into own struct
    pub fn authorize_with_token(&mut self, auth_token: String) -> Result<(), String> {
        self.send(overlay_manager::InitConnectionReqPayload::Client { auth_token: Some(auth_token) }.into())?;
        Ok(())
    }
    fn _authorize(&mut self, auth_token: Option<String>) -> Result<overlay_manager::InitConnectionResPayload, String> {
        self.send(overlay_manager::InitConnectionReqPayload::Client { auth_token }.into())?;
        // let start_auth = Instant::now();
        loop {
            if let Some(response) = self.read::<overlay_manager::InitConnectionResPayload>().map_err(|e| e.to_string())? {
                return Ok(response)
            }
        }
        Err("Authorization timed out".to_string())
    }
    
    fn send(&mut self, msg: Message) -> Result<(), String> {
        if self.socket.is_none() {
            return Err("Not connected to socket".to_string());
        }
        trace!("send: pre");
        let socket = self.socket.as_mut().unwrap();
        socket.send(msg).map_err(|e| e.to_string())?;
        trace!("send: done");
        Ok(())
    }

    /// Reads from socket
    pub fn read<T: DeserializeOwned> (&mut self) -> Result<Option<T>, tungstenite::Error> {
        if self.socket.is_none() {
            return Err(tungstenite::Error::ConnectionClosed);
        }
        // send disconnect
        let socket = self.socket.as_mut().unwrap();
        match socket.read() {
            Ok(msg) => {
                Ok(msg.into_text().map(|text| {
                    serde_json::from_str(&text).expect("could not serialize read()")
                }).ok())
            },
            Err(e) => {
                // if let Error::AlreadyClosed() = e || Error::ConnectionClosed
                Err(e)
            }
        }
    }

    pub fn process_request(&mut self, req: ClientIncomingRequest) {
        match req {
            ClientIncomingRequest::JoinedServer { .. } => {}
            ClientIncomingRequest::LeftServer => {}
            ClientIncomingRequest::GameData { .. } => {}
            ClientIncomingRequest::Authorized { .. } => {}
            ClientIncomingRequest::ManagerConnState(state) => {
                self.connection_state = state;
            }
            ClientIncomingRequest::RegisterTempElement { .. } => {}
            ClientIncomingRequest::CreateElement { .. } => {}
            ClientIncomingRequest::UpdateElement { .. } => {}
            ClientIncomingRequest::ChangeAudioState { .. } => {}
        }
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
        let str = serde_json::to_string(&ClientOutgoingEvent::Action {
            command,
            namespace,
            input: input.unwrap_or("".to_string()),
            instance_id
        }).unwrap();
        self.send(Message::Text(str))
    }
}

fn manager_thread_init(manager: OverlayManager, tx: Sender<ClientIncomingRequest>) {
    debug!("Starting initial connection to manager...", );

    let mut manager = manager.lock().unwrap();
    manager.wait_for_connected(None);
    tx.send(ClientIncomingRequest::ManagerConnState(ManagerConnState::Connected)).unwrap();

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
    tx.send(ClientIncomingRequest::ManagerConnState(ManagerConnState::WaitingForAuth)).unwrap();

    debug!("waiting for auth");
    match manager.wait_for_authorized() {
        Ok(auth_data) => {
            // TODO: store steamid,user
            debug!("Authorized! {} {}", auth_data.steamid2, auth_data.user.persona_name);
            keyring.set_password(&auth_data.auth_token).unwrap();
            tx.send(ClientIncomingRequest::Authorized {
                steamid2: auth_data.steamid2,
                auth_token: auth_data.auth_token,
                user: auth_data.user,
            }).unwrap();
        },
        _ => panic!("unexpected manager response")
    }
}

static MANAGER_READ_INTERVAL: Duration = Duration::from_secs(5);
const RX_CHANNEL_BUFFER: usize = 4;
pub fn start_manager_read_thread(manager: OverlayManager) -> Receiver<ClientIncomingRequest> {
    let (tx, rx) = broadcast::channel::<ClientIncomingRequest>(RX_CHANNEL_BUFFER);
    std::thread::Builder::new()
        .name("manager-read-thread".to_string())
        .spawn(move || {
            manager_thread_init(manager.clone(), tx.clone());
            loop {
                let mut manager = manager.lock().unwrap();
                match manager.read::<ClientIncomingRequest>() {
                    Ok(response) => {
                        if let Some(response) = response {
                            debug!("data: {:?}", response);
                            manager.process_request(response);
                            // tx.send(response).unwrap();
                        }
                    },
                    Err(e) => {
                        eprintln!("read error: {}", e);
                        if let tungstenite::Error::Io(_) = e {
                            // Lost connection, attempt to reconnect:
                            // tx.send(ClientIncomingRequest::ManagerConnState(ManagerConnState::Disconnected {
                            //     reason: Some("websocket_error".to_string())
                            // })).unwrap();
                            manager.wait_for_connected(None);
                            // tx.send(ClientIncomingRequest::ManagerConnState(ManagerConnState::Connected)).unwrap();
                        }
                    },
                }
                drop(manager);
                sleep(MANAGER_READ_INTERVAL)
            }
        })
        .expect("failed to create read thread");
    rx
}


