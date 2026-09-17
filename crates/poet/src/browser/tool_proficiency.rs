//! Getting started / Everyday / Workshop presentation for the Tool Chest.
//!
//! The same record serves humans (labels, tooltips, ARIA) and agents
//! (`data-tool-id`, capability token, catalog JSON). Machine ids stay
//! under the hood except in Workshop mode.

use serde::{Deserialize, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event};

const STORAGE_KEY: &str = "qualia-ui:tool-proficiency";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Proficiency {
    #[default]
    Novice,
    Intermediate,
    Expert,
}

impl Proficiency {
    pub fn as_token(self) -> &'static str {
        match self {
            Self::Novice => "novice",
            Self::Intermediate => "intermediate",
            Self::Expert => "expert",
        }
    }

    pub fn human_label(self) -> &'static str {
        match self {
            Self::Novice => "Getting started",
            Self::Intermediate => "Everyday",
            Self::Expert => "Workshop",
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            Self::Novice => "Show a short set of everyday tools with plain-language names.",
            Self::Intermediate => "Show the usual working set. Technical names stay hidden.",
            Self::Expert => "Show every tool, including workshop names and machine ids.",
        }
    }

    #[allow(dead_code)]
    pub fn from_token(token: &str) -> Self {
        match token {
            "intermediate" => Self::Intermediate,
            "expert" => Self::Expert,
            _ => Self::Novice,
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::Novice => 0,
            Self::Intermediate => 1,
            Self::Expert => 2,
        }
    }

    pub fn shows(self, minimum: Self) -> bool {
        self.rank() >= minimum.rank()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Preference {
    #[serde(default)]
    global: Proficiency,
}

pub fn current() -> Proficiency {
    load().global
}

pub fn restore(document: &Document) {
    apply(document, current());
}

pub fn apply(document: &Document, level: Proficiency) {
    let token = level.as_token();
    if let Some(root) = document.document_element() {
        let _ = root.set_attribute("data-tool-proficiency", token);
    }
    if let Some(app) = document.query_selector(".app").ok().flatten() {
        let _ = app.set_attribute("data-tool-proficiency", token);
    }
    if let Some(dock) = document.query_selector(".toolbox-dock").ok().flatten() {
        let _ = dock.set_attribute("data-tool-proficiency", token);
        let _ = dock.set_attribute("data-agent-catalog", &agent_catalog_json(level));
    }
    if let Ok(buttons) = document.query_selector_all("[data-proficiency-choice]") {
        for index in 0..buttons.length() {
            let Some(node) = buttons.get(index) else {
                continue;
            };
            let Ok(button) = node.dyn_into::<Element>() else {
                continue;
            };
            let chosen = button.get_attribute("data-proficiency-choice") == Some(token.into());
            let _ = button.set_attribute("aria-pressed", if chosen { "true" } else { "false" });
            if chosen {
                let _ = button.class_list().add_1("is-active");
            } else {
                let _ = button.class_list().remove_1("is-active");
            }
        }
    }
}

pub fn persist(level: Proficiency) {
    let preference = Preference { global: level };
    if let Ok(json) = serde_json::to_string(&preference) {
        super::storage_set(STORAGE_KEY, &json);
    }
}

fn load() -> Preference {
    super::storage_get(STORAGE_KEY)
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn set(document: &Document, level: Proficiency) {
    persist(level);
    apply(document, level);
    if let Some(flyout) = document.query_selector(".toolbox-flyout").ok().flatten() {
        if let Some(toolbox_id) = flyout.get_attribute("data-toolbox-id") {
            super::docks::show_flyout(document, &toolbox_id);
        }
    }
}

pub fn render_switcher(document: &Document) -> Element {
    let row = document.create_element("div").unwrap();
    row.set_class_name("tool-proficiency-switcher");
    row.set_attribute("role", "radiogroup").ok();
    row.set_attribute("aria-label", "How many tools to show")
        .ok();
    let current = current();
    for level in [
        Proficiency::Novice,
        Proficiency::Intermediate,
        Proficiency::Expert,
    ] {
        let button = document.create_element("button").unwrap();
        button.set_class_name("tool-proficiency-choice");
        button.set_attribute("type", "button").ok();
        button
            .set_attribute("data-proficiency-choice", level.as_token())
            .ok();
        button.set_attribute("role", "radio").ok();
        button
            .set_attribute(
                "aria-pressed",
                if level == current { "true" } else { "false" },
            )
            .ok();
        button.set_attribute("title", level.tooltip()).ok();
        button.set_attribute("aria-label", level.tooltip()).ok();
        if level == current {
            let _ = button.class_list().add_1("is-active");
        }
        button.set_text_content(Some(level.human_label()));
        let chosen = level;
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            set(&document, chosen);
        }) as Box<dyn FnMut(Event)>);
        let _ = button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
        closure.forget();
        row.append_child(&button).unwrap();
    }
    row
}

pub fn agent_catalog_json(level: Proficiency) -> String {
    let registry = super::registration::build_registry();
    let mut tools = Vec::new();
    for toolbox in registry.toolboxes() {
        for chain in toolbox.chains() {
            for tool in chain.tools() {
                let meta = tool.metadata();
                let copy = super::tool_copy::presentation(&meta.id, &meta.label, &meta.description);
                if !level.shows(copy.min_proficiency) {
                    continue;
                }
                let available = super::tool_actions::current_disabled_reason(&meta.id).is_none();
                tools.push(serde_json::json!({
                    "id": meta.id,
                    "label": copy.label,
                    "tooltip": copy.tooltip,
                    "capability": meta.capability_scope,
                    "available": available,
                    "min_proficiency": copy.min_proficiency.as_token(),
                }));
            }
        }
    }
    serde_json::json!({
        "audience": ["human", "agent"],
        "proficiency": level.as_token(),
        "tools": tools,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn novice_hides_expert_tools() {
        assert!(Proficiency::Expert.shows(Proficiency::Novice));
        assert!(!Proficiency::Novice.shows(Proficiency::Expert));
        assert!(Proficiency::Intermediate.shows(Proficiency::Novice));
        assert!(!Proficiency::Intermediate.shows(Proficiency::Expert));
    }

    #[test]
    fn agent_catalog_is_json_and_omits_coder_verbs() {
        let json = agent_catalog_json(Proficiency::Novice);
        let value: serde_json::Value = serde_json::from_str(&json).expect("json");
        assert_eq!(value["proficiency"], "novice");
        let tools = value["tools"].as_array().expect("tools");
        assert!(!tools.is_empty());
        for tool in tools {
            let label = tool["label"].as_str().unwrap_or("");
            let tooltip = tool["tooltip"].as_str().unwrap_or("");
            assert!(!label.contains("capability.invoke"));
            assert!(!tooltip.contains("capability.invoke"));
            assert!(!label.contains("ALL_BOUND"));
        }
    }
}
