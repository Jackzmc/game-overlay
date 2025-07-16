mod client;
mod server;
mod manager;
mod steam;

use axum_template::engine::Engine;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::{ json, Value};
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{atomic::{Ordering}, Arc, Mutex, OnceLock};
use std::time::Duration;
use std::{env};
use tokio::sync::{RwLock};

use crate::manager::{Manager, ManagerInstance};
use crate::steam::{SteamClient};
use crate::web::get_http_client;
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use sha2::digest::KeyInit;
use sqlx::{MySqlPool};

mod web;
mod defs;
mod json;

static CLIENT_AUTH_TIMEOUT: Duration = Duration::from_secs(60 * 3);
static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);
static JWT_SECRET_KEY: Lazy<hmac::Hmac<sha2::Sha256>> = Lazy::new(|| {
    let raw = std::env::var("JWT_SECRET").expect("missing JWT_SECRET env");
    hmac::Hmac::new_from_slice(raw.as_bytes()).expect("could not generate Hmac<Sha256> from JWT_SECRET")
});

static POOL: OnceLock<MySqlPool> = OnceLock::new();

pub type AppEngine = Engine<Handlebars<'static>>;
struct ServeDir(PathBuf);
#[derive(Clone)]
struct AppState {
    manager: Manager,
    steam: SteamClient,
    http: reqwest::Client,
    engine: AppEngine,
    public_url: String
}

impl AppState {

    pub async fn new(public_url: String) -> Self {
        let http_client = get_http_client();
        let steam = SteamClient::new(http_client.clone(), env::var("STEAM_APIKEY").expect("missing STEAM_APIKEY"));
        let manager = ManagerInstance::new(steam.clone());
        let manager: Manager = Arc::new(tokio::sync::Mutex::new(manager));

        let hb = web::get_template_engine();

        Self {
            manager,
            steam,
            http: http_client,
            engine: Engine::from(hb),
            public_url
        }
    }
}
#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();
    if std::env::var("RUST_LOG").is_err() {
        // TODO: use same logger as overlay. this is safe, before any threads created:
        unsafe { std::env::set_var("RUST_LOG", format!("warn,{}=info", env!("CARGO_PKG_NAME"))); }
    }
    if env::var("STEAM_DONT_VALIDATE").is_ok() {
        warn!("Env STEAM_DONT_VALIDATE is set, validation of steam logins will not take place");
    }
    pretty_env_logger::init();
    setup_pool().await;


    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
    let listen_host = std::env::var("LISTEN_HOST").unwrap_or_else(|_| "127.0.0.1:3011".to_string());



    match tokio::net::TcpListener::bind(&listen_host).await {
        Ok(listener) => {
            let public_url = env::var("PUBLIC_URL").unwrap_or_else(|_| format!("http://localhost:{}", listener.local_addr().unwrap().port()));
            info!("listening on {}", listen_host);
            info!("public url: {}", public_url);

            let state = AppState::new(public_url).await;

            let app = web::routes::get_router()
                .with_state(Arc::new(state))
                .with_state(AppEngine::new(Handlebars::new()));
            axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
        },
        Err(e) => {
            error!("FATAL: {}", e);
            std::process::exit(1);
        }
    }
}

async fn setup_pool() {
    let pool = async {
        let url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not set".to_owned())?;
        MySqlPool::connect(&url).await.map_err(|e| e.to_string())
    };
    let pool = match pool.await {
        Ok(pool) => pool,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };
    POOL.set(pool).unwrap();
}
