use serde::{Deserialize, Serialize};
use crate::{CreateElementRegister, SteamUser, UITemplate, UpdateElementRegister};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being sent to client (Client <- Manager)
pub enum ClientEvent {
    JoinedServer { server_id: String, server_name: String, server_ip: String },
    LeftServer,
    GameData {}, // TODO: implement
    Authorized { steamid2: String, auth_token: String, user: SteamUser },
    // Manual activation for UI side:
    // ManagerConnState(ManagerConnStatus),
    RegisterTempElement { elem_id: String, expires_seconds: Option<u64>, element: UITemplate },
    // Clients will fetch UI if received (with visibility=true)
    CreateElement {
        #[serde(flatten)]
        registry: CreateElementRegister
    },
    UpdateElement {
        #[serde(flatten)]
        registry: UpdateElementRegister
    },
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

