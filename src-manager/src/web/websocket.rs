use axum::extract::ws::{Message, WebSocket};
use std::net::SocketAddr;
use log::{debug, error, trace, warn};
use futures_util::{SinkExt, StreamExt};
use overlay_manager::{InitConnectionReqPayload, InitConnectionResPayload};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use overlay_common::{requests, AuthFailure};
use overlay_common::events::ServerEvent;
use std::ops::Deref;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use overlay_common::requests::ServerRequest;
use crate::manager::{Client, Manager, Server};
use crate::{CLIENT_AUTH_TIMEOUT, PUBLIC_URL};

pub async fn init_connection(mut ws: WebSocket, addr: SocketAddr, manager: Manager) {
    debug!("init_connection addr={:?}", addr);
    // let (mut conn_tx, mut conn_rx) = ws.split();
    // Wait for the first initial connection
    tokio::task::spawn(async move {
        if let Some(Ok(message)) = ws.next().await {
            trace!("incoming msg");
            match serde_json::from_str::<InitConnectionReqPayload>(&message.into_text().unwrap()) {
                Ok(json) => {
                    login_connection(ws, manager, json, addr).await;
                },
                Err(err) => {
                    warn!("invalid payload: {}", err);
                    send(&mut ws, InitConnectionResPayload::InvalidPayload { message: Some(err.to_string()) }).await;
                    ws.close().await.ok();
                }
            }
        }
    });

}

async fn login_connection(mut ws: WebSocket, manager: Manager, req: InitConnectionReqPayload, addr: SocketAddr) {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);
    let mut mngr = manager.lock().await;
    // TODO: add timeout, to remove temp clients
    match req {
        InitConnectionReqPayload::Client { auth_token} => {
            debug!("login_connection - creating client");
            match mngr.start_client(addr, tx.clone()) {
                Ok((client, id)) => {
                    if let Some(token) = auth_token {
                        if let Err(err) = mngr.authorize_client_token(&id, token).await {
                            // Cleanup ID
                            send(&mut ws, InitConnectionResPayload::AuthError(AuthFailure::General { message: err.to_string() })).await;
                            mngr.remove_client(&id);
                            return;
                        }
                    } else {
                        send(&mut ws, InitConnectionResPayload::PendingClientLogin { url: format!("{}/auth/login?id={id}", PUBLIC_URL.deref()) }).await;
                    }
                    drop(mngr);
                    init_client_connection(ws, (tx, rx), manager, client).await;
                },
                Err(e) => {
                    send(&mut ws, InitConnectionResPayload::AuthError(e)).await;
                }
            }
        }
        InitConnectionReqPayload::Server { auth_token } => {
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
                Err(e) => {
                    trace!("Server auth failed: {}", e);
                    send(&mut ws, InitConnectionResPayload::AuthError(e)).await;
                }
            }
        }
    }
    // Socket exiting
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

async fn init_client_connection(mut ws: WebSocket, (mut tx, mut rx): (UnboundedSender<Message>, UnboundedReceiverStream<Message>), manager: Manager, client: Client) {
    trace!("entering client read loop");
    // Timeout if client doesn't authorize within CLIENT_AUTH_TIMEOUT
    if let Err(_) = tokio::time::timeout(CLIENT_AUTH_TIMEOUT, wait_for_client_auth(client.clone())).await {
        send(&mut ws, InitConnectionResPayload::AuthError(AuthFailure::Timeout)).await;
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
                    Err(e) => {
                        tx.send(InitConnectionResPayload::InvalidPayload { message: Some(e.to_string()) }.into()).ok();
                    }
                }
            }
        });

        // Send messages to client
        while let Some(msg) = rx.next().await {
            if let Err(e) = ws_tx.send(msg).await {
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

async fn init_server_connection(mut ws: WebSocket, (mut tx, mut rx): (UnboundedSender<Message>, UnboundedReceiverStream<Message>), manager: Manager, server: Server) {
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
                        tx.send(InitConnectionResPayload::InvalidPayload { message: Some(e.to_string()) }.into()).ok();
                    }
                }
            }
        });
    }

    while let Some(msg) = rx.next().await {
        if let Err(e) = ws_tx.send(msg).await {
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

async fn send(ws: &mut WebSocket, response: InitConnectionResPayload) -> bool {
    match serde_json::to_string(&response) {
        Ok(json) => {
            ws.send(Message::Text(json)).await.is_ok()
        },
        Err(e) => false
    }

}