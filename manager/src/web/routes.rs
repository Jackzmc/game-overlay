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
use serde::{Deserialize, Serialize};
use tracing::debug;
use serde_json::json;
use steamid_ng::SteamID;
use overlay_common::ws::{AppError, AuthFailure, AuthReq_Server, WSRequest, WSResponse};
use overlay_common::ws::AuthFailure::Timeout;
use crate::{AppEngine, AppState};
use crate::defs::ResponseError;
use crate::json::AppJson;
use crate::web::{OpenIdCallback};
use crate::web::websocket::setup_conn;

pub(crate) fn get_router() -> Router<Arc<AppState>> {
   Router::new()
        // .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/socket", get(route_socket))
        .route("/auth/server", post(route_auth_server))
        .route("/auth/login", get(route_steam_login))
        .route("/auth/callback", get(route_steam_callback))

       .route("/req", get(route_request))
}

async fn route_request(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AppJson(body): AppJson<WSRequest>
) -> Result<impl IntoResponse, ResponseError> {
    // headers.get("authorization") // TODO: validate
    if let WSRequest::Server(req) = body {
        if let Some(sess_token) = headers.get("authorization") {
            let sess_token = sess_token.to_str().map_err(|_|AppError::BadRequest { message: Some("bad session token string".to_string()) })?;
            let manager = state.manager.clone();
            let mut manager = manager.lock().await;
            let server = manager.get_session_server(sess_token)?;

            // let server = server.lock().await;
            manager.on_server_request(&req, server).await?;
            return Ok((StatusCode::NO_CONTENT, ""));
        }
        Err(AppError::AuthError(AuthFailure::InvalidAuthToken { message: Some("Session token is missing".to_string()) }).into())
    } else {
        Err(AppError::AuthError(AuthFailure::BadRequest { message: "expected server request got another request type".to_string() }).into())
    }

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
    AppJson(body): AppJson<AuthReq_Server>
) -> Result<Json<WSResponse>, ResponseError> {
    let manager = state.manager.clone();
    let mut lock = manager.lock().await;
    debug!("got auth request: {:?}", body);
    let (sess_token, expires_at) = lock.server_start_session(addr.ip(), body)?;;
    Ok(Json(WSResponse::SessionStarted { sess_token, expires_at }))
}

#[derive(Deserialize)]
struct SteamLogin {
    id: String
}
async fn route_steam_login(
    query: Query<SteamLogin>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ResponseError> {
    if state.manager.lock().await.verify_client(&query.id, &addr).await {
        Ok((StatusCode::OK, ""))
    } else {
        Err(AppError::AuthError(Timeout).into())
    }
}
async fn route_steam_callback(
    Query(mut query): Query<OpenIdCallback>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>
) -> Result<impl IntoResponse, ResponseError> {
    let (_,steamid2) = query.openid.identity.rsplit_once("/").unwrap();
    let steamid2: u64 = steamid2.parse().unwrap();
    let steamid = SteamID::from(steamid2);
    state.steam.verify_openid(&mut query.openid).await
        .map_err(|e| AppError::InternalServerError { message: e.to_string() })?;
    debug!("auth success, authorizing with manager");
    let mut manager = state.manager.lock().await;
    manager.mark_client_authorized(&query.id, steamid.clone()).await
        .map_err(|e| AppError::InternalServerError { message: e.to_string() })?;
    // Ok(RenderHtml("login_success", state.engine.clone(), json!({})))
    Ok((StatusCode::OK, ""))

}