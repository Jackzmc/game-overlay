use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use tauri::{Manager, Url, Window, WindowBuilder, WindowUrl};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};
use once_cell::unsync::Lazy;
use crate::{OverlayManager};
use overlay_manager;

pub struct OverlayManagerInstance {
    url: Url,
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>
}


impl OverlayManagerInstance {
    pub fn new(url: Url) -> Self {
        info!("Using url: {}", url);
        Self {
            socket: None,
            url
        }
    }
    
    pub fn reconnect(&mut self) -> tungstenite::Result<()> {
        connect(&self.url).map(|(mut socket, response)| {
            self.socket = Some(socket);
            ()
        })
    }

    // two auth procedures: first time (get url) or token
    pub fn begin_new_account(&mut self) -> Result<String, String> {
        if self.socket.is_none() {
            return Err("Not connected to socket".to_string());
        }
        match self._authorize(None)? {
            overlay_manager::InitConnectionResPayload::PendingClientLogin { url } => Ok(url),
            other => Err(format!("manager returned unexpected response"))
        }
    }
    pub fn wait_for_authorized(&mut self) -> Result<(), String> {
        loop {
            match self.read::<overlay_manager::InitConnectionResPayload>() {
                Ok(Some(overlay_manager::InitConnectionResPayload::ClientAuthorized)) => return Ok(()),
                Ok(None) => {}
                Ok(Some(_)) =>  panic!("manager returned unexpected response"),
                Err(e) => return Err(e)
            }
        }
    }
    pub fn authorize_with_token(&mut self, auth_token: String) -> Result<(), String> {
        if self.socket.is_none() {
            return Err("Not connected to socket".to_string());
        }
        match self._authorize(None)? {
            overlay_manager::InitConnectionResPayload::ClientAuthorized => Ok(()),
            other => Err(format!("manager returned unexpected response"))
        }
    }
    fn _authorize(&mut self, auth_token: Option<String>) -> Result<overlay_manager::InitConnectionResPayload, String> {
        let msg: Message = overlay_manager::InitConnectionReqPayload::Client {
            auth_token
        }.into();
        // let start_auth = Instant::now();
        loop {
            if let Some(response) = self.read::<overlay_manager::InitConnectionResPayload>()? {
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
    pub fn read<T: DeserializeOwned> (&mut self) -> Result<Option<T>, String> {
        if self.socket.is_none() {
            return Err("Not connected to socket".to_string());
        }
        let msg = self.socket.as_mut().unwrap().read().expect("Error reading message");
        Ok(if msg.is_text() {
            Some(serde_json::from_str(&msg.into_text().unwrap()).unwrap())
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
            if let Err(err) = manager.reconnect() {
                error!("Could not connect to manager: {}", err);
                window.emit("manager", overlay_manager::ClientIncomingRequest::ManagerDisconnected).unwrap();
                return;
            }
            info!("Connected to manager successful");
            if let Some(auth_token) = keyring.get_password().ok() {
                manager.authorize_with_token(auth_token).expect("bad auth token")
            } else {
                window.hide().unwrap();
                let url = Url::parse(&manager.begin_new_account().expect("failed to begin new account")).unwrap();
                let auth_window = WindowBuilder::new(&window.app_handle(), "auth_window", WindowUrl::External(url))
                    .title("Login with Steam")
                    .closable(false)
                    .build()
                    .expect("could not create auth window");
                manager.wait_for_authorized();
                auth_window.close().unwrap();
            }
        }

        loop {
            let mut manager = manager.lock().unwrap();
            if let Ok(Some(response)) = manager.read::<overlay_manager::ClientIncomingRequest>() {
                window.emit("manager", response).unwrap();
            }
            drop(manager);
            sleep(Duration::from_secs(5))
        }
    });
}

