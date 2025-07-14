use egui::{ImageSource};
use overlay_common::ElementState;
use crate::defs::{ServerInfo, TeamShow};
use crate::templates::{Template};

#[derive(Default)]
pub struct TemplateGenericText;
impl Template for TemplateGenericText {
    fn id(&self) -> &str { "overlay:generic_text" }

    fn is_state_valid(&self, state: &ElementState) -> Result<(), String> {
        if state["content"].is_null() { return Err("'content' field is missing".to_string()) };
        Ok(())
    }

    fn render(&self, ui: &mut egui::Ui, server: &ServerInfo, state: &mut ElementState) {
        ui.label(state["content"].as_str().unwrap());
    }
}

#[derive(Default)]
pub struct TemplateGenericImage;
impl Template for TemplateGenericImage {
    fn id(&self) -> &str { "overlay:generic_image" }

    fn is_state_valid(&self, state: &ElementState) -> Result<(), String> {
        if state["url"].is_null() { return Err("'url' field is missing".to_string()) };
        Ok(())
    }

    fn render(&self, ui: &mut egui::Ui, server: &ServerInfo, state: &mut ElementState) {
        ui.image(ImageSource::Uri(state["url"].as_str().unwrap().into()));
    }
}
