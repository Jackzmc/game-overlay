use axum::extract::{FromRequest, MatchedPath, Request};
use axum::extract::rejection::JsonRejection;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use axum::extract::ws::Message as AxumMessage;
use axum::http::{StatusCode};
use axum::{Json, RequestPartsExt};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use overlay_common::ws::AuthFailure;

pub struct ResponseError(pub AuthFailure);

impl From<AuthFailure> for ResponseError {
    fn from(err: AuthFailure) -> Self {
        ResponseError(err)
    }
}

impl ResponseError {
    fn status_code(&self) -> StatusCode {
        match self.0 {
            AuthFailure::Unknown => StatusCode::BAD_REQUEST,
            AuthFailure::ObjectNotFound => StatusCode::NOT_FOUND,
            AuthFailure::InvalidAuthToken { .. } => StatusCode::BAD_REQUEST,
            AuthFailure::IPMismatch { .. } => StatusCode::UNAUTHORIZED,
            AuthFailure::Timeout { .. } => StatusCode::REQUEST_TIMEOUT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::response::Response {
        (self.status_code(), Json(self.0)).into_response()
    }
}

