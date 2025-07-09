use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::sync::Arc;
use egui_overlay::egui_render_three_d::three_d::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use strum_macros::EnumString;
use uuid::Uuid;
use crate::defs::ServerInfo;

pub mod list_player;
pub trait Template/*<State> where State: Serialize + Deserialize + Debug*/ {
    /// The ID of the template, usually UUID
    fn id(&self) -> &str;

    /// Validate a state, returning Ok(()) if all fields are correct
    /// Return Err(String) with a custom error if invalid, the element will show an error instead
    fn is_state_valid(&self, state: &ElementState) -> Result<(), String> { Ok(()) }

    fn render(&self, ui: &mut egui::Ui, server: &ServerInfo, state: &mut ElementState);
}

pub type TemplateInstance = Arc<Box<dyn Template>>;
pub struct Registry {
    list: HashMap<String, TemplateInstance>,
}
impl Registry {
    pub fn new() -> Self {
        Self { list: HashMap::new() }
    }

    /// Registers a new template
    /// full_id: should be namespaced, such as overlay:my_template
    pub fn register(&mut self, full_id: &str, template: impl Template + 'static) {
        self.list.insert(full_id.into(), Arc::new(Box::new(template)));
    }

    /// Get a template by
    pub fn get(&self, id: TemplateId) -> Option<TemplateInstance> {
        self.list.get(&id.to_string()).cloned()
    }

    /// Get a template by full id
    pub fn get_2(&self, id: &str) -> Option<TemplateInstance> {
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

pub type ElementState = Value;
pub struct Element {
    pub id: String,
    pub template: TemplateInstance,
    pub state: ElementState,
}

impl Element {

    pub fn temp(template: TemplateInstance, state: ElementState) -> Self {
        Self::with_id(Uuid::new_v4().to_string(), template, state)
    }

    pub fn with_id(id: String, template: TemplateInstance, state: ElementState) -> Self {
        Self {
            id,
            template: template.clone(),
            state,
        }
    }
}
