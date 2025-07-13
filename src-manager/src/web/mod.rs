use std::{env, fs};
use std::io::ErrorKind;
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum_template::engine::Engine;
use handlebars::Handlebars;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::APP_USER_AGENT;
use crate::steam::OpenIDPayload;

pub mod routes;
pub mod websocket;

pub fn get_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .https_only(true)
        .user_agent(APP_USER_AGENT)
        .build()
        .expect("could not create HTTP client")
}

pub fn get_template_engine() -> Engine<Handlebars<'static>> {
    let mut hb = Handlebars::new();
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
    Engine::from(hb)
}

#[derive(Serialize)]
#[serde(tag = "error", rename = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "snake_case")]
enum AppError {
    SessionExpired,
    GenericServerError { message: String },
    EntityNotFound { message: String },
    MissingQueryParameter(String),
    DatabaseError { message: String }
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
            e @ AppError::DatabaseError { .. } => {
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

#[derive(Serialize, Deserialize)]
pub struct OpenIdCallback {
    id: String,

    #[serde(flatten)]
    openid: OpenIDPayload
}


#[derive(Serialize, Debug)]
struct ErrorResponse {
    error: String,
    message: Option<String>
}

