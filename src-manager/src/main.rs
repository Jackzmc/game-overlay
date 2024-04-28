mod client;
mod server;
mod manager;

// #![deny(warnings)]
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt, TryFutureExt, TryStreamExt};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::{Filter, method, Reply, reply};
use log::{debug, error, info, warn};
use serde_json::{json, Value};
use steamid_ng::SteamID;
use uuid::Uuid;
use warp::addr::remote;
use serde::{Serialize,Deserialize};
use warp::filters::method;
use crate::client::ClientOutgoingEvent;
use crate::manager::{AuthFailure, Client, Manager, Server};
use crate::server::{ServerIncomingRequest, ServerOutgoingEvent};

type QueryMap = HashMap<String, String>;
static CLIENT_AUTH_TIMEOUT: Duration = Duration::from_secs(60 * 4);


#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    pretty_env_logger::init();
    let manager_inst = Manager::default();

    let manager = warp::any().map(move || manager_inst.clone());

    // GET /chat -> websocket upgrade
    let socket = warp::path("socket")
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(manager)
        .and(remote())
        .map(|ws: warp::ws::Ws, manager: Manager, addr: Option<SocketAddr>| -> Box<dyn warp::Reply> {
            if let Some(addr) = addr {
                Box::new(ws.on_upgrade(move |socket| init_connection(socket, addr, manager.clone())))
            } else {
                Box::new(warp::reply::with_status(warp::reply::json(&json!({
                    "error": "UNSUPPORTED_TRANSPORT",
                    "message": "Transport does not provide IP addresses, unsupported"
                })), warp::http::StatusCode::BAD_REQUEST))
            }
        });

    // GET / -> index html
    // let login = warp::path!("auth"/"login")
    //     .and(warp::query::<QueryMap>())
    //     .and(manager)
    //     .and(remote())
    //     .and_then(|manager: Manager, addr: Option<SocketAddr>, query: QueryMap| async move {
    //         let id = query.get("id");
    //         match query.get("id") {
    //             Some(id) => {
                // TODO: need html form that then has user submit POST
    //                 // warp::reply::with_status(warp::reply::with_header("Location", ""))
    //             },
    //             None => Err(warp::reject::not_found())
    //         }
    //     });
    // let callback = warp::path!("auth" / "callback")
    //     .and(method::post())
    //     .and(warp::query::<HashMap<String, String>>())
    //     .and(manager)
    //     .and(remote())
    //     .map(|manager: Manager, addr: Option<SocketAddr>| {
    //
    //     });

    let routes = socket; //.or(login).or(callback);

    let host: SocketAddr = std::env::var("HOSTNAME").unwrap_or_else(|_| "127.0.0.1:3011".to_string())
        .parse().expect("bad HOSTNAME");
    warp::serve(routes).run(host).await;
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum InitConnectionReqPayload {
    Client {},
    Server { auth_token: String }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "result")]
enum InitConnectionResPayload {
    PendingClientLogin { url: String },
    ServerAuthorized,
    InvalidPayload { message: Option<String> },
    AuthFailure(AuthFailure)
}

async fn init_connection(mut ws: WebSocket, addr: SocketAddr, manager: Manager) {
    debug!("init_connection addr={:?}", addr);
    // let (mut conn_tx, mut conn_rx) = ws.split();
    // Wait for the first initial connection
    tokio::task::spawn(async move {
        if let Some(Ok(message)) = ws.next().await {
            debug!("incoming msg");
            match serde_json::from_str::<InitConnectionReqPayload>(message.to_str().unwrap()) {
                Ok(json) => {
                    login_connection(ws, manager, json, addr).await;
                },
                Err(err) => {
                    warn!("invalid payload: {}", err);
                    let json = serde_json::to_string(&InitConnectionResPayload::InvalidPayload { message: None }).unwrap();
                    ws.send(Message::text(json)).await.ok();
                    ws.close().await.unwrap();
                }
            }
        }
    });

}

async fn login_connection(mut ws: WebSocket, manager: Manager, req: InitConnectionReqPayload, addr: SocketAddr) {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);
    let mut mngr = manager.lock().await;

    match req {
        InitConnectionReqPayload::Client { } => {
            debug!("login_connection - starting temp client");
            match mngr.start_temp_client(addr, tx.clone()) {
                Ok(client) => {
                    let id = client.lock().await.id();
                    let host = std::env::var("PUBLIC_URL").unwrap_or_else(|_| "localhost:3011".to_string());
                    send(&mut ws, InitConnectionResPayload::PendingClientLogin { url: format!("{host}/auth/login?id={id}") }).await;
                    drop(mngr);
                    init_client_connection(ws, manager, client).await;
                },
                Err(e) => {
                    send(&mut ws, InitConnectionResPayload::AuthFailure(e)).await;
                }
            }
        }
        InitConnectionReqPayload::Server { auth_token } => {
            debug!("login_connection - authorizing server");
            match mngr.try_authorize_server(addr, tx.clone(), auth_token) {
                Ok(server) => {
                    send(&mut ws, InitConnectionResPayload::ServerAuthorized).await;
                    drop(mngr);
                    init_server_connection(ws, manager, server).await;
                },
                Err(e) => {
                    send(&mut ws, InitConnectionResPayload::AuthFailure(e)).await;
                }
            }
        }
    }
    // Socket exiting
}
/// Continuously checks client to see if it has authorized, sleeping in between
async fn wait_for_client_auth(client: Client) {
    loop {
        let client = client.lock().await;
        if client.is_authorized() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        tokio::task::yield_now().await;
    }
}

async fn init_client_connection(mut ws: WebSocket, manager: Manager, client: Client) {
    debug!("entering client read loop");
    // Timeout if client doesn't authorize within CLIENT_AUTH_TIMEOUT
    if let Err(_) = tokio::time::timeout(CLIENT_AUTH_TIMEOUT, wait_for_client_auth(client.clone())).await {
        send(&mut ws, InitConnectionResPayload::AuthFailure(AuthFailure::Timeout)).await;
    } else {
        tokio::task::spawn(async move {
            while let Some(Ok(message)) = ws.next().await {
                debug!("got message from client");
                match serde_json::from_str::<ClientOutgoingEvent>(message.to_str().unwrap()) {
                    Ok(event) => {
                        let mut manager = manager.lock().await;
                        if let Err(err) = manager.on_client_event(&event, client.clone()).await {
                            error!("on_client_event error: {:?}", err);
                        }
                    },
                    Err(e) => {
                        send(&mut ws, InitConnectionResPayload::InvalidPayload { message: Some(e.to_string()) }).await;
                    }
                }
            }
        });
    }
}
async fn init_server_connection(mut ws: WebSocket, manager: Manager, server: Server) {
    debug!("entering server read loop");
    tokio::task::spawn(async move {
        while let Some(Ok(message)) = ws.next().await {
            debug!("got message from server");
            match serde_json::from_str::<ServerOutgoingEvent>(message.to_str().unwrap()) {
                Ok(event) => {
                    let mut manager = manager.lock().await;
                    if let Err(err) = manager.on_server_event(&event, server.clone()).await {
                        error!("on_server_event error: {:?}", err);
                    }
                },
                Err(e) => {
                    send(&mut ws, InitConnectionResPayload::InvalidPayload { message: Some(e.to_string()) }).await;
                }
            }
        }
    });
}
async fn send(ws: &mut WebSocket, response: InitConnectionResPayload) -> bool {
    let json = serde_json::to_string(&response).unwrap();
    ws.send(Message::text(json)).await.is_ok()
}