use std::collections::HashMap;
use egui::{Align, CollapsingHeader, Layout, Margin, RichText, Window};
use egui::scroll_area::State;
use egui_overlay::egui_render_three_d::three_d::Context;
use serde_json::Value;
use crate::defs::{ServerInfo, TeamShow};
use crate::templates::Template;
// struct Element {
//     id: String,
//     template_id: TemplateId,
//     variables: Value
// }




#[derive(Default)]
pub struct Template_ListPlayers;

impl Template for Template_ListPlayers {
    fn id(&self) -> &str { "overlay:list_players" }
    fn render(&self, ui: &mut egui::Ui, server: &ServerInfo, state: &mut Value) {
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
                                    ui.label(&player.label_name());
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new(&player.steamid).monospace());
                                    });
                                })
                                .body(|ui| {
                                    ui.style_mut().spacing.item_spacing = egui::vec2(16.0, 12.0);
                                    ui.columns(2, |cols| {
                                        cols[0].label("SteamID");
                                        cols[1].label(&player.steamid);

                                        cols[0].label("Team");
                                        cols[1].label(&team.name);

                                        cols[0].label("User Id");
                                        cols[1].label(format!("#{}", player.user_id));

                                        cols[0].label("Admin Permissions");
                                        cols[1].label(player.admin_perms.as_ref().map(|s| s.clone()).unwrap_or("-".to_string()));

                                        cols[0].label("Joined");
                                        cols[1].label(format!("{}s ago", player.connected.elapsed().unwrap().as_secs()));
                                    });
                                    ui.add_space(8.0);
                                    ui.horizontal(|ui| {
                                        // TODO: pull from elem
                                        ui.button("Kick Player");
                                        ui.button("Ban Player");
                                        let response = ui.button("Perform Action");
                                        // Popup::menu(&response)
                                        //     .gap(4).align(Align2::LEFT_CENTER)
                                        //     .show(|ui| { /* menu contents */ });
                                    });
                                    // ui.add_space(10.0);
                                });
                        }
                    });
            }
        }
    }
}
