use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use axum::extract::{ConnectInfo, Query, State, WebSocketUpgrade};
use axum::extract::ws::WebSocket;
use axum::response::IntoResponse;
use axum::Router;
use axum::routing::get;
use axum_template::RenderHtml;
use tracing::debug;
use serde_json::json;
use steamid_ng::SteamID;
use crate::{AppState, PUBLIC_URL};
use crate::web::{AppError, OpenIdCallback};
use crate::web::websocket::init_connection;

pub(crate) fn get_router() -> Router<Arc<AppState>> {
   Router::new()
        // .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/socket", get(route_socket))
        .route("/auth/login", get(route_steam_login))
        .route("/auth/callback", get(route_steam_callback))
        .route("/manage", get(route_manage_ui))
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
    let steamid2: u64 = steamid2.parse().unwrap();
    let steamid = SteamID::from(steamid2);
    state.steam.verify_openid(&mut query.openid).await
        .map_err(|e| AppError::GenericServerError { message: e.to_string() })?;
    debug!("auth success, authorizing with manager");
    let mut manager = state.manager.lock().await;
    manager.mark_client_authorized(&query.id, steamid.clone()).await
        .map_err(|e| AppError::GenericServerError { message: e.to_string() })?;
    Ok(RenderHtml("login_success", state.engine.clone(), json!({})))
}


async fn route_manage_ui(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    RenderHtml("manage", state.engine.clone(), json!({

    }))
}