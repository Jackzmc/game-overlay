use std::fmt::Display;
use std::net::SocketAddr;
use std::ops::Sub;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};
use egui::{pos2, vec2, Align2, Button, Color32, FontId, TextEdit, TextFormat, Theme, Widget};
use egui::ImageSource::Uri;
use egui::text::LayoutJob;
use egui_extras::install_image_loaders;
use egui_overlay::EguiOverlay;
use egui_overlay::egui_render_three_d::ThreeDBackend as DefaultGfxBackend;
use overlay_manager::ClientIncomingRequest;
use serde_json::{json, Value};
use tokio::sync::broadcast::Receiver;
pub(crate) use crate::defs::{PlayerInfo, PlayerTeam, ServerInfo};
use crate::defs::{TeamConfig, TeamShow};
use crate::manager::OverlayManager;
use crate::registry::Registry;
use crate::templates::list_player::TemplateListPlayers;
use crate::templates::{Element, Template};
use crate::templates::CoreTemplate::GenericText;
use crate::templates::generic::{TemplateGenericImage, TemplateGenericText};

struct PromptData {
    multiline: bool,
    title: String,
    //

    value: String
}

pub struct OverlayData {
    manager: OverlayManager,
    server: Option<ServerInfo>,
    initialized: bool,
    startup_message_remain: Option<Instant>,

    ui_state: UIState,

    read_rx: Receiver<ClientIncomingRequest>,

    elements: Vec<Element>,

    registry: Registry,

    prompt_state: Option<PromptData>
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
        let mut registry = Registry::new();
        registry.register("overlay:list_players", TemplateListPlayers {});
        registry.register("overlay:generic_text", TemplateGenericText);
        registry.register("overlay:generic_image", TemplateGenericImage);

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
                        steamid: "STEAM_1:0:49243767".to_string(),
                        health: 96.0,
                        user_id: 0,
                    },
                    PlayerInfo {
                        name: "Rochelle".to_string(),
                        connected: SystemTime::now().sub(Duration::from_secs(3600)),
                        team: PlayerTeam(2),
                        is_idle: false,
                        admin_perms: None,
                        steamid: "BOT".to_string(),
                        health: 25.0,
                        user_id: 0,
                    },
                    PlayerInfo {
                        name: "Player 2".to_string(),
                        connected: SystemTime::now(),
                        team: PlayerTeam(2),
                        is_idle: true,
                        admin_perms: Some("xz".to_string()),
                        steamid: "STEAM_#:#:######1".to_string(),
                        health: 36.0,
                        user_id: 0,
                    },
                    PlayerInfo {
                        name: "Hunter".to_string(),
                        connected: SystemTime::now(),
                        team: PlayerTeam(3),
                        is_idle: false,
                        admin_perms: None,
                        steamid: "BOT".to_string(),
                        health: 100.0,
                        user_id: 0,
                    }
                ],
                my_player: PlayerInfo {
                    name: "Jackzie".to_string(),
                    connected: SystemTime::now(),
                    team: PlayerTeam(2),
                    is_idle: false,
                    admin_perms: Some("z".to_string()),
                    steamid: "STEAM_1:0:49243767".to_string(),
                    health: 96.0,
                    user_id: 0,
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
                registry.try_temp("overlay:list_players", json!({
                    "actions": {
                       "STEAM_1:0:49243767": [
                            {
                                "label": "Kick Player",
                                "command": "sm_kick #23"
                            },
                                                        {
                                "label": "Slay Player",
                                "command": "sm_slay #23"
                            }
                        ]
                    }
                })),
                // registry.try_temp("overlay:list_players", json!({ "invalid": true })),
                // registry.try_temp("overlay:invalid_test", Value::Null),
                // registry.try_temp("overlay:generic_text", json!({ "content": "Hello"})),
                // registry.try_temp("overlay:generic_image", json!({ "url": "https://cdn.jackz.me/img/steve.jpg" }))

            ],
            prompt_state: Some(PromptData {
                multiline: false,
                title: "Enter name:".to_string(),
                value: "".to_string(),
            }),
            registry
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
            install_image_loaders(egui_context);
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

        if let Some(prompt) = &mut self.prompt_state {
            egui::Window::new(&prompt.title).id("prompt_text".into()).show(egui_context, |ui| {
                ui.style_mut().spacing.item_spacing = vec2(14.0, 14.0);
                ui.style_mut().spacing.button_padding = vec2(13.0, 7.0);
                ui.add_sized(vec2(ui.available_width(), 17.0),
                             TextEdit::singleline(&mut prompt.value)
                                 .desired_rows(1)
                                 .hint_text("Enter text here"));
                ui.horizontal(|ui| {
                    // TODO: somehow dynamically set up prompt _and_ feed it back to what needs it
                    Button::new("Submit").fill(Color32::LIGHT_BLUE).ui(ui);
                    ui.button("Cancel");
                })
            });
        }


        for elem in &mut self.elements {
            egui::Window::new(elem.template.id())
                .id(elem.id.to_string().into())
                .default_pos(pos2(1600.0, 400.0))
                .show(egui_context, |ui| {
                    ui.group(|ui| {
                        ui.label(format!("ID: {}", elem.id));
                        ui.label(format!("TemplateId: {}", elem.template.id()));
                        let pos = ui.next_widget_position();
                        ui.label(format!("Pos: ({}, {})", pos.x, pos.y));
                        ui.collapsing("State", |ui| {
                            ui.label(elem.state.to_string());

                        })
                    });
                    if let Err(err) = elem.template.is_state_valid(&elem.state) {
                        ui.colored_label(Color32::RED, format!("This element has been misconfigured.\nError: {}", err));
                    } else {
                        // No issues, render
                        elem.template.render(ui, self.server.as_ref().unwrap(), &mut elem.state);
                    }
                });
        }

        egui::Window::new("Dummy Player")
            .default_open(false)
            .default_pos(pos2(0.0, 0.0))
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

        let needs_controls = egui_context.wants_pointer_input() || egui_context.wants_keyboard_input();
        glfw_backend.set_passthrough(!needs_controls);

        // egui_context.request_repaint();
    }
}
