use std::net::SocketAddr;
use std::time::{Instant, SystemTime};

pub struct ServerInfo {
    pub name: String,
    pub ip_addr: SocketAddr,
    pub connected_time: Instant,

    pub players: Vec<crate::ui::PlayerInfo>,
    pub my_player: crate::ui::PlayerInfo,

    pub teams: Vec<TeamConfig>
}


/// Represents the team the player is in
pub struct PlayerTeam(pub(crate) u8);
#[derive(PartialEq)]
pub enum TeamShow {
    /// Team is hidden in the UI
    Hidden,
    /// Team is shown but collapsed
    Collapsed,
    /// Team is shown and is expanded
    Open
}
/// Defines a team, with its ID and number
pub(crate) struct TeamConfig { pub name: String, pub show: TeamShow }
impl ServerInfo {
    pub fn get_team_config(&self, team: &PlayerTeam) -> Option<&TeamConfig> {
        self.teams.get(team.0 as usize)
    }
    pub fn get_team_config_name(&self, team: &PlayerTeam) -> Option<&str> {
        self.teams.get(team.0 as usize).map(|s| s.name.as_str())
    }
}


pub struct PlayerInfo {
    pub steamid: String,
    pub name: String,
    pub connected: SystemTime,
    pub team: PlayerTeam,
    pub is_idle: bool,
    pub admin_perms: Option<String>,
    pub health: f32,
    pub user_id: u32
}



impl PlayerInfo {
    pub fn label_name(&self) -> String {
        if self.is_idle {
            format!("{} [IDLE]", self.name)
        } else if self.steamid == "BOT" {
            format!("{} [BOT]", self.name)
        } else {
            self.name.to_string()
        }
    }
}
