use std::cmp::PartialEq;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{read_to_string, File};
use std::io::{ErrorKind, PipeReader, Write};
use std::net::SocketAddr;
use std::ops::Sub;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};
use directories_next::ProjectDirs;
use egui::{pos2, vec2, Align, Align2, Button, Color32, FontId, Frame, Id, InputState, Key, KeyboardShortcut, Layout, Margin, MenuBar, Modal, Modifiers, Order, Response, ScrollArea, Stroke, TextEdit, TextFormat, Theme, Ui, Widget, Window};
use egui::ImageSource::Uri;
use egui::text::LayoutJob;
use egui_extras::{install_image_loaders, Column, TableBuilder};
use egui_overlay::EguiOverlay;
use egui_overlay::egui_render_three_d::ThreeDBackend as DefaultGfxBackend;
use overlay_common::events::ClientEvent;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast::Receiver;
use tracing::{debug, error, info};
use tracing::log::trace;
pub(crate) use crate::defs::{PlayerInfo, PlayerTeam, ServerInfo};
use crate::defs::{TeamConfig, TeamShow};
use crate::manager::{SocketClient, SocketMessage};
use crate::registry::Registry;
use crate::templates::list_player::TemplateListPlayers;
use crate::templates::{Element, ElementState, Template};
use crate::templates::CoreTemplate::GenericText;
use crate::templates::generic::{TemplateGenericImage, TemplateGenericText};

struct PromptData {
    multiline: bool,
    title: String,
    //

    value: String
}

pub struct OverlayData {
    manager: SocketClient,
    server: Option<ServerInfo>,
    initialized: bool,
    startup_message_remain: Option<Instant>,

    /// Determines visibility of the UI
    ui_state: UIState,

    socket_rx: Receiver<SocketMessage>,

    /// Contains list of all active shown elements. Key is element ID
    elements: HashMap<String, Element>,
    /// Contains list of all template ids
    registry: Registry,

    prompt_state: Option<PromptData>,

    shortcuts: ShortcutContainer,

    startup_msg: StartupMessage,
    /// Struct that is saved and loaded on startup
    store: OverlayStorage,
    /// list of notification messages
    messages: Vec<Message>,
    reader: std::sync::mpsc::Receiver<Vec<u8>>,
    pub target_pid: u32,
}

enum MessageType {
    Normal,
    Info,
    Success,
    Error,
    Warning
}
struct Message {
    created_at: Instant,
    _type: MessageType,
    title: Option<String>,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApproveEntry {
    /// Is this element enabled?
    enabled: bool,

    /// The template ID the requested elem has. Must match for requests
    template_id: String,

    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    /// The state to use for a pending approval
    state: Option<ElementState>
}
// TODO: need list of addon approval, when pending needs in memory the state
#[derive(Debug, Serialize, Deserialize)]
/// Holds all data that gets saved between restarts
pub struct OverlayStorage {
    /// Maps server ips to a list of namespace:elem_id
    approved_elems_ids: HashMap<SocketAddr, HashMap<String, ApproveEntry>> // TODO: replace SocketAddr with uuid of server from manager?
}
impl Default for OverlayStorage {
    fn default() -> Self {
        Self {
            approved_elems_ids: HashMap::new()
        }
    }
}
impl OverlayStorage {
    pub fn save(&mut self) -> Result<(), String> {
        let paths = ProjectDirs::from("me.jackz", "jackzmc", "gameoverlay")
            .ok_or("Could not find save location".to_string())?;
        let data_dir = paths.data_dir();
        let file_path = data_dir.join("state.json");
        info!("Save to {:?}", file_path);
        std::fs::create_dir_all(data_dir).map_err(|err| err.to_string())?;
        let mut file = File::create(file_path).map_err(|err| err.to_string())?;
        file.write_all(serde_json::to_string_pretty(self).unwrap().as_bytes()).map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn load() -> Result<OverlayStorage, String> {
        let paths = ProjectDirs::from("me.jackz", "jackzmc", "gameoverlay")
            .ok_or("Could not find save location".to_string())?;
        let save_path = paths.data_dir().join("state.json");
        info!("Load from {:?}", save_path);
        match read_to_string(save_path) {
            Ok(content) => {
                serde_json::from_str(&content).map_err(|err| err.to_string())
            },
            Err(e) => {
                // File doesn't exist, give a default state one
                if e.kind() == ErrorKind::NotFound {
                    return Ok(OverlayStorage::default())
                }
                Err(e.to_string())
            }
        }
    }

    /// Returns list of approved elem for server, or creates a new list if none
    pub fn approved_elems(&mut self, ip_addr: SocketAddr) -> &mut HashMap<String, ApproveEntry> {
        self.approved_elems_ids.entry(ip_addr).or_insert(HashMap::new())
    }
}

#[derive(PartialEq)]
enum UIState {
    /// Game not active, UI not running
    Inactive,
    /// Game active, UI is shown
    Active,
    /// Detail view active, extra UI is shown
    DetailActive
}
impl OverlayData {
    // server sends element, we get entry to see if approved
    // if approved in past, spawn it
    pub fn request_elem(&mut self, template_id: &str, id: &str, state: ElementState) -> Result<bool, String> {
        debug!("request_elem template={} id={}", template_id, id);
        if !self.registry.has(template_id) {
            return Err("Template not registered".to_string())
        }
        if self.server.is_none() {
            return Err("Not connected to any server".to_string())
        }
        let server = self.server.as_ref().unwrap();
        let entry = self.store.approved_elems(server.ip_addr.clone()).entry(id.to_string())
            .or_insert_with(|| ApproveEntry {
                enabled: false,
                template_id: template_id.to_string(),
                // We store the state so if it is approved later, it can be pulled and have the correct state
                state: Some(state),
            });
        if &entry.template_id != template_id {
            // TODO: just drop prev entry and require re-approval
            return Err(format!("Requested template ({}) does not match registered template ({})", template_id, entry.template_id));
        }
        // Finally, if entry is already enabled (from previous times), then we can spawn it right away
        let enabled = entry.enabled;
        if enabled {
            let state = entry.state.take();
            self.spawn_elem(template_id, id.to_string(), state)
        }
        Ok(enabled)
    }

    /// Spawns an element
    pub fn spawn_elem(&mut self, template_id: &str, id: String, state: Option<ElementState>) {
        let state = state.unwrap_or(json!({}));
        // TODO: check element doesn't exist? , instead of vec, hashmap?
        self.elements.insert(id.to_string(), self.registry.named(template_id, id.to_string(), state).unwrap());
        debug!("spawn_elem template={} id={}", template_id, id);
    }

    pub fn server(&self) -> Option<&ServerInfo> {
        self.server.as_ref()
    }
    pub fn server_id(&self) -> Option<SocketAddr> {
        self.server.as_ref().map(|server| server.ip_addr.clone())
    }
    pub fn notify(&mut self, _type: MessageType, title: Option<String>, content: String) {
        let msg = Message { created_at: Instant::now(), _type, title: title, content: content.into() };
        self.messages.push(msg);
    }
    pub fn example(manager: SocketClient, manager_rx: Receiver<SocketMessage>, reader: std::sync::mpsc::Receiver<Vec<u8>>, target_pid: u32) -> Self {
        let mut registry = Registry::new();
        registry.register("overlay:list_players", TemplateListPlayers {});
        registry.register("overlay:generic_text", TemplateGenericText);
        registry.register("overlay:generic_image", TemplateGenericImage);


        registry.named("template:blah", "my_elem", Default::default());

        let mut s = OverlayData {
            target_pid: target_pid,
            reader,
            store: OverlayStorage::load().unwrap(),
            manager,
            messages: vec![],
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
            socket_rx: manager_rx,
            elements: HashMap::new(),
            prompt_state: Some(PromptData {
                multiline: false,
                title: "Enter name:".to_string(),
                value: "".to_string(),
            }),
            startup_msg: StartupMessage::new(),
            shortcuts: ShortcutContainer {
                toggle: KeyboardShortcut::new(Modifiers::CTRL, Key::Home)
            },
            registry
        };
        s.request_elem("overlay:list_players", "list_players_test",
       json!({
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
        })).unwrap();
        s
    }

    pub fn toggle_ui_state(&mut self) {
        debug!("CTRL+HOME pressed, switching state");
        if self.ui_state == UIState::DetailActive {
            self.ui_state = UIState::Active;
            trace!("state set to normal active");
        } else {
            self.ui_state = UIState::DetailActive;
            trace!("state set to detail active");
        }
    }
}

struct ShortcutContainer {
    toggle: KeyboardShortcut
}

struct StartupMessage {
    remaining_time: Option<Instant>
}

trait UIElement {
    fn run_ui(&mut self, ctx: &egui::Context);
}
impl StartupMessage {
    pub fn new() -> Self { Self { remaining_time: Some(Instant::now()) } }
}
impl UIElement for StartupMessage {
    fn run_ui(&mut self, ctx: &egui::Context) {
        if let Some(startup_msg_time) = self.remaining_time {
            Window::new("welcome_message")
                // .default_pos(egui::pos2(200.0, 400.0))
                .anchor(Align2::RIGHT_TOP, egui::vec2(-15.0, 15.0))
                .title_bar(false)
                .fade_in(true)
                .fade_out(true)
                .collapsible(false)
                .resizable(false)
                .order(Order::Foreground)
                .show(ctx, |ui| {
                    let mut job = LayoutJob::default();
                    job.append(
                        "Overlay active",
                        0.0,
                        TextFormat::simple(FontId::default(), Color32::BLACK)
                    );
                    ui.label(job);
                    ui.label("Connect to a supported server to use the overlay");

                    ui.add_space(14.0);
                    let mut job = LayoutJob::default();
                    job.append("Open the overlay with ", 0.0, TextFormat::simple(FontId::default(), Color32::BLACK));
                    job.append("CTRL + HOME", 0.0, TextFormat::simple(FontId::monospace(12.0), Color32::BLACK));
                    ui.label(job);
                });
            // End the message
            if startup_msg_time.elapsed().as_secs() > 8 {
                self.remaining_time = None;
            }
        }
    }
}

impl EguiOverlay for OverlayData {
    fn gui_run(
        &mut self,
        egui_context: &egui::Context,
        _default_gfx_backend: &mut DefaultGfxBackend,
        glfw_backend: &mut egui_overlay::egui_window_glfw_passthrough::GlfwBackend,
    ) {
        if !self.initialized {
            self.initialized = true;
            glfw_backend.set_window_size([2560.0, 1440.0]);
            install_image_loaders(egui_context);
            egui_context.set_theme(Theme::Light);
        }
        egui_context.input_mut(|input| {
            if input.consume_shortcut(&self.shortcuts.toggle) {
                self.toggle_ui_state();
            }
        });


        if let Ok(data) = self.reader.try_recv() {
            debug!("got data: {:?}", data);
            // TODO: use self.target_pid, grab main window, and then run below:
            // glfw_backend.window.set_pos(0, 0);
            // glfw_backend.set_window_size([2560.0, 1440.0]);
        }


        if self.ui_state == UIState::DetailActive {
            // TODO: own struct ?
            Window::new("bg").resizable(false).movable(false).title_bar(false).interactable(true).collapsible(false).order(Order::Background)
                .frame(Frame::new().fill(Color32::from_rgba_unmultiplied(220, 220, 220, 20)))
                // .default_pos(pos2(0.0,0.0)).fixed_size(vec2(2560.0, 1440.0))
                .fixed_rect(egui_context.screen_rect())
                .show(egui_context, |ui| {
                    // ui.style_mut().visuals.window_fill = Color32::from_rgba_unmultiplied(220, 220, 220, 20);
                    ui.allocate_space(ui.available_size());
                });
            // TODO: own struct
            Window::new("topbar").resizable(false).movable(false).title_bar(false).interactable(true).collapsible(false).order(Order::Foreground)
                .frame(Frame::new().fill(Color32::from_rgba_unmultiplied(20, 20, 20, 255)).inner_margin(Margin::same(10)))
                .fixed_pos(pos2(0.0, 0.0))
                .fixed_size(vec2(egui_context.screen_rect().width(), 80.0))
                .show(egui_context, |ui| {
                    ui.style_mut().spacing.item_spacing = egui::vec2(14.0, 14.0);
                    ui.style_mut().spacing.interact_size = egui::vec2(16.0, 16.0);
                    ui.style_mut().spacing.button_padding = vec2(13.0, 7.0);
                    // ui.horizontal(|ui| {
                    //
                    // });
                    MenuBar::new().ui(ui, |ui| {
                        // ui.style_mut().visuals
                       ui.menu_button("Overlay", |ui| {
                           ui.label("?");
                       });
                        ui.button("About");
                    });
//                     ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
//                         ui.button("Overlay");
//                         if ui.button("About").clicked() {
//                             // TODO: own struct
// //                             egui::Modal::new("about".into()).show(ui.ctx(), |ui| {
// //                                 // TODO: state to keep open
// //                                 ui.label("Mothers! Women!\n
// // When the years pass by and the wounds of war are stanched; when the memory of the sad and bloody days dissipates in a present of liberty, of peace and of wellbeing; when the rancor have died out and pride in a free country is felt equally by all Spaniards, speak to your children. Tell them of these men of the International Brigades.\n\
// // \n\
// // Recount for them how, coming over seas and mountains, crossing frontiers bristling with bayonets, sought by raving dogs thirsting to tear their flesh, these men reached our country as crusaders for freedom, to fight and die for Spain’s liberty and independence threatened by German and Italian fascism. \
// // They gave up everything — their loves, their countries, home and fortune, fathers, mothers, wives, brothers, sisters and children — and they came and said to us: “We are here. Your cause, Spain’s cause, is ours. It is the cause of all advanced and progressive mankind.”\n\
// // \n\
// // - Dolores Ibárruri, 1938");
// //                             });
//                         }
//                     });
                });
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

        self.startup_msg.run_ui(egui_context);

        // if let Some(prompt) = &mut self.prompt_state {
        //     Modal::new("prompt_text".into()).show(egui_context, |ui| {
        //         ui.style_mut().spacing.item_spacing = vec2(14.0, 14.0);
        //         ui.style_mut().spacing.button_padding = vec2(13.0, 7.0);
        //         ui.strong(&prompt.title);
        //         ui.add_sized(vec2(ui.available_width(), 17.0),
        //            TextEdit::singleline(&mut prompt.value)
        //              .desired_rows(1)
        //              .hint_text("Enter text here"));
        //         ui.horizontal(|ui| {
        //             // TODO: somehow dynamically set up prompt _and_ feed it back to what needs it
        //             Button::new("Submit").fill(Color32::LIGHT_BLUE).ui(ui);
        //             ui.button("Cancel");
        //         })
        //     });
        // }

        if let Some(server) = &self.server {
            for (elem_id, elem) in &mut self.elements {
                elem.show(egui_context, server);
            }
        }

        Window::new("Main Window").show(egui_context, |ui| {
            ui.style_mut().spacing.item_spacing = egui::vec2(16.0, 14.0);
            ui.style_mut().spacing.window_margin = Margin::same(10);
            ui.strong("Elements");
            if let Some(server_id) = self.server_id() {
                ScrollArea::vertical().show(ui, |ui| {
                    TableBuilder::new(ui)
                        // .auto_shrink(false)
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::remainder())
                        .header(10.0, |mut row| {
                            row.col(|col| {});
                            row.col(|col| { col.strong("Element ID"); });
                            row.col(|col| { col.strong("Template ID"); });
                            // row.col(|col| { col.strong("")});
                        })
                        .body(|mut ui| {
                            let mut changes = false;
                            let mut to_enable = vec![];
                            for (elem_id, entry) in self.store.approved_elems(server_id) {
                                ui.row(10.0, |mut ui| {
                                    ui.col(|col| {
                                        if col.checkbox(&mut entry.enabled, "").changed() {
                                            changes = true;
                                            if entry.enabled {
                                                // Can't re-use self here, so hack, throw in list and enable it later
                                                to_enable.push((entry.template_id.to_string(), elem_id.to_string(), entry.state.take()));
                                                // self.spawn_elem(&template_id, id, None);
                                            }
                                        };
                                    });
                                    ui.col(|col| {
                                        col.strong(elem_id);
                                    });
                                    ui.col(|col| {
                                        col.label(&entry.template_id);
                                    });
                                })
                            }
                            if changes {
                                for item in to_enable {
                                    self.spawn_elem(&item.0, item.1, item.2)
                                }
                                if let Err(e) = self.store.save() {
                                    error!("Failed to save element approval changes: {}", e);
                                    self.notify(MessageType::Error, Some("Saving failed".to_string()), e);
                                }
                            }
                        });
                });
            } else {
                ui.label("Not connected to server");
            }
        });


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

        if self.ui_state == UIState::DetailActive {
            glfw_backend.set_passthrough(false);
        } else {
            glfw_backend.set_passthrough(!egui_context.wants_pointer_input() && !egui_context.wants_keyboard_input());
        }
        // egui_context.request_repaint();
    }
}
