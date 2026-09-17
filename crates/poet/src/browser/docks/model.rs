//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Cloneable dock view models, family order, and toolbox-view storage.

use std::cell::RefCell;

use crate::tool_chest::core::intent_bus::ActionType;
use crate::tool_chest::core::tool::ToolKind;
use crate::tool_chest::core::tool_chain::ToolChainMetadata;
use crate::tool_chest::core::toolbox::{Toolbox, ToolboxMetadata};

use crate::browser::tool_widgets::ToolWidget;

// ---------------------------------------------------------------------------
// Cloneable view models (the registry holds Box<dyn Tool> which is not Clone)
// ---------------------------------------------------------------------------

/// A cloneable view of a single tool's metadata for UI rendering.
#[derive(Clone, Debug)]
pub struct ToolView {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub kind: ToolKind,
    pub action: ActionType,
    pub capability_scope: Option<String>,
    pub description: String,
}

/// A cloneable view of a tool-chain with its tools and domain widgets.
#[derive(Clone, Debug)]
pub struct ToolChainView {
    pub metadata: ToolChainMetadata,
    pub tools: Vec<ToolView>,
    pub widgets: Vec<ToolWidget>,
}

/// A cloneable view of a toolbox with its tool-chains.
#[derive(Clone, Debug)]
pub struct ToolboxView {
    pub metadata: ToolboxMetadata,
    pub chains: Vec<ToolChainView>,
}

/// Dock position orientations for 4-way docking architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPosition {
    Left,
    Top,
    Right,
    Bottom,
}

impl DockPosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            DockPosition::Left => "left",
            DockPosition::Top => "top",
            DockPosition::Right => "right",
            DockPosition::Bottom => "bottom",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "top" => DockPosition::Top,
            "right" => DockPosition::Right,
            "bottom" => DockPosition::Bottom,
            _ => DockPosition::Left,
        }
    }
}

/// Metadata for a toolbox family group.
#[derive(Clone, Debug)]
pub struct ToolboxFamily {
    pub id: String,
    pub label: String,
    pub icon: String,
}

/// Get the ordered list of 12 master toolbox families.
pub fn family_order() -> Vec<ToolboxFamily> {
    vec![
        ToolboxFamily {
            id: "epistemic".into(),
            label: "Epistemic Mindware".into(),
            icon: "\u{1F9ED}".into(), // 🧭
        },
        ToolboxFamily {
            id: "authoring".into(),
            label: "Word Processor & CML".into(),
            icon: "\u{1F4DD}".into(), // 📝
        },
        ToolboxFamily {
            id: "sheet".into(),
            label: "Spreadsheets & Tensors".into(),
            icon: "\u{1F4CA}".into(), // 📊
        },
        ToolboxFamily {
            id: "graphics".into(),
            label: "Graphics & Vector".into(),
            icon: "\u{1F3A8}".into(), // 🎨
        },
        ToolboxFamily {
            id: "spatial".into(),
            label: "3D & Geospatial".into(),
            icon: "\u{1F9CA}".into(), // 🧊
        },
        ToolboxFamily {
            id: "audio".into(),
            label: "Triad Formant Audio".into(),
            icon: "\u{1F399}\u{FE0F}".into(), // 🎙️
        },
        ToolboxFamily {
            id: "code".into(),
            label: "Code IDE & Vibe REPL".into(),
            icon: "\u{1F4BB}".into(), // 💻
        },
        ToolboxFamily {
            id: "erp".into(),
            label: "Cooperative ERP & PM".into(),
            icon: "\u{1F4C5}".into(), // 📅
        },
        ToolboxFamily {
            id: "mail".into(),
            label: "Mail & Web Presence".into(),
            icon: "\u{2709}\u{FE0F}".into(), // ✉️
        },
        ToolboxFamily {
            id: "lab".into(),
            label: "Scientific & Clinical".into(),
            icon: "\u{1F52C}".into(), // 🔬
        },
        ToolboxFamily {
            id: "ai".into(),
            label: "AI Co-Pilot & Sentinel".into(),
            icon: "\u{2728}".into(), // ✨
        },
        ToolboxFamily {
            id: "governance".into(),
            label: "Governance & Rights".into(),
            icon: "\u{2696}\u{FE0F}".into(), // ⚖️
        },
        ToolboxFamily {
            id: "sdn".into(),
            label: "SDN & Economics".into(),
            icon: "\u{1F310}".into(), // 🌐
        },
    ]
}

/// Extract cloneable views from the registry's toolboxes.
pub fn extract_toolbox_views(toolboxes: &[Toolbox]) -> Vec<ToolboxView> {
    toolboxes
        .iter()
        .map(|tb| ToolboxView {
            metadata: tb.metadata().clone(),
            chains: tb
                .chains()
                .iter()
                .map(|chain| {
                    let tools: Vec<ToolView> = chain
                        .tools()
                        .iter()
                        .map(|tool| {
                            let m = tool.metadata();
                            let copy = crate::browser::tool_copy::presentation(
                                &m.id,
                                &m.label,
                                &m.description,
                            );
                            ToolView {
                                id: m.id.clone(),
                                label: copy.label,
                                icon: m.icon.clone(),
                                kind: m.kind,
                                action: tool.action_type(),
                                capability_scope: m.capability_scope.clone(),
                                description: copy.tooltip,
                            }
                        })
                        .collect();

                    let widgets =
                        super::widgets::build_toolchain_widgets(&chain.metadata().id, &tools);

                    ToolChainView {
                        metadata: chain.metadata().clone(),
                        tools,
                        widgets,
                    }
                })
                .collect(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Thread-local storage for flyout rendering
// ---------------------------------------------------------------------------

thread_local! {
    static TOOLBOX_VIEWS: RefCell<Vec<ToolboxView>> = RefCell::new(Vec::new());
}

/// Store toolbox views in the thread-local for access from click handlers.
pub fn store_toolbox_views(views: Vec<ToolboxView>) {
    TOOLBOX_VIEWS.with(|v| {
        *v.borrow_mut() = views;
    });
}

pub(super) fn find_stored_toolbox_view(toolbox_id: &str) -> Option<ToolboxView> {
    TOOLBOX_VIEWS.with(|v| {
        v.borrow()
            .iter()
            .find(|t| t.metadata.id == toolbox_id)
            .cloned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_toolbox_families_count() {
        let families = family_order();
        assert!(
            families.len() >= 12,
            "Expected at least 12 master toolbox families, got {}",
            families.len()
        );
        let ids: Vec<&str> = families.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"epistemic"));
        assert!(ids.contains(&"authoring"));
        assert!(ids.contains(&"sheet"));
        assert!(ids.contains(&"graphics"));
        assert!(ids.contains(&"spatial"));
        assert!(ids.contains(&"audio"));
        assert!(ids.contains(&"code"));
        assert!(ids.contains(&"erp"));
        assert!(ids.contains(&"mail"));
        assert!(ids.contains(&"lab"));
        assert!(ids.contains(&"ai"));
        assert!(ids.contains(&"governance"));
        assert!(ids.contains(&"sdn"));
    }

    #[test]
    fn test_dock_position_conversions() {
        assert_eq!(DockPosition::from_str("top"), DockPosition::Top);
        assert_eq!(DockPosition::from_str("right"), DockPosition::Right);
        assert_eq!(DockPosition::from_str("bottom"), DockPosition::Bottom);
        assert_eq!(DockPosition::from_str("left"), DockPosition::Left);
        assert_eq!(DockPosition::from_str("invalid"), DockPosition::Left);

        assert_eq!(DockPosition::Top.as_str(), "top");
        assert_eq!(DockPosition::Right.as_str(), "right");
        assert_eq!(DockPosition::Bottom.as_str(), "bottom");
        assert_eq!(DockPosition::Left.as_str(), "left");
    }
}
