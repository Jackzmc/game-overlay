use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use tauri::{Manager, Url, Window, WindowBuilder, WindowUrl};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};
use once_cell::unsync::Lazy;
use crate::{OverlayManager};
use overlay_manager;

pub struct ClientAuthorized {
    pub steamid2: String,
    pub user: overlay_manager::SteamUser,
    pub auth_token: String,
}


pub struct OverlayManagerInstance {
    url: Url,
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    connect_attempts: u16
}


impl OverlayManagerInstance {
    pub fn new(url: Url) -> Self {
        info!("Using url: {}", url);
        Self {
            socket: None,
            url,
            connect_attempts: 0
        }
    }
    
    pub fn reconnect(&mut self) -> tungstenite::Result<()> {
        connect(&self.url).map(|(mut socket, response)| {
            self.socket = Some(socket);
            self.connect_attempts = 0;
            info!("Connected to manager successfully");
            ()
        }).map_err(|e| {
            error!("Could not connect: {}", e.to_string());
            self.connect_attempts += 1;
            e
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
        let socket = self.socket.as_mut().unwrap();
        socket.send(msg).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Reads from socket
    pub fn read<T: DeserializeOwned> (&mut self) -> Result<Option<T>, tungstenite::Error> {
        if self.socket.is_none() {
            return Err(tungstenite::Error::ConnectionClosed);
        }
        // send disconnect
        let msg = self.socket.as_mut().unwrap().read()?;
        Ok(if msg.is_text() {
            // TODO: make own error type for this cause
            Some(serde_json::from_str(&msg.into_text().unwrap()).expect("could not serialize read()"))
        } else {
            None
        })
    }
}

pub fn start_manager_read_thread(window: Window, manager: OverlayManager) {
    let keyring = keyring::Entry::new("game-overlay", "steamid").unwrap();
    std::thread::spawn(move || {
        {
            debug!("Starting initial connection to manager...", );
            let mut manager = manager.lock().unwrap();
            while let Some(duration) = manager.reconnect_delayed() {
                sleep(duration);
            }
            // TODO: send manager connected
            window.emit("manager", overlay_manager::ClientIncomingRequest::ManagerConnected).unwrap();
            if let Some(auth_token) = keyring.get_password().ok() {
                debug!("using stored keyring");
                trace!("{}", auth_token);
                manager.authorize_with_token(auth_token).expect("bad auth token");
            } else {
                debug!("creating auth window");
                window.hide().unwrap();
                let url = manager.begin_client().expect("failed to begin new account");
                webbrowser::open(&url).expect("could not open browser");
            }
            // let auth_window = WindowBuilder::new(&window.app_handle(), "auth_window", WindowUrl::External(url))
            //     .title("Login with Steam")
            //     .closable(false)
            //     .disable_file_drop_handler()
            //     .content_protected(true)
            //     .build()
            //     .expect("could not create auth window");
            debug!("waiting for auth");
            match manager.wait_for_authorized() {
                Ok(auth_data) => {
                    // TODO: store steamid,user
                    debug!("Authorized! {} {}", auth_data.steamid2, auth_data.user.persona_name);
                    keyring.set_password(&auth_data.auth_token).unwrap();
                    window.emit("manager", overlay_manager::ClientIncomingRequest::Authorized {
                        steamid2: auth_data.steamid2,
                        auth_token: auth_data.auth_token,
                        user: auth_data.user,
                    }).unwrap();
                },
                _ => panic!("unexpected manager response")
            }

            window.show().unwrap();
            // auth_window.close().unwrap();
        }
        loop {
            let mut manager = manager.lock().unwrap();
            match manager.read::<overlay_manager::ClientIncomingRequest>() {
                Ok(response) => {
                    if let Some(response) = response {
                        window.emit("manager", response).unwrap();
                    }
                },
                Err(e) => {
                    eprintln!("read error: {}", e);
                    if let tungstenite::Error::Io(ref io) = e {
                        window.emit("manager", overlay_manager::ClientIncomingRequest::ManagerDisconnected).unwrap();
                        while let Some(duration) = manager.reconnect_delayed() {
                            sleep(duration);
                        }
                        window.emit("manager", overlay_manager::ClientIncomingRequest::ManagerConnected).unwrap();
                    }
                },
            }
            // TODO: if disc
            drop(manager);
            sleep(Duration::from_secs(5))
        }
    });
}


