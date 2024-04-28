mod client;
mod server;
mod manager;

// #![deny(warnings)]
use std::collections::HashMap;
use std::{env, fs};
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt, TryFutureExt, TryStreamExt};
use handlebars::Handlebars;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::{Filter, method, reject, Rejection, Reply, reply};
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
use once_cell::sync::Lazy;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use warp::http::StatusCode;

type QueryMap = HashMap<String, String>;
static CLIENT_AUTH_TIMEOUT: Duration = Duration::from_secs(60 * 4);
static LISTEN_ADDRESS: Lazy<SocketAddr> = Lazy::new(|| std::env::var("LISTEN_HOST").unwrap_or_else(|_| "127.0.0.1:3011".to_string())
    .parse().expect("bad LISTEN_HOST"));
static PUBLIC_URL: Lazy<String> = Lazy::new(|| std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:3011".to_string()) );


#[derive(Debug)]
struct MissingQueryParameter(String);

impl reject::Reject for MissingQueryParameter {}
#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let manager_inst = Manager::default();
    let manager = warp::any().map(move || manager_inst.clone());

    let mut hb = Handlebars::new();
    load_templates(&mut hb);
    let hb = Arc::new(hb);
    let handlebars = move |with_template| render(with_template, hb.clone());

    let socket = warp::path("socket")
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(manager.clone())
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

    let login = warp::path!("auth"/"login")
        .and(warp::query::<QueryMap>())
        .and(manager.clone())
        .and(remote())
        .and_then(|query: QueryMap, manager: Manager, addr: Option<SocketAddr>| async move  {
            if let Some(id) = query.get("id") {
                if manager.lock().await.verify_client(id, &addr.unwrap()).await {
                    Ok(id.to_string())
                } else {
                    Err(reject::not_found())
                }
            } else {
                Err(reject::custom(MissingQueryParameter("id".to_string())))
            }
        })
        .map(|id: String| WithTemplate {
            name: "login",
            value: json!({
                "host": PUBLIC_URL.deref(),
                "id": id
            }),
        })
        .map(handlebars)
        .recover(handle_rejection);

    // let callback = warp::path!("auth" / "callback")
    //     .and(method::post())
    //     .and(warp::query::<HashMap<String, String>>())
    //     .and(manager)
    //     .and(remote())
    //     .map(|manager: Manager, addr: Option<SocketAddr>| {
    //
    //     });

    let routes = socket.or(login);//.or(callback);

    warp::serve(routes).run(*LISTEN_ADDRESS).await;
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: Option<String>
}
async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let code;
    let response: ErrorResponse;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        response = ErrorResponse {
            error: "NOT_FOUND".to_string(),
            message: Some("Resource not found".to_string())
        }
    } else if let Some(MissingQueryParameter(param)) = err.find() {
        code = StatusCode::BAD_REQUEST;
        response = ErrorResponse {
            error: "QUERY_PARAMETER_REQUIRED".to_string(),
            message: Some(format!("{param} is a required query parameter"))
        }
    }  else {
        // We should have expected this... Just log and say its a 500
        eprintln!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        response = ErrorResponse {
            error: "UNHANDLED_REJECTION".to_string(),
            message: None
        };
    }

    let json = warp::reply::json(&response);
    Ok(warp::reply::with_status(json, code))
}

struct WithTemplate<T: Serialize> {
    name: &'static str,
    value: T,
}

fn render<T>(template: WithTemplate<T>, hbs: Arc<Handlebars<'_>>) -> impl warp::Reply
    where
        T: Serialize,
{
    let render = hbs
        .render(template.name, &template.value)
        .unwrap_or_else(|err| err.to_string());
    warp::reply::html(render)
}
fn load_templates(hb: &mut Handlebars) {
    match fs::read_dir(env::current_dir().unwrap().join("templates")) {
        Ok(files) => {
            for entry in files {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    let path = entry.path();
                    let name = path.file_stem().unwrap().to_str().unwrap();
                    debug!("registering template \"{}\"", name);
                    hb.register_template_file(name, &path).unwrap()
                }
            }
        },
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                warn!("No templates folder found, no templates will be loaded. ({})", e);
            } else {
                panic!("load_templates: {}", e);
            }
        }
    }
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
impl Into<Message> for InitConnectionResPayload {
    fn into(self) -> Message {
        Message::text(serde_json::to_string(&self).unwrap())
    }
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
                    send(&mut ws, InitConnectionResPayload::InvalidPayload { message: Some(err.to_string()) }).await;
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
                    send(&mut ws, InitConnectionResPayload::PendingClientLogin { url: format!("{}/auth/login?id={id}", PUBLIC_URL.deref()) }).await;
                    drop(mngr);
                    init_client_connection(ws, (tx,rx), manager, client).await;
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
                    init_server_connection(ws, (tx, rx), manager, server).await;
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

async fn init_client_connection(mut ws: WebSocket, (mut tx, mut rx): (UnboundedSender<Message>, UnboundedReceiverStream<Message>), manager: Manager, client: Client) {
    debug!("entering client read loop");
    // Timeout if client doesn't authorize within CLIENT_AUTH_TIMEOUT
    if let Err(_) = tokio::time::timeout(CLIENT_AUTH_TIMEOUT, wait_for_client_auth(client.clone())).await {
        send(&mut ws, InitConnectionResPayload::AuthFailure(AuthFailure::Timeout)).await;
    } else {
        let (mut ws_tx, mut ws_rx) = ws.split();
        // Read incoming messages from websocket:
        tokio::task::spawn(async move {
            while let Some(Ok(message)) = ws_rx.next().await {
                debug!("got message from client");
                match serde_json::from_str::<ClientOutgoingEvent>(message.to_str().unwrap()) {
                    Ok(event) => {
                        let mut manager = manager.lock().await;
                        if let Err(err) = manager.on_client_event(&event, client.clone()).await {
                            error!("on_client_event error: {:?}", err);
                        }
                    },
                    Err(e) => {
                        tx.send(InitConnectionResPayload::InvalidPayload { message: Some(e.to_string()) }.into()).unwrap();
                    }
                }
            }
        });

        // Send messages to client
        while let Some(msg) = rx.next().await {
            ws_tx.send(msg).await.unwrap();
        }
    }
}
async fn init_server_connection(mut ws: WebSocket, (mut tx, mut rx): (UnboundedSender<Message>, UnboundedReceiverStream<Message>), manager: Manager, server: Server) {
    debug!("entering server read loop");
    let (mut ws_tx, mut ws_rx) = ws.split();
    tokio::task::spawn(async move {
        while let Some(Ok(message)) = ws_rx.next().await {
            debug!("got message from server");
            match serde_json::from_str::<ServerOutgoingEvent>(message.to_str().unwrap()) {
                Ok(event) => {
                    let mut manager = manager.lock().await;
                    if let Err(err) = manager.on_server_event(&event, server.clone()).await {
                        error!("on_server_event error: {:?}", err);
                    }
                },
                Err(e) => {
                    tx.send(InitConnectionResPayload::InvalidPayload { message: Some(e.to_string()) }.into()).unwrap();
                }
            }
        }
    });

    while let Some(msg) = rx.next().await {
        ws_tx.send(msg).await.unwrap();
    }
}
async fn send(ws: &mut WebSocket, response: InitConnectionResPayload) -> bool {
    let json = serde_json::to_string(&response).unwrap();
    ws.send(Message::text(json)).await.is_ok()
}
