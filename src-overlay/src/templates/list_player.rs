use std::collections::HashMap;
use egui::{Align, CollapsingHeader, Layout, Margin, RichText, Window};
use egui::ImageSource::Uri;
use egui::scroll_area::State;
use egui_overlay::egui_render_three_d::three_d::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::Value::Null;
use steamid_ng::SteamID;
use tracing::debug;
use tracing::log::warn;
use crate::defs::{ServerInfo, TeamShow};
use crate::templates::{ElementState, Template};

#[derive(Default)]
pub struct TemplateListPlayers;

impl Template for TemplateListPlayers {
    fn id(&self) -> &str { "overlay:list_players" }

    fn is_state_valid(&self, state: &ElementState) -> Result<(), String> {
        if state["actions"].is_null() { return Err("'actions' field is missing".to_string()) };
        Ok(())
    }

    fn render(&self, ui: &mut egui::Ui, server: &ServerInfo, state: &mut ElementState) {
        {
            let mut style = ui.style_mut();
            style.spacing.item_spacing = egui::vec2(16.0, 8.0);
            style.spacing.window_margin = Margin::from(4.0);
        }
        ui.label("4 Survivors, 1 Infected, 0 Spectators.");
        let server = server;
        for (team_id, team) in server.teams.iter().enumerate() {
            if team.show != TeamShow::Hidden {
                CollapsingHeader::new(format!("{} ({})", &team.name, -1))
                    .default_open(team.show == TeamShow::Open)
                    .show(ui, |ui| {
                        for player in server.players.iter().filter(|p| p.team.0 == team_id as u8) {
                            // TODO: Player 1 [IDLE/BOT] ---- STEAM_###
                            let id = ui.make_persistent_id(format!("player_info_{}", player.steamid));
                            // FIXME: header part not clickable, abandon or custom hack?
                            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                                .show_header(ui, |ui| {
                                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                        ui.style_mut().spacing.item_spacing = egui::vec2(4.0, 8.0);
                                        if player.steamid.starts_with("STEAM") {
                                            ui.image(Uri(format!("https://admin.jackz.me/api/users/{}/avatar", player.steamid).into()));
                                        }
                                        ui.strong(&player.label_name());
                                    });
                                    // ui.horizontal(|ui| {
                                    //     ui.image(Uri(format!("https://admin.jackz.me/api/users/{}/avatar", player.steamid).into()));
                                    //     ui.strong(&player.label_name());
                                    // });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new(&player.steamid).monospace());
                                    });
                                })
                                .body(|ui| {
                                    ui.style_mut().spacing.item_spacing = egui::vec2(16.0, 12.0);
                                    ui.columns(2, |cols| {
                                        cols[0].label("SteamID");
                                        if let Ok(steamid) = SteamID::from_steam2(&player.steamid) {
                                            cols[1].hyperlink_to(&player.steamid, format!("https://steamcommunity.com/profiles/{}", steamid.steam64()));
                                        } else {
                                            cols[1].label(&player.steamid);
                                        }

                                        cols[0].label("Team");
                                        cols[1].label(&team.name);

                                        cols[0].label("User Id");
                                        cols[1].label(format!("#{}", player.user_id));

                                        cols[0].label("Admin Permissions");
                                        cols[1].label(player.admin_perms.as_ref().map(|s| s.clone()).unwrap_or("-".to_string()));

                                        cols[0].label("Joined");
                                        cols[1].label(format!("{}s ago", player.connected.elapsed().unwrap().as_secs()));

                                    });
                                    if player.steamid.starts_with("STEAM") {
                                        ui.hyperlink_to("Open admin panel profile", format!("https://admin.jackz.me/admin/players/{}/overview/profile", player.steamid));
                                    }
                                    ui.add_space(8.0);
                                    ui.horizontal(|ui| {
                                        // TODO: button to open another element with overlay:player_details
                                        if let Some(actions) = state["actions"][&player.steamid].as_array(){
                                            for action in actions {
                                                let name = action["label"].as_str().unwrap_or("?");
                                                if ui.button(name).clicked() {
                                                    let cmd = action["command"].as_str();
                                                    debug!("pressed {} -> {:?}", name, cmd);
                                                }
                                            }
                                        }
                                    });
                                    // ui.add_space(10.0);
                                });
                        }
                    });
            }
        }
    }
}
