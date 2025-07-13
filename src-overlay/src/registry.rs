use std::collections::HashMap;
use std::sync::Arc;
use serde_json::json;
use overlay_common::ElementState;
use crate::templates::{Element, Template, TemplateId, TemplateInstance, TemplateInvalid};

pub struct Registry {
    templates: HashMap<String, TemplateInstance>,
    // approved_elems: HashMap<SocketAddr, Vec<String>>
}

impl Registry {
    pub fn new() -> Self {
        let mut s = Self { templates: HashMap::new() };
        s.register("overlay:invalid", TemplateInvalid {});
        s
    }

    /// Registers a new template
    /// full_id: should be namespaced, such as overlay:my_template
    pub fn register(&mut self, full_id: &str, template: impl Template + 'static) {
        self.templates.insert(full_id.into(), Arc::new(Box::new(template)));
    }

    /// Get a template by
    pub fn get(&self, id: TemplateId) -> Option<TemplateInstance> {
        self.templates.get(&id.to_string()).cloned()
    }

    /// Get a template by full id
    pub fn get_2(&self, id: &str) -> Option<TemplateInstance> {
        self.templates.get(id).cloned()
    }

    pub fn has(&self, id: &str) -> bool {
        self.templates.contains_key(id)
    }

    /// Creates a new temporarily element from a template id
    pub fn temp(&self, template_id: &str, state: ElementState) -> Option<Element> {
        self.templates.get(template_id)
            .map(|template| Element::temp(template.clone(), state))
    }

    /// Creates a new temporarily element from a template id,
    /// falling back on overlay:invalid if no template id
    pub fn try_temp(&self, template_id: &str, state: ElementState) -> Element {
        self.temp(template_id, state).unwrap_or(self.invalid(template_id))
    }

    /// Creates an element with a fixed id
    pub fn named<S>(&self, template_id: &str, id: S, state: ElementState) -> Option<Element> where S: Into<String> {
        self.templates.get(template_id)
            .map(|template| Element::with_id(id.into(), template.clone(), state))
    }


    pub fn invalid(&self, id: &str) -> Element {
       Element::temp(self.templates["overlay:invalid"].clone(), json!({"template": id}))
    }
}