use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use axum::extract::{ConnectInfo, Query, State, WebSocketUpgrade};
use axum::extract::ws::WebSocket;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use axum::routing::{get, post};
use axum_template::RenderHtml;
use jwt::Header;
use log::__private_api::Value;
use tracing::debug;
use serde_json::json;
use steamid_ng::SteamID;
use overlay_common::ws::{AuthFailure, AuthReq_Server, WSRequest, WSResponse};
use crate::{AppState};
use crate::defs::ResponseError;
use crate::web::{AppError, OpenIdCallback};
use crate::web::websocket::setup_conn;

pub(crate) fn get_router() -> Router<Arc<AppState>> {
   Router::new()
        // .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/socket", get(route_socket))
        .route("/auth/server", post(route_auth_server))
        .route("/auth/login", get(route_steam_login))
        .route("/auth/callback", get(route_steam_callback))
        .route("/manage", get(route_manage_ui))

       .route("/req", get(route_request))
}

async fn route_request(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<WSRequest>
) -> impl IntoResponse {
    let manager = state.manager.clone();

}


async fn route_socket(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse {
    let manager = state.manager.clone();
    ws.on_upgrade(move |socket: WebSocket| setup_conn(socket, addr, manager, state.public_url.clone()))
}

async fn route_auth_server(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthReq_Server>
) -> Result<Json<WSResponse>, ResponseError> {
    let manager = state.manager.clone();
    let mut lock = manager.lock().await;
    debug!("got auth request: {:?}", body);
    let (sess_token, expires_at) = lock.server_start_session(addr.ip(), body).await?;;
    Ok(Json(WSResponse::SessionStarted { sess_token, expires_at }))
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
                "host": state.public_url.clone(),
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