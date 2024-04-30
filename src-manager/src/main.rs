mod client;
mod server;
mod manager;
mod util;
mod steam;

// #![deny(warnings)]
use std::collections::HashMap;
use std::{env, fs};
use std::cell::OnceCell;
use std::future::Future;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc, OnceLock, Mutex};
use std::time::Duration;
use axum::{Router, ServiceExt};
use axum::body::Body;
use axum::extract::{ConnectInfo, Query, State};
use axum::routing::{get, post};
use axum_template::engine::Engine;
use futures_util::{SinkExt, StreamExt, TryFutureExt, TryStreamExt};
use handlebars::Handlebars;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use log::{debug, error, info, warn};
use serde_json::{json, Value};
use steamid_ng::SteamID;
use uuid::Uuid;
use serde::{Serialize,Deserialize};

use crate::client::{ClientIncomingRequest, ClientOutgoingEvent};
use crate::manager::{AuthFailure, Client, Manager, ManagerInstance, Server};
use crate::server::{ServerIncomingRequest, ServerOutgoingEvent};
use once_cell::sync::Lazy;
use sha2::digest::KeyInit;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::steam::{OpenIDPayload, SteamClient, SteamUser};
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::{Message, WebSocket};
use axum::http::response::Parts;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum_template::{RenderHtml, TemplateEngine};
use tokio::time::timeout;

type QueryMap = HashMap<String, String>;
static CLIENT_AUTH_TIMEOUT: Duration = Duration::from_secs(60 * 4);
static LISTEN_ADDRESS: Lazy<SocketAddr> = Lazy::new(|| std::env::var("LISTEN_HOST").unwrap_or_else(|_| "127.0.0.1:3011".to_string())
    .parse().expect("bad LISTEN_HOST"));
static PUBLIC_URL: Lazy<String> = Lazy::new(|| std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:3011".to_string()) );
static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);
static JWT_SECRET_KEY: Lazy<hmac::Hmac<sha2::Sha256>> = Lazy::new(|| {
    let raw = std::env::var("JWT_SECRET").expect("missing JWT_SECRET env");
    hmac::Hmac::new_from_slice(raw.as_bytes()).expect("could not generate Hmac<Sha256> from JWT_SECRET")
});

#[derive(Debug)]
struct MissingQueryParameter(String);
#[derive(Debug, Serialize)]
struct SteamAuthError(String);

struct ServeDir(PathBuf);
#[derive(Clone)]
struct AppState {
    manager: Manager,
    steam: SteamClient,
    http: reqwest::Client,
    engine: Engine<Handlebars<'static>>
}
impl AppState {

    pub fn new() -> Self {
        let http_client = get_client();
        let steam = SteamClient::new(http_client.clone(), std::env::var("STEAM_APIKEY").expect("missing STEAM_APIKEY"));
        let manager = ManagerInstance::new(steam.clone());
        let manager: Manager = Arc::new(tokio::sync::Mutex::new(manager));
        let mut hb = Handlebars::new();
        load_templates(&mut hb);

        Self {
            manager,
            steam,
            http: http_client,
            engine: Engine::from(hb),
        }
    }
}
#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", format!("warn,{}=info", env!("CARGO_PKG_NAME")));
    }
    if env::var("STEAM_DONT_VALIDATE").is_ok() {
        warn!("Env STEAM_DONT_VALIDATE is set, validation of steam logins will not take place");
    }
    pretty_env_logger::init();

    let state = AppState::new();
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");

    let app = Router::new()
        // .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/socket", get(route_socket))
        .route("/auth/login", get(route_steam_login))
        .route("/auth/callback", get(route_steam_callback))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind(*LISTEN_ADDRESS).await.unwrap();
    info!("listening on {}", LISTEN_ADDRESS.to_string());
    info!("public url: {}", PUBLIC_URL.deref());
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}

#[derive(Serialize)]
#[serde(tag = "error", rename = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "snake_case")]
enum AppError {
    SessionExpired,
    GenericServerError { message: String },
    EntityNotFound { message: String },
    MissingQueryParameter(String)
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::MissingQueryParameter(param) => {
                (StatusCode::BAD_REQUEST, serde_json::to_string(&json!({
                    "error": "MISSING_QUERY_PARAMETER",
                    "param": &param,
                    "message": format!("The parameter \"{param}\" is required").to_string()
                })))
            },
            e @ AppError::EntityNotFound { .. } => {
                (StatusCode::NOT_FOUND, serde_json::to_string(&e))
            },
            e @ AppError::GenericServerError { .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, serde_json::to_string(&e))
            },
            AppError::SessionExpired => {
                (StatusCode::NOT_FOUND, serde_json::to_string(&json!({
                    "error": "SESSION_EXPIRED",
                    "message": "Session has expired or id is invalid"
                })))
            }
        };

        axum::response::Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(Body::from(message.expect("could not serialize error response")))
            .unwrap()
    }
}

async fn route_socket(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse {
    let manager = state.manager.clone();
    ws.on_upgrade(move |socket: WebSocket| init_connection(socket, addr, manager))
}


async fn route_steam_login(
    Query(query): Query<HashMap<String, String>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse{
    if let Some(id) = query.get("id") {
        if state.manager.lock().await.verify_client(id, &addr).await {
            // Ok(state.engine.render("login", ).map_err(|e| AppError::GenericServerError { message: e.to_string() })?)
            Ok(RenderHtml("login", state.engine.clone(), json!({
                "host": PUBLIC_URL.deref(),
                "id": id
            })))
        } else {
            Err(AppError::SessionExpired)
        }
    } else {
        Err(AppError::MissingQueryParameter("id".to_string()))
    }
}

async fn route_steam_callback(
    Query(mut query): Query<OpenIdCallback>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>
) -> Result<impl IntoResponse, AppError> {
    let (_,steamid2) = query.openid.identity.rsplit_once("/").unwrap();
    debug!("raw: {}. steamid2: {}", query.openid.identity, steamid2);
    let steamid2: u64 = steamid2.parse().unwrap();
    let steamid = SteamID::from(steamid2);
    state.steam.verify_openid(&mut query.openid).await
        .map_err(|e| AppError::GenericServerError { message: e.0 })?;
    debug!("auth success, authorizing with manager");
    let mut manager = state.manager.lock().await;
    manager.authorize_client(&query.id, steamid.clone()).await
        .map_err(|e| AppError::GenericServerError { message: e.to_string() })?;
    Ok(RenderHtml("login_success", state.engine.clone(), json!({})))
}

#[derive(Serialize, Deserialize)]
struct OpenIdCallback {
    id: String,

    #[serde(flatten)]
    openid: OpenIDPayload
}

fn get_client() -> reqwest::Client {
    reqwest::Client::builder()
        .https_only(true)
        .user_agent(APP_USER_AGENT)
        .build()
        .expect("could not create HTTP client")
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    error: String,
    message: Option<String>
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
    Client { auth_token: Option<String> },
    Server { auth_token: String }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "result")]
enum InitConnectionResPayload {
    PendingClientLogin { url: String },
    ServerAuthorized,
    InvalidPayload { message: Option<String> },
    AuthError(AuthFailure)
}
impl Into<Message> for InitConnectionResPayload {
    fn into(self) -> Message {
        Message::Text(serde_json::to_string(&self).unwrap())
    }
}

async fn init_connection(mut ws: WebSocket, addr: SocketAddr, manager: Manager) {
    debug!("init_connection addr={:?}", addr);
    // let (mut conn_tx, mut conn_rx) = ws.split();
    // Wait for the first initial connection
    tokio::task::spawn(async move {
        if let Some(Ok(message)) = ws.next().await {
            debug!("incoming msg");
            match serde_json::from_str::<InitConnectionReqPayload>(&message.into_text().unwrap()) {
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
    // TODO: add timeout, to remove temp clients
    match req {
        InitConnectionReqPayload::Client { auth_token} => {
            debug!("login_connection - creating client");
            match mngr.start_client(addr, tx.clone()) {
                Ok((client, id)) => {
                    if let Some(token) = auth_token {
                        if let Err(err) = mngr.authorize_client_token(&id, token).await {
                            send(&mut ws, InitConnectionResPayload::AuthError(AuthFailure::General(err.to_string()))).await;
                        } else {
                            // Cleanup ID
                            mngr.remove_client(&id);
                        }
                    } else {
                        send(&mut ws, InitConnectionResPayload::PendingClientLogin { url: format!("{}/auth/login?id={id}", PUBLIC_URL.deref()) }).await;
                        drop(mngr);
                        init_client_connection(ws, (tx, rx), manager, client).await;
                    }
                },
                Err(e) => {
                    send(&mut ws, InitConnectionResPayload::AuthError(e)).await;
                }
            }
        }
        InitConnectionReqPayload::Server { auth_token } => {
            debug!("login_connection - authorizing server");
            match mngr.try_authorize_server(addr, tx.clone(), auth_token) {
                Ok(server) => {
                    drop(mngr);
                    {
                        let server_inst = server.lock().await;
                        server_inst.send_request(&ServerIncomingRequest::Authorized).unwrap();
                    }
                    init_server_connection(ws, (tx, rx), manager, server).await;
                },
                Err(e) => {
                    send(&mut ws, InitConnectionResPayload::AuthError(e)).await;
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
        send(&mut ws, InitConnectionResPayload::AuthError(AuthFailure::Timeout)).await;
    } else {
        let (mut ws_tx, mut ws_rx) = ws.split();
        let client = client.clone();
        let manager = manager.clone();
        // Read incoming messages from websocket:
        tokio::task::spawn(async move {
            while let Some(Ok(message)) = ws_rx.next().await {
                debug!("got message from client");
                match serde_json::from_str::<ClientOutgoingEvent>(&message.into_text().unwrap()) {
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
    debug!("entering server read loop");
    let (mut ws_tx, mut ws_rx) = ws.split();
    {
        let server = server.clone();
        let manager = manager.clone();
        tokio::task::spawn(async move {
            while let Some(Ok(message)) = ws_rx.next().await {
                debug!("got message from server");
                match serde_json::from_str::<ServerOutgoingEvent>(&message.into_text().unwrap()) {
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
    }

    while let Some(msg) = rx.next().await {
        if let Err(e) = ws_tx.send(msg).await {
            break;
        }
    }

    // Cleanup server
    let server = server.lock().await;
    let id = server.id();
    drop(server);
    manager.lock().await.remove_server(&id);
}
async fn send(ws: &mut WebSocket, response: InitConnectionResPayload) -> bool {
    let json = serde_json::to_string(&response).unwrap();
    ws.send(Message::Text(json)).await.is_ok()
}
