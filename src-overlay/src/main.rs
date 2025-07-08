#![windows_subsystem = "windows"] // to turn off console.

use std::net::{IpAddr, SocketAddr};
use std::time::Instant;
use egui::{Align2, DragValue, RichText};
use egui_overlay::EguiOverlay;
use egui_overlay::egui_render_three_d::ThreeDBackend as DefaultGfxBackend;

use std::str::FromStr;

struct ServerInfo {
    name: String,
    ip_addr: SocketAddr,
    connected_time: Instant,

    players: Vec<PlayerInfo>,
    my_player: PlayerInfo
}

struct PlayerInfo {
    steamid: String,
    name: String,
}

struct OverlayState {
    server: Option<ServerInfo>,
    initialized: bool,
    startup_message_remain: Option<Instant>
}

fn main() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    // if RUST_LOG is not set, we will use the following filters
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or(EnvFilter::new("overlay_ui=debug,warn")),
        )
        .init();

    let state = OverlayState {
        server: Some(ServerInfo {
            name: "My Server".to_string(),
            ip_addr: SocketAddr::from_str("127.0.0.1:27015").unwrap(),
            connected_time: Instant::now(),

            players: vec![],
            my_player: PlayerInfo {
                name: "Jackzie".to_string(),
                steamid: "1".to_string(),
            }
        }),
        initialized: false,
        startup_message_remain: Some(Instant::now())
    };
    egui_overlay::start(state);
}

impl EguiOverlay for OverlayState {
    fn gui_run(
        &mut self,
        egui_context: &egui::Context,
        _default_gfx_backend: &mut DefaultGfxBackend,
        glfw_backend: &mut egui_window_glfw_passthrough::GlfwBackend,
    ) {
        if !self.initialized {
            self.initialized = true;
            glfw_backend.set_window_size([2560.0, 1440.0]);
        }

        if let Some(startup_msg_time) = self.startup_message_remain {
            egui::Window::new("welcome_message")
                // .default_pos(egui::pos2(200.0, 400.0))
                .anchor(Align2::RIGHT_TOP, egui::vec2(-15.0, 15.0))
                .title_bar(false)
                .fade_in(true)
                .fade_out(true)
                .show(egui_context, |ui| {
                    ui.label("Detected left4dead2");
                    ui.label("Connect to a supported server to use the overlay");

                    ui.add_space(14.0);
                    ui.label("Open the overlay with CTRL+HOME");
                });
            // End the message
            if startup_msg_time.elapsed().as_secs() > 8 {
                self.startup_message_remain = None;
            }
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



        egui::Window::new("state").show(egui_context, |ui| {
            if let Some(server) = &self.server {
                ui.label(format!("Connected to {} ({})", server.name, server.ip_addr));
                ui.label(format!("Playing for {} minute(s)", server.connected_time.elapsed().as_secs() / 64));
                ui.label(format!("Hello {}", server.my_player.name));
            }
        });

        let needs_controls = egui_context.wants_pointer_input() || egui_context.wants_keyboard_input();
        glfw_backend.set_passthrough(!needs_controls);

        // egui_context.request_repaint();
    }
}
