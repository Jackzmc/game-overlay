use std::net::TcpStream;
use tauri::Url;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, WebSocket};
use crate::MANAGER_WS_URL;

pub struct Manager {
    url: Url,
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>
}

impl Manager {
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

#[derive(serde::Serialize, Clone)]
#[serde(tag = "type")]
pub enum ManagerResponse {
    Error { message: String },
    ServerConnected { host: String, ip: usize },
    ServerDisconnected { message: Option<String> },
    ManagerDisconnected { message: Option<String> },
    Unknown { message: String }
}
