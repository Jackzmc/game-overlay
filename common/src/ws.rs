use std::error::Error;
use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
/// Requests from connections before authenticated
/// After authentication, it becomes ClientRequest or ServerRequest
pub enum AuthRequest {
    Client { auth_token: Option<String> },
    Server { auth_token: String }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "code")]
/// Represents an authentication error
pub enum AuthFailure {
    InvalidAuthToken { message: Option<String> },
    InternalError { message: Option<String> },
    Unknown,
    General { message: String },
    Timeout,
    ObjectNotFound
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Any response from manager (to either client OR server)
pub enum WSResponse {
    /// An error occurred processing auth
    Error { error: AuthFailure },
    /// Client has started an oauth2 login session
    PendingLogin { url: String },
    /// A bad or malformed request was made
    InvalidRequest { message: Option<String> }
}

impl Error for AuthFailure {}
impl fmt::Display for AuthFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthFailure::General { message} => write!(f, "{}", message),
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