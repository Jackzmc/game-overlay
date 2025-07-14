use serde::{Serialize, Deserialize};
use crate::{ElementOptions, ElementState, SteamUser, TargetSelection};
use crate::game::TeamConfig;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being received from the client (Client -> Manager)
pub enum ClientRequest {


    /// Perform an action (command) on server
    Action { command: String, namespace: String, input: String, instance_id: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being received from server. (Server -> Manager)
pub enum ServerRequest {
    /*
    TODO: plan
        either:
         - [A] have manager be the one making decisions:
            - server says player A connected
            - manager then tells client you joined server, here's server info
            - basically: manager adds on data

            - can impl GameState, but the types of ServerInfo will be diff
         - [B] have server send full event
            - server sends ClientEvent::Connected with full server info, goes straight to client with full data
            - basically: server sends full data, manager just passes along


    */
    InitialServerInfo {
        hostname: String,
        teams: Vec<TeamConfig>
    },
    ServerInfo {
        hostname: Option<String>,
        // game: Option<GameInfo>
    },
    PlayerJoined { steamid: String },
    PlayerLeft { steamid: String },
    GameState {}, // TODO: implement
    /// Creates a new element for a client
    RequestElement { target: TargetSelection, elem_id: String, template_id: String, state: ElementState, options: Option<ElementOptions> },
    /// Updates an element by id, with optional new options (overwrites existing)
    UpdateElement { target: TargetSelection, elem_id: String, state: ElementState, new_options: Option<ElementOptions> },
    ChangeAudioState { steamids: Option<Vec<String>> , source: String, state: u8, volume: Option<f32>, start_time: Option<f32>, repeat: Option<bool> }
}

