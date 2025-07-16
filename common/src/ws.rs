use std::sync::mpsc::SendError;
use std::error::Error;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::game::TeamConfig;
use crate::requests::ServerRequest;
use crate::events::ServerEvent;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
/// Requests from connections before authenticated
/// After authentication, it becomes ClientRequest or ServerRequest
pub enum AuthRequest {
    Client { auth_token: Option<String> },
    Server(AuthReq_Server)
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthReq_Server {
    pub auth_token: String,
    pub info: InitialServerInfo
}

/// Initial server information, sent in authentication, before server is registered
/// Necessary to prevent Option<...> in server
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InitialServerInfo {
    pub hostname: String,
    pub teams: Vec<TeamConfig>,
    pub game_type: u32
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
/// Represents an authentication error
pub enum AuthFailure {
    InvalidAuthToken { message: Option<String> },
    InternalError { message: Option<String> },
    IPMismatch { message: String },
    Unknown,
    BadRequest { message: String },
    Timeout,
    ObjectNotFound
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WSRequest {
    Auth(AuthRequest),
    Server(ServerRequest)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Any response from manager (to either client OR server)
pub enum WSResponse {
    /// An error occurred processing auth
    Error(AppError),
    /// Client has started an oauth2 login session
    PendingLogin { url: String },
    /// A bad or malformed request was made
    InvalidRequest { message: Option<String> },
    /// Session was started successfully
    SessionStarted { sess_token: String, expires_at: u64 },

    ServerEvent(ServerEvent)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "error", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppError {
    AuthError(AuthFailure),
    SocketError { message: Option<String> },
    BadRequest { message: Option<String> },
    InternalServerError { message: String },
    EntityNotFound { message: String },
    MissingQueryParameter(String),
    DatabaseError { message: String }
}

impl Error for AuthFailure {}
impl fmt::Display for AuthFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthFailure::BadRequest { message} => write!(f, "{}", message),
            AuthFailure::InvalidAuthToken { message} => {
                if let Some(msg) = message {
                    write!(f, "{}", msg)
                } else {
                    write!(f, "auth token is either invalid or unauthorized")
                }
            },
            AuthFailure::ObjectNotFound => write!(f, "client or server being authorized does not exist"),
            _ => write!(f, "generic authentication failure")
        }
    }
}