use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use log::info;
use tauri::{Manager, Url, Window};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, WebSocket};
use crate::{OverlayManager};

pub struct OverlayManagerInstance {
    url: Url,
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>
}

impl OverlayManagerInstance {
    pub fn new(url: Url) -> Self {
        Self {
            socket: None,
            url
        }
    }
    
    pub fn reconnect(&mut self) -> tungstenite::Result<()> {
        connect(&self.url).map(|(mut socket, response)| {
            self.socket = Some(socket);
            println!("connected to {}", self.url);
            ()
        })
    }

    /// Reads from socket
    pub fn read(&mut self) -> Result<Option<ManagerResponse>, String> {
        if self.socket.is_none() {
            return Err("Not connected to socket".to_string());
        }
        let msg = self.socket.as_mut().unwrap().read().expect("Error reading message");
        Ok(if msg.is_text() {
            Some(ManagerResponse::Unknown { message: msg.into_text().unwrap() })
        } else {
            None
        })
    }
}

pub fn start_manager_read_thread(window: Window, manager: OverlayManager) {
    std::thread::spawn(move || {
        {
            let mut manager = manager.lock().unwrap();
            if let Err(err) = manager.reconnect() {
                window.emit("manager", ManagerResponse::ManagerDisconnected { message: Some(err.to_string()) }).unwrap();
                return;
            }
            info!("Manager connection successful")
        }
        loop {
            if let Ok(Some(response)) = manager.lock().unwrap().read() {
                window.emit("manager", response).unwrap();
            }
            sleep(Duration::from_secs(5))
        }
    });
}

#[derive(serde::Serialize, Clone)]
#[serde(tag = "type")]
pub enum ManagerResponse {
    Error { message: String },
    ServerConnected { host: String, ip: usize },
    ServerDisconnected { message: Option<String> },
    ManagerDisconnected { message: Option<String> },
    Unknown { message: String }
}
