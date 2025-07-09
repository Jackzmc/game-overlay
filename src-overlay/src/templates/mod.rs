use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use egui_overlay::egui_render_three_d::three_d::Context;
use serde_json::{json, Value};
use strum_macros::EnumString;
use uuid::Uuid;
use crate::defs::ServerInfo;

pub mod list_player;
pub trait Template {
    /// The ID of the template, usually UUID
    fn id(&self) -> &str;

    fn render(&self, ui: &mut egui::Ui, server: &ServerInfo, variables: &mut Value);
}

pub struct Registry {
    list: HashMap<String, Arc<Box<dyn Template>>>,
}
impl Registry {
    pub fn new() -> Self {
        Self { list: HashMap::new() }
    }

    pub fn register(&mut self, full_id: &str, template: Box<dyn Template>) {
        self.list.insert(full_id.into(), Arc::new(template));
    }

    pub fn get(&self, id: TemplateId) -> Option<Arc<Box<dyn Template>>> {
        self.list.get(&id.to_string()).cloned()
    }

    pub fn get_2(&self, id: &str) -> Option<Arc<Box<dyn Template>>> {
        self.list.get(id).cloned()
    }
}

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

pub struct Element {
    pub id: String,
    pub template: Arc<Box<dyn Template>>,
    pub variables: Value
}

impl Element {

    pub fn temp(template: Arc<Box<dyn Template>>, variables: Value) -> Self {
        Self::with_id(Uuid::new_v4().to_string(), template, variables)
    }

    pub fn with_id(id: String, template: Arc<Box<dyn Template>>, variables: Value) -> Self {
        Self {
            id,
            template: template.clone(),
            variables,
        }
    }
}
