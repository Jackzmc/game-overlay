use serde::{Deserialize, Serialize};
use crate::{ElementOptions, ElementState, SteamUser};
use crate::game::ServerInfo;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being sent to client (Client <- Manager)
pub enum ClientEvent {
    // ServerChanged { server: Option<ServerInfo> },
    /// The server client is connected to has changed (either connected, switched, or disconnected)
    ChangedServer(Option<ServerInfo>),
    #[deprecated]
    JoinedServer { server_id: String, server_name: String, server_ip: String },
    #[deprecated]
    LeftServer,
    GameData {}, // TODO: implement
    Authorized { steamid2: String, auth_token: String, user: SteamUser },
    // Manual activation for UI side:
    // ManagerConnState(ManagerConnStatus),
    /// Server is requesting a new element to be displayed
    RequestElement { elem_id: String, template_id: String, state: ElementState, options: Option<ElementOptions> },
    /// Server is updating an existing element
    UpdateElement { elem_id: String, state: ElementState, new_options: Option<ElementOptions> },

    // RequestTempElement { template_id: String, state: ElementState, options: Option<ElementOptions> },
    ChangeAudioState { source: String, state: u8, volume: Option<f32>, start_time: Option<f32>, repeat: Option<bool> }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being sent to server (Server <- Manager)
pub enum ServerEvent {
    Authorized,
    ManagerDisconnected,
    Action { steamid2: String, command: String, namespace: String, input: String, instance_id: String }
}

