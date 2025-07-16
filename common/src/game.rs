use std::net::{IpAddr, SocketAddr};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
/// The index of a player's team
pub struct PlayerTeam(pub u8);
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
/// Determine how team should be shown in UI
pub enum TeamShow {
    /// Team is hidden in the UI
    Hidden,
    /// Team is shown but collapsed
    Collapsed,
    /// Team is shown and is expanded
    Open
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeamConfig {
    /// The displayed name of team
    pub name: String,
    pub show: TeamShow
}
impl ServerInfo {
    pub fn get_team_config(&self, team: &PlayerTeam) -> Option<&TeamConfig> {
        self.teams.get(team.0 as usize)
    }
    pub fn get_team_config_name(&self, team: &PlayerTeam) -> Option<&str> {
        self.teams.get(team.0 as usize).map(|s| s.name.as_str())
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInfo {
    pub steamid: String,
    pub user_id: u32,
    pub name: String,

    pub team: PlayerTeam,
    pub connected_at: SystemTime,
    pub is_idle: bool,
    pub admin_perms: Option<String>,
    pub health: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub ip_addr: IpAddr,
    /// When client has been connected to server? or manager idk ill figure it out
    // pub connected_at: std::time::SystemTime,
    pub game_type: usize, // appid for now

    pub players: Vec<PlayerInfo>,

    pub teams: Vec<TeamConfig>
}


