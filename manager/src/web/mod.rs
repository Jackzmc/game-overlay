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
use crate::{AppEngine, APP_USER_AGENT};
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

pub fn get_template_engine() -> AppEngine {
    let mut hbs = Handlebars::new();
    match fs::read_dir(env::current_dir().unwrap().join("templates")) {
        Ok(files) => {
            for entry in files {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    let path = entry.path();
                    let name = path.file_stem().unwrap().to_str().unwrap();
                    debug!("registering template \"{}\"", name);
                    hbs.register_template_file(name, &path).unwrap()
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
    Engine::from(hbs)
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

