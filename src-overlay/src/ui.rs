use strum_macros::EnumString;
use std::fmt::Display;
use std::net::SocketAddr;
use std::ops::Sub;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};
use egui::{Align, Align2, CollapsingHeader, Color32, DragValue, FontFamily, FontId, Label, Layout, Margin, RichText, TextFormat, Theme, Widget};
use egui::text::LayoutJob;
use egui::UiKind::Popup;
use egui::X11WindowType::PopupMenu;
use egui_extras::Column;
use egui_overlay::EguiOverlay;
use egui_overlay::egui_render_three_d::ThreeDBackend as DefaultGfxBackend;
use overlay_manager::{ClientIncomingRequest, ManagerConnState};
use serde_json::Value;
use tokio::sync::broadcast::Receiver;
use tracing::debug;
use tracing::field::debug;
use tracing::log::trace;
use uuid::{uuid, Uuid};
use crate::manager::{OverlayManager, OverlayManagerInstance};

struct ServerInfo {
    name: String,
    ip_addr: SocketAddr,
    connected_time: Instant,

    players: Vec<PlayerInfo>,
    my_player: PlayerInfo,

    teams: Vec<TeamConfig>
}
impl ServerInfo {
    pub fn get_team_config(&self, team: &PlayerTeam) -> Option<&TeamConfig> {
        self.teams.get(team.0 as usize)
    }
    pub fn get_team_config_name(&self, team: &PlayerTeam) -> Option<&str> {
        self.teams.get(team.0 as usize).map(|s| s.name.as_str())
    }
}

// #[derive(FromRepr, Debug, PartialEq, Clone)]
// #[repr(u8)]
// enum Team {
//     Unassigned = 0,
//     Spectator = 1,
//     Survivor,
//     Infected,
//     SurvivorBot
// }

/// Represents the team the player is in
pub struct PlayerTeam(u8);
#[derive(PartialEq)]
enum TeamShow {
    /// Team is hidden in the UI
    Hidden,
    /// Team is shown but collapsed
    Collapsed,
    /// Team is shown and is expanded
    Open
}
/// Defines a team, with its ID and number
struct TeamConfig { name: String, show: TeamShow }

/// Represents a team for display purposes
struct TeamCategory {
    /// The displayed label for team
    label: String,
    /// If set, the team ID this entry is for
    /// If not set, the first one found is a catch-all
    team_id: Option<u8>
}
// impl Display for Team {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Team::Unassigned => write!(f, "Unassigned"),
//             Team::Spectator => write!(f, "Spectator"),
//             Team::Survivor => write!(f, "Survivor"),
//             Team::Infected => write!(f, "Infected"),
//             _ => write!(f, "Unknown"),
//         }
//     }
// }

struct PlayerInfo {
    steamid: String,
    name: String,
    connected: SystemTime,
    team: PlayerTeam,
    is_idle: bool,
    admin_perms: Option<String>,
    health: f32
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

#[derive(Debug, PartialEq, EnumString)]
enum TemplateNamespace {
    #[strum(serialize = "overlay")]
    Core(CoreTemplate),
    Other(String, String)
}
#[derive(Debug, PartialEq, EnumString, Default, strum_macros::Display)]
enum CoreTemplate {
    #[strum(serialize = "invalid")]
    #[default]
    Invalid,

    #[strum(serialize = "list_players")]
    ListPlayers,
    #[strum(serialize = "generic_text")]
    GenericText,
    #[strum(serialize = "generic_image")]
    GenericImage,
    #[strum(serialize = "motd")]
    MOTD,
}
enum TemplateId {
    Core(CoreTemplate),
    Other(String, String)
}
impl TemplateId {
    pub fn custom(namespace: &str, id: &str) -> Self {
        Self::Other(namespace.to_string(), id.to_string())
    }

    pub fn core(core_template: CoreTemplate) -> Self {
        Self::Core(core_template)
    }
}

impl Display for TemplateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            TemplateId::Core(part) => format!("core:{}", part.to_string()),
            TemplateId::Other(ns, part) => format!("{}:{}", ns, part)
        };
        write!(f, "{}", str)
    }
}

struct Element {
    id: String,
    template_id: TemplateId,
    variables: Value
}

impl Element {
    pub fn temp(template_id: TemplateId, variables: Value) -> Self {
        Self::with_id(Uuid::new_v4().to_string(), template_id, variables)
    }

    pub fn with_id(id: String, template_id: TemplateId, variables: Value) -> Self {
        Self {
            id,
            template_id,
            variables,
        }
    }
}

pub struct OverlayData {
    manager: OverlayManager,
    server: Option<ServerInfo>,
    initialized: bool,
    startup_message_remain: Option<Instant>,

    ui_state: UIState,

    read_rx: Receiver<ClientIncomingRequest>,

    elements: Vec<Element>
}

enum UIState {
    /// Game not active, UI not running
    Inactive,
    /// Game active, UI is shown
    Active,
    /// Detail view active, extra UI is shown
    DetailActive
}

impl OverlayData {
    pub fn example(manager: OverlayManager, rx: Receiver<ClientIncomingRequest>) -> Self {
        OverlayData {
            manager,
            server: Some(ServerInfo {
                name: "My Server".to_string(),
                ip_addr: SocketAddr::from_str("127.0.0.1:27015").unwrap(),
                connected_time: Instant::now(),

                players: vec![
                    PlayerInfo {
                        name: "Jackzie".to_string(),
                        connected: SystemTime::now(),
                        team: PlayerTeam(2),
                        is_idle: false,
                        admin_perms: Some("z".to_string()),
                        steamid: "STEAM_#:#:######".to_string(),
                        health: 96.0,
                    },
                    PlayerInfo {
                        name: "Rochelle".to_string(),
                        connected: SystemTime::now().sub(Duration::from_secs(3600)),
                        team: PlayerTeam(2),
                        is_idle: false,
                        admin_perms: None,
                        steamid: "BOT".to_string(),
                        health: 25.0,
                    },
                    PlayerInfo {
                        name: "Player 2".to_string(),
                        connected: SystemTime::now(),
                        team: PlayerTeam(2),
                        is_idle: true,
                        admin_perms: Some("xz".to_string()),
                        steamid: "STEAM_#:#:######1".to_string(),
                        health: 36.0,
                    },
                    PlayerInfo {
                        name: "Hunter".to_string(),
                        connected: SystemTime::now(),
                        team: PlayerTeam(3),
                        is_idle: false,
                        admin_perms: None,
                        steamid: "BOT".to_string(),
                        health: 100.0,
                    }
                ],
                my_player: PlayerInfo {
                    name: "Jackzie".to_string(),
                    connected: SystemTime::now(),
                    team: PlayerTeam(2),
                    is_idle: false,
                    admin_perms: Some("z".to_string()),
                    steamid: "STEAM_#:#:######2".to_string(),
                    health: 96.0,
                },
                // this should in future be pulled from server
                teams: vec![
                    TeamConfig { name: "Unassigned".to_string(), show: TeamShow::Hidden  },    // 0
                    TeamConfig { name: "Spectators".to_string(), show: TeamShow::Collapsed   },   // 1
                    TeamConfig { name: "Survivors".to_string(), show: TeamShow::Open  },       // 2
                    TeamConfig { name: "Infected".to_string(), show: TeamShow::Collapsed  },   // 3
                    TeamConfig { name: "Holdout Bots".to_string(), show: TeamShow::Hidden   }, // 4
                ],
            }),
            initialized: false,
            startup_message_remain: Some(Instant::now()),
            ui_state: UIState::Inactive,
            read_rx: rx,
            elements: vec![
                Element::temp(TemplateId::Core(CoreTemplate::ListPlayers), Value::default()),
                Element::temp(TemplateId::Core(CoreTemplate::GenericText), Value::default())
            ]
        }
    }
}


impl EguiOverlay for OverlayData {
    fn gui_run(
        &mut self,
        egui_context: &egui::Context,
        _default_gfx_backend: &mut DefaultGfxBackend,
        glfw_backend: &mut egui_window_glfw_passthrough::GlfwBackend,
    ) {
        egui_context.set_theme(Theme::Light);
        if !self.initialized {
            self.initialized = true;
            glfw_backend.set_window_size([2560.0, 1440.0]);
        }
        /// Process incoming payloads and pass to manager
        /* TODO:
            read thread sends to manager directly, manager has its own queue
            manager has conn_state(&self) -> ConnState

            manager needs two pieces:
            - one that holds state, this is Clone or Arc<RwLock>, doesn't need to be locked
            - one that does all WS communication, read thread and sending locks
        */

        /* TODO:
            - all templates are static, as needs rust code
            - templates have all changes from variables
            - core plugin has some core templates like "core:list_players" with defaults
            - add some additional ones that are just like picture, or text, or command input
            - some API for custom plugins to add entries to some core templates
                - ListPlayers_AddCustomAction(char[] name, char[] command, filters?...)

         */

        if let Some(startup_msg_time) = self.startup_message_remain {
            egui::Window::new("welcome_message")
                // .default_pos(egui::pos2(200.0, 400.0))
                .anchor(Align2::RIGHT_TOP, egui::vec2(-15.0, 15.0))
                .title_bar(false)
                .fade_in(true)
                .fade_out(true)
                .collapsible(false)
                .resizable(false)
                .show(egui_context, |ui| {
                    let mut job = LayoutJob::default();
                    job.append(
                        "Overlay active",
                        0.0,
                        TextFormat::simple(FontId::default(), Color32::BLACK)
                    );
                    ui.label(job);
                    ui.label("Connect to a supported server to use the overlay");

                    ui.add_space(14.0);
                    ui.label("Open the overlay with CTRL + HOME");
                });
            // End the message
            if startup_msg_time.elapsed().as_secs() > 8 {
                self.startup_message_remain = None;
            }
        }

        for elem in &self.elements {
            egui::Window::new(elem.template_id.to_string())
                .id(elem.id.to_string().into())
                .show(egui_context, |ui| {
                    ui.label(format!("ID: {}", elem.id));
                    ui.label(format!("TemplateId: {}", elem.template_id));
                    ui.label(format!("Variables: {}", elem.variables.to_string()));

                });
        }

        // just some controls to show how you can use glfw_backend
        egui::Window::new("controls").show(egui_context, |ui| {
            ui.set_width(300.0);
            // sometimes, you want to see the borders to understand where the overlay is.
            let mut borders = glfw_backend.window.is_decorated();
            if ui.checkbox(&mut borders, "window borders").changed() {
                glfw_backend.window.set_decorated(borders);
            }

            ui.label(format!(
                "pixels_per_virtual_unit: {}",
                glfw_backend.physical_pixels_per_virtual_unit
            ));
            ui.label(format!("window scale: {}", glfw_backend.scale));
            ui.label(format!("cursor pos x: {}", glfw_backend.cursor_pos[0]));
            ui.label(format!("cursor pos y: {}", glfw_backend.cursor_pos[1]));

            ui.label(format!(
                "passthrough: {}",
                glfw_backend.window.is_mouse_passthrough()
            ));
            // how to change size.
            // WARNING: don't use drag value, because window size changing while dragging ui messes things up.
            let mut size = glfw_backend.window_size_logical;
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("width: ");
                ui.add_enabled(false, DragValue::new(&mut size[0]));
                if ui.button("inc").clicked() {
                    size[0] += 10.0;
                    changed = true;
                }
                if ui.button("dec").clicked() {
                    size[0] -= 10.0;
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("height: ");
                ui.add_enabled(false, DragValue::new(&mut size[1]));
                if ui.button("inc").clicked() {
                    size[1] += 10.0;
                    changed = true;
                }
                if ui.button("dec").clicked() {
                    size[1] -= 10.0;
                    changed = true;
                }
            });
            if changed {
                glfw_backend.set_window_size(size);
            }
            // how to change size.
            // WARNING: don't use drag value, because window size changing while dragging ui messes things up.
            let mut pos = glfw_backend.window_position;
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("x: ");
                ui.add_enabled(false, DragValue::new(&mut pos[0]));
                if ui.button("inc").clicked() {
                    pos[0] += 10;
                    changed = true;
                }
                if ui.button("dec").clicked() {
                    pos[0] -= 10;
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("y: ");
                ui.add_enabled(false, DragValue::new(&mut pos[1]));
                if ui.button("inc").clicked() {
                    pos[1] += 10;
                    changed = true;
                }
                if ui.button("dec").clicked() {
                    pos[1] -= 10;
                    changed = true;
                }
            });
            if changed {
                glfw_backend.window.set_pos(pos[0], pos[1]);
            }
        });

        egui::Window::new("Dummy Player")
            .show(egui_context, |ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(16.0, 12.0);
                ui.columns(2, |cols| {
                    cols[0].label("SteamID");
                    cols[1].label("STEAM_#:#:#####");

                    cols[0].label("Team");
                    cols[1].label("Survivors");

                    cols[0].label("Admin Permissions");
                    cols[1].label("None");

                    cols[0].label("Joined");
                    cols[1].label("5 minutes ago");
                });
            });

        egui::Window::new("Manage Players 2")
            .default_pos(egui::pos2(500.0, 400.0))
            .show(egui_context, |ui| {
                {
                    let mut style = ui.style_mut();
                    style.spacing.item_spacing = egui::vec2(16.0, 8.0);
                    style.spacing.window_margin = Margin::from(4.0);
                }
                struct Dummy {
                    name: String,
                    steamid: String,
                    team: String,
                    other: String
                }
                ui.label("4 Survivors, 1 Infected, 0 Spectators.");
                let players = vec![Dummy {
                    name: "Jackzie :heart:".to_string(),
                    steamid: "STEAM_#:#:#####".to_string(),
                    team: "Survivor".to_string(),
                    other: "".to_string(),
                }, Dummy {
                    name: "Player 2".to_string(),
                    steamid: "STEAM_#:#:#####".to_string(),
                    team: "Survivor [IDLE]".to_string(),
                    other: "".to_string(),
                }, Dummy {
                    name: "Rochelle".to_string(),
                    steamid: "BOT".to_string(),
                    team: "Survivor".to_string(),
                    other: "".to_string(),
                }];
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    egui_extras::TableBuilder::new(ui)
                        .column(Column::auto())
                        .column(Column::exact(120.0))
                        .column(Column::auto())
                        .column(Column::auto())
                        .header(20.0, |mut header| {
                            header.col(|col| {
                                col.strong("Name");
                            });
                            header.col(|col| {
                               col.strong("SteamID");
                            });
                            header.col(|col| {
                                col.strong("Team");
                            });
                            header.col(|col| {
                                col.strong("?");
                            });
                        })
                        .body(|mut body| {
                            let server = self.server.as_ref().unwrap();
                            for player in &server.players {
                                body.row(20.0, |mut row| {
                                    row.col(|col| {
                                        col.label(&player.name);
                                    });
                                    row.col(|col| {
                                        col.label(&player.steamid);
                                    });
                                    row.col(|col| {
                                        col.label(server.get_team_config_name(&player.team).unwrap_or("unknown"));
                                    });
                                    row.col(|col| {
                                        // col.label(player.other);
                                        if col.button("View").clicked() {
                                            debug!("open window");
                                            // need to add to state
                                        }
                                    });
                                })
                            }
                        })

                });
            });


        egui::Window::new("Manage Players")
            .default_pos(egui::pos2(1000.0, 300.0))
            .default_width(200.0)
            .max_width(2560.0)
            .show(egui_context, |ui| {
                {
                    let mut style = ui.style_mut();
                    style.spacing.item_spacing = egui::vec2(16.0, 8.0);
                    style.spacing.window_margin = Margin::from(4.0);
                }
                ui.label("4 Survivors, 1 Infected, 0 Spectators.");
                let server = self.server.as_ref().unwrap();
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

                                                cols[0].label("Admin Permissions");
                                                cols[1].label(player.admin_perms.as_ref().map(|s| s.clone()).unwrap_or("-".to_string()));

                                                cols[0].label("Joined");
                                                cols[1].label(format!("{}s ago", player.connected.elapsed().unwrap().as_secs()));
                                            });
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
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

                // CollapsingHeader::new("Spectators / Unassigned")
                //     .default_open(false)
                //     .show(ui, |ui| {
                //         for player in &self.server.as_ref().unwrap().players {
                //             // TODO: Player 1 [IDLE/BOT] ---- STEAM_###
                //             let id = ui.make_persistent_id(format!("player_info_{}", player.steamid));
                //             egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                //                 .show_header(ui, |ui| {
                //                     ui.horizontal(|ui| {
                //                         ui.label(&player.name);
                //                         ui.label(&player.steamid);
                //                         ui.label(player.team.to_string());
                //                     })
                //                 })
                //                 .body(|ui| ui.label("Body"));
                //         }
                //     });
                // CollapsingHeader::new("Survivors")
                //     .default_open(true)
                //     .show(ui, |ui| {
                //         for player in &self.server.as_ref().unwrap().players {
                //             // TODO: Player 1 [IDLE/BOT] ---- STEAM_###
                //             let id = ui.make_persistent_id(format!("player_info_{}", player.steamid));
                //             egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                //                 .show_header(ui, |ui| {
                //                     ui.horizontal(|ui| {
                //                         ui.label(&player.name);
                //                         ui.label(&player.steamid);
                //                         ui.label(player.team.to_string());
                //                     })
                //                 })
                //                 .body(|ui| ui.label("Body"));
                //         }
                //     });
                // CollapsingHeader::new("Infected")
                //     .default_open(false)
                //     .show(ui, |ui| {
                //         for player in &self.server.as_ref().unwrap().players {
                //             // TODO: Player 1 [IDLE/BOT] ---- STEAM_###
                //             let id = ui.make_persistent_id(format!("player_info_{}", player.steamid));
                //             egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                //                 .show_header(ui, |ui| {
                //                     ui.horizontal(|ui| {
                //                         ui.label(&player.name);
                //                         ui.label(&player.steamid);
                //                         ui.label(player.team.to_string());
                //                     })
                //                 })
                //                 .body(|ui| ui.label("Body"));
                //         }
                //     });
                //
                //
                // for player in &self.server.as_ref().unwrap().players {
                //     ui.collapsing(player.label_name(), |ui| {
                //         ui.style_mut().spacing.item_spacing = egui::vec2(16.0, 12.0);
                //         ui.columns(2, |cols| {
                //             cols[0].label("SteamID");
                //             cols[1].label(&player.steamid);
                //
                //             cols[0].label("Team");
                //             cols[1].label(player.team.to_string());
                //
                //             cols[0].label("Admin Permissions");
                //             cols[1].label(player.admin_perms.as_ref().map(|s| s.clone()).unwrap_or("-".to_string()));
                //
                //             cols[0].label("Joined");
                //             cols[1].label(format!("{}s ago", player.connected.elapsed().unwrap().as_secs()));
                //         });
                //         ui.add_space(8.0);
                //         ui.horizontal(|ui| {
                //             ui.button("Kick Player");
                //             ui.button("Ban Player");
                //             let response = ui.button("Perform Action");
                //             // Popup::menu(&response)
                //             //     .gap(4).align(Align2::LEFT_CENTER)
                //             //     .show(|ui| { /* menu contents */ });
                //         });
                //         // ui.add_space(10.0);
                //     });
                // }
            });


        let needs_controls = egui_context.wants_pointer_input() || egui_context.wants_keyboard_input();
        glfw_backend.set_passthrough(!needs_controls);

        // egui_context.request_repaint();
    }
}
