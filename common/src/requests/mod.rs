use serde::{Serialize, Deserialize};
use crate::{CreateElementRegister, SteamUser, UITemplate, UpdateElementRegister};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being received from the client (Client -> Manager)
pub enum ClientRequest {
    Action { command: String, namespace: String, input: String, instance_id: String }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
/// Messages that are being received from server. (Server -> Manager)
pub enum ServerRequest {
    PlayerJoined { steamid: String },
    PlayerLeft { steamid: String },
    GameState {}, // TODO: implement
    RegisterTempUi { steamids: Option<Vec<String>>,  elem_id: String, expires_seconds: Option<u64>, element: UITemplate },
    CreateElement { steamids: Option<Vec<String>>,
        #[serde(flatten)]
        registry: CreateElementRegister
    },
    UpdateElement { steamids: Option<Vec<String>>,
        #[serde(flatten)]
        registry: UpdateElementRegister
    },
    ChangeAudioState { steamids: Option<Vec<String>> , source: String, state: u8, volume: Option<f32>, start_time: Option<f32>, repeat: Option<bool> }

}