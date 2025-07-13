use axum::extract::ws::{Message, WebSocket};
use std::net::SocketAddr;
use log::{debug, error, trace, warn};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use overlay_common::{requests};
use overlay_common::events::ServerEvent;
use std::ops::Deref;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use overlay_common::requests::{ServerRequest};
use overlay_common::ws::{AuthFailure, AuthRequest, WSResponse};
use crate::manager::{Client, Manager, Server};
use crate::{CLIENT_AUTH_TIMEOUT, PUBLIC_URL};
use crate::web::AppError;



/// Sets up a task and waits for the first message to be received
pub async fn setup_conn(mut ws: WebSocket, addr: SocketAddr, manager: Manager) {
    debug!("setup_conn addr={:?}", addr);
    // let (mut conn_tx, mut conn_rx) = ws.split();
    // Wait for the first initial connection
    tokio::task::spawn(async move {
        if let Some(Ok(message)) = ws.next().await {
            trace!("incoming msg");
            match serde_json::from_str::<AuthRequest>(&message.into_text().unwrap()) {
                Ok(json) => {
                    login_connection(ws, manager, json, addr).await;
                },
                Err(err) => {
                    warn!("invalid payload: {}", err);
                    send_err(&mut ws, AuthFailure::InternalError { message: Some(err.to_string()) }).await;
                    ws.close().await.ok();
                }
            }
        }
    });

}

/// Called with the auth payload, handles authenticating
async fn login_connection(mut ws: WebSocket, manager: Manager, req: AuthRequest, addr: SocketAddr) {
    let (tx, rx) = mpsc::unbounded_channel::<WSMessage>();
    let mut rx = UnboundedReceiverStream::new(rx);
    let mut mngr = manager.lock().await;
    // TODO: add timeout, to remove temp clients
    match req {
        AuthRequest::Client { auth_token} => {
            debug!("login_connection - creating client");
            match mngr.start_client(addr, tx.clone()) {
                Ok((client, id)) => {
                    if let Some(token) = auth_token {
                        if let Err(err) = mngr.authorize_client_token(&id, token).await {
                            // Cleanup ID
                            send_err(&mut ws, AuthFailure::InternalError{ message: Some(err.to_string()) }).await;
                            mngr.remove_client(&id);
                            return;
                        }
                    } else {
                        send(&mut ws, WSResponse::PendingLogin { url: format!("{}/auth/login?id={id}", PUBLIC_URL.deref()) }).await;
                    }
                    drop(mngr);
                    init_client_connection(ws, (tx, rx), manager, client).await;
                },
                Err(err) => {
                    send_err(&mut ws,err).await;
                }
            }
        }
        AuthRequest::Server { auth_token } => {
            debug!("login_connection - authorizing server");
            match mngr.try_authorize_server(addr, tx.clone(), auth_token).await {
                Ok(server) => {
                    drop(mngr);
                    {
                        let server_inst = server.lock().await;
                        server_inst.send_event(&ServerEvent::Authorized).unwrap();
                    }
                    debug!("server authorized");
                    init_server_connection(ws, (tx, rx), manager, server).await;
                },
                Err(err) => {
                    trace!("Server auth failed: {}", err);
                    send_err(&mut ws, err).await;
                }
            }
        }
    }
    // Socket exiting
}

async fn init_client_connection(mut ws: WebSocket, (mut tx, mut rx): (UnboundedSender<WSMessage>, UnboundedReceiverStream<WSMessage>), manager: Manager, client: Client) {
    trace!("entering client read loop");
    // Timeout if client doesn't authorize within CLIENT_AUTH_TIMEOUT
    if let Err(_) = tokio::time::timeout(CLIENT_AUTH_TIMEOUT, wait_for_client_auth(client.clone())).await {
        send_err(&mut ws, AuthFailure::Timeout).await;
    } else {
        let (mut ws_tx, mut ws_rx) = ws.split();
        let client = client.clone();
        let manager = manager.clone();
        // Read incoming messages from websocket:
        tokio::task::spawn(async move {
            while let Some(Ok(message)) = ws_rx.next().await {
                match serde_json::from_str::<requests::ClientRequest>(&message.into_text().unwrap()) {
                    Ok(event) => {
                        debug!("got message from client {:?}", event);
                        let mut manager = manager.lock().await;
                        if let Err(err) = manager.on_client_request(&event, client.clone()).await {
                            error!("on_client_event error: {:?}", err);
                        }
                    },
                    Err(err) => {
                        tx.send(WSResponse::InvalidRequest { message: Some(err.to_string()) }.into()).ok();
                    }
                }
            }
        });

        // Send messages to client
        while let Some(msg) = rx.next().await {
            if let Err(e) = ws_tx.send(msg.0).await {
                break;
            }
        }
    }

    // Clean up client
    let client = client.lock().await;
    let id = client.id();
    drop(client);
    manager.lock().await.remove_client(&id);
}

/// Work around not being able to impl Into<Message>
pub struct WSMessage(pub Message);
impl Into<WSMessage> for WSResponse {
    fn into(self) -> WSMessage {
        WSMessage(Message::Text(serde_json::to_string(&self).unwrap()))
    }
}

/// Continuously checks client to see if it has authorized, sleeping in between
pub async fn wait_for_client_auth(client: Client) {
    loop {
        let client = client.lock().await;
        if client.is_authorized() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
        tokio::task::yield_now().await;
    }
}

async fn init_server_connection(mut ws: WebSocket, (mut tx, mut rx): (UnboundedSender<WSMessage>, UnboundedReceiverStream<WSMessage>), manager: Manager, server: Server) {
    trace!("entering server read loop");
    let (mut ws_tx, mut ws_rx) = ws.split();
    {
        let server = server.clone();
        let manager = manager.clone();
        tokio::task::spawn(async move {
            while let Some(Ok(message)) = ws_rx.next().await {
                match serde_json::from_str::<ServerRequest>(&message.into_text().unwrap()) {
                    Ok(request) => {
                        debug!("got message from server {:?}", request);
                        let mut manager = manager.lock().await;
                        if let Err(err) = manager.on_server_request(&request, server.clone()).await {
                            error!("on_server_request error: {:?}", err);
                        }
                    }
                    Err(e) => {
                        trace!("server: invalid payload {}", e);
                        tx.send(WSResponse::InvalidRequest { message: Some(e.to_string()) }.into()).ok();
                    }
                }
            }
        });
    }

    while let Some(msg) = rx.next().await {
        if let Err(e) = ws_tx.send(msg.0).await {
            break;
        }
    }

    // Cleanup server
    let mut server = server.lock().await;
    server.notify_disconnect().await;
    let id = server.id();
    manager.lock().await.remove_server(&id);
    drop(server);
}

async fn send(ws: &mut WebSocket, response: WSResponse) -> bool {
    match serde_json::to_string(&response) {
        Ok(json) => {
            ws.send(Message::Text(json)).await.is_ok()
        },
        Err(e) => false
    }
}
async fn send_err(ws: &mut WebSocket, error: AuthFailure) -> bool {
    send(ws, WSResponse::Error { error }).await
}