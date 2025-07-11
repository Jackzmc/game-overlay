use std::any::Any;
use std::fmt::{Debug, Display};
use std::sync::Arc;
use egui::{pos2, Color32, Id, Response, Ui, Widget, Window};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::EnumString;
use uuid::Uuid;
use crate::defs::ServerInfo;

pub mod list_player;
pub mod generic;

pub trait Template/*<State> where State: Serialize + Deserialize + Debug*/ {
    /// The ID of the template, usually UUID
    fn id(&self) -> &str;

    /// Validate a state, returning Ok(()) if all fields are correct
    /// Return Err(String) with a custom error if invalid, the element will show an error instead
    fn is_state_valid(&self, state: &ElementState) -> Result<(), String> { Ok(()) }

    fn render(&self, ui: &mut egui::Ui, server: &ServerInfo, state: &mut ElementState);
}

pub type TemplateInstance = Arc<Box<dyn Template>>;

#[derive(Debug, PartialEq, EnumString, Default, strum_macros::Display)]
pub enum CoreTemplate {
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
pub enum TemplateId {
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

pub type ElementState = Value;
pub struct Element {
    pub id: String,
    pub win_id: egui::Id,
    pub template: TemplateInstance,
    pub state: ElementState,
}

impl Element {

    pub fn temp(template: TemplateInstance, state: ElementState) -> Self {
        Self::with_id(Uuid::new_v4().to_string(), template, state)
    }

    pub fn with_id(id: String, template: TemplateInstance, state: ElementState) -> Self {
        Self {
            id: id.clone(),
            win_id: Id::new(id),
            template: template.clone(),
            state,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, server: &ServerInfo) {
        Window::new(self.template.id())
            .id(self.id.to_string().into())
            .default_pos(pos2(1600.0, 400.0))
            .show(ctx, |ui| {
                ui.group(|ui| {
                    ui.label(format!("ID: {}", self.id));
                    ui.label(format!("TemplateId: {}", self.template.id()));
                    let pos = ui.next_widget_position();
                    ui.label(format!("Pos: ({}, {})", pos.x, pos.y));
                    ui.collapsing("State", |ui| {
                        ui.label(self.state.to_string());

                    })
                });
                if let Err(err) = self.template.is_state_valid(&self.state) {
                    ui.colored_label(Color32::RED, format!("This element has been misconfigured.\nError: {}", err));
                } else {
                    // No issues, render
                    self.template.render(ui, server, &mut self.state);
                }
            });
    }
}



pub struct TemplateInvalid;

impl Template for TemplateInvalid {
    fn id(&self) -> &str { "overlay:invalid" }

    fn is_state_valid(&self, state: &ElementState) -> Result<(), String> {
        Ok(())
    }

    fn render(&self, ui: &mut egui::Ui, server: &ServerInfo, state: &mut ElementState) {
        let template_id = state["template"].as_str().unwrap_or("[null]");
            ui.colored_label(Color32::RED, format!("Template \"{}\" does not exist", template_id));
    }
}
