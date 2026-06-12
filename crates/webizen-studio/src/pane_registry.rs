/// Pane Registry — maps semantic type hashes to human-readable component identifiers.
///
/// When the Studio canvas encounters a `PanePlacement`, it looks up the `component_id`
/// in this registry to determine which Shoelace/Dioxus widget to render.
/// This implements the SolidOS "Pane Dispatcher" pattern from redesign-web-platform.md §5.

use serde::{Deserialize, Serialize};

/// A registered pane type that the Studio knows how to render.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PaneDefinition {
    /// Human-readable identifier (e.g., "time-series-chart", "data-ingest-form")
    pub component_id: String,
    /// Display name shown in the sidebar palette
    pub display_name: String,
    /// The Shoelace/Dioxus element tag or custom component name
    pub element_tag: String,
    /// Icon name (Shoelace icon library)
    pub icon: String,
    /// Category for sidebar grouping
    pub category: PaneCategory,
    /// Default grid dimensions when dropped onto canvas
    pub default_w: u8,
    pub default_h: u8,
    /// Optional: semantic type hash this pane is bound to (for auto-dispatch)
    pub rdf_type_hash: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PaneCategory {
    DataDisplay,
    DataInput,
    Layout,
    Media,
    System,
}

/// The built-in pane palette available to all users.
/// Extensions can register additional panes at runtime.
pub fn builtin_pane_definitions() -> Vec<PaneDefinition> {
    vec![
        // --- Data Display ---
        PaneDefinition {
            component_id: "card-view".into(),
            display_name: "Card".into(),
            element_tag: "sl-card".into(),
            icon: "card-heading".into(),
            category: PaneCategory::DataDisplay,
            default_w: 4, default_h: 3,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "details-view".into(),
            display_name: "Expandable Details".into(),
            element_tag: "sl-details".into(),
            icon: "chevron-expand".into(),
            category: PaneCategory::DataDisplay,
            default_w: 6, default_h: 2,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "progress-monitor".into(),
            display_name: "Progress Bar".into(),
            element_tag: "sl-progress-bar".into(),
            icon: "bar-chart-fill".into(),
            category: PaneCategory::DataDisplay,
            default_w: 6, default_h: 1,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "badge-indicator".into(),
            display_name: "Badge / Status".into(),
            element_tag: "sl-badge".into(),
            icon: "patch-check".into(),
            category: PaneCategory::DataDisplay,
            default_w: 2, default_h: 1,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "rating-widget".into(),
            display_name: "Rating".into(),
            element_tag: "sl-rating".into(),
            icon: "star-half".into(),
            category: PaneCategory::DataDisplay,
            default_w: 4, default_h: 1,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "qr-code-display".into(),
            display_name: "QR Code".into(),
            element_tag: "sl-qr-code".into(),
            icon: "qr-code".into(),
            category: PaneCategory::DataDisplay,
            default_w: 3, default_h: 3,
            rdf_type_hash: None,
        },

        // --- Data Input ---
        PaneDefinition {
            component_id: "dynamic-form".into(),
            display_name: "SHACL Form".into(),
            element_tag: "qualia-dynamic-form".into(),
            icon: "ui-radios-grid".into(),
            category: PaneCategory::DataInput,
            default_w: 6, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "text-input".into(),
            display_name: "Text Input".into(),
            element_tag: "sl-input".into(),
            icon: "input-cursor-text".into(),
            category: PaneCategory::DataInput,
            default_w: 4, default_h: 1,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "text-area".into(),
            display_name: "Text Area".into(),
            element_tag: "sl-textarea".into(),
            icon: "textarea-resize".into(),
            category: PaneCategory::DataInput,
            default_w: 6, default_h: 3,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "checkbox-toggle".into(),
            display_name: "Checkbox".into(),
            element_tag: "sl-checkbox".into(),
            icon: "check2-square".into(),
            category: PaneCategory::DataInput,
            default_w: 3, default_h: 1,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "switch-toggle".into(),
            display_name: "Switch".into(),
            element_tag: "sl-switch".into(),
            icon: "toggles".into(),
            category: PaneCategory::DataInput,
            default_w: 3, default_h: 1,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "select-dropdown".into(),
            display_name: "Select / Dropdown".into(),
            element_tag: "sl-select".into(),
            icon: "menu-button-wide".into(),
            category: PaneCategory::DataInput,
            default_w: 4, default_h: 1,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "color-picker".into(),
            display_name: "Color Picker".into(),
            element_tag: "sl-color-picker".into(),
            icon: "palette".into(),
            category: PaneCategory::DataInput,
            default_w: 3, default_h: 3,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "range-slider".into(),
            display_name: "Range Slider".into(),
            element_tag: "sl-range".into(),
            icon: "sliders".into(),
            category: PaneCategory::DataInput,
            default_w: 6, default_h: 1,
            rdf_type_hash: None,
        },

        // --- Layout ---
        PaneDefinition {
            component_id: "tab-group".into(),
            display_name: "Tab Group".into(),
            element_tag: "sl-tab-group".into(),
            icon: "layout-text-window".into(),
            category: PaneCategory::Layout,
            default_w: 12, default_h: 6,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "split-panel".into(),
            display_name: "Split Panel".into(),
            element_tag: "sl-split-panel".into(),
            icon: "layout-split".into(),
            category: PaneCategory::Layout,
            default_w: 12, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "dialog-modal".into(),
            display_name: "Dialog / Modal".into(),
            element_tag: "sl-dialog".into(),
            icon: "window-stack".into(),
            category: PaneCategory::Layout,
            default_w: 6, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "divider".into(),
            display_name: "Divider".into(),
            element_tag: "sl-divider".into(),
            icon: "dash-lg".into(),
            category: PaneCategory::Layout,
            default_w: 12, default_h: 1,
            rdf_type_hash: None,
        },

        // --- Media ---
        PaneDefinition {
            component_id: "image-comparer".into(),
            display_name: "Image Comparer".into(),
            element_tag: "sl-image-comparer".into(),
            icon: "images".into(),
            category: PaneCategory::Media,
            default_w: 6, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "carousel".into(),
            display_name: "Carousel".into(),
            element_tag: "sl-carousel".into(),
            icon: "collection-play".into(),
            category: PaneCategory::Media,
            default_w: 8, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "avatar".into(),
            display_name: "Avatar".into(),
            element_tag: "sl-avatar".into(),
            icon: "person-circle".into(),
            category: PaneCategory::Media,
            default_w: 2, default_h: 2,
            rdf_type_hash: None,
        },

        // --- System ---
        PaneDefinition {
            component_id: "alert-notification".into(),
            display_name: "Alert / Notification".into(),
            element_tag: "sl-alert".into(),
            icon: "bell".into(),
            category: PaneCategory::System,
            default_w: 6, default_h: 2,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "spinner".into(),
            display_name: "Spinner".into(),
            element_tag: "sl-spinner".into(),
            icon: "arrow-clockwise".into(),
            category: PaneCategory::System,
            default_w: 2, default_h: 2,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "skeleton-loader".into(),
            display_name: "Skeleton Loader".into(),
            element_tag: "sl-skeleton".into(),
            icon: "layout-wtf".into(),
            category: PaneCategory::System,
            default_w: 6, default_h: 2,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "system-diagnostics".into(),
            display_name: "Diagnostics & Telemetry".into(),
            element_tag: "qualia-system-diagnostics".into(),
            icon: "cpu".into(),
            category: PaneCategory::System,
            default_w: 6, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "error-logs".into(),
            display_name: "Error & Audit Logs".into(),
            element_tag: "qualia-error-logs".into(),
            icon: "exclamation-triangle".into(),
            category: PaneCategory::System,
            default_w: 12, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "sensor-data".into(),
            display_name: "Sensor & IoT Stream".into(),
            element_tag: "qualia-sensor-data".into(),
            icon: "activity".into(),
            category: PaneCategory::DataDisplay,
            default_w: 6, default_h: 3,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "custom-web-module".into(),
            display_name: "Web Module (RPC/IFrame)".into(),
            element_tag: "qualia-web-module".into(),
            icon: "window-dock".into(),
            category: PaneCategory::System,
            default_w: 8, default_h: 6,
            rdf_type_hash: Some(qualia_core_db::q_hash("q42:WebModule")),
        },
        
        // --- Webizen Integrations ---
        PaneDefinition {
            component_id: "neuro-symbolic-chat".into(),
            display_name: "Neuro-Symbolic Chat".into(),
            element_tag: "neuro-symbolic-chat".into(),
            icon: "chat-dots".into(),
            category: PaneCategory::DataInput,
            default_w: 8, default_h: 6,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "llm-model-harness".into(),
            display_name: "LLM Model Harness".into(),
            element_tag: "llm-harness".into(),
            icon: "cpu-fill".into(),
            category: PaneCategory::System,
            default_w: 6, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "health-vital-monitor".into(),
            display_name: "Health Vital Monitor".into(),
            element_tag: "health-vital-monitor".into(),
            icon: "heart-pulse".into(),
            category: PaneCategory::DataDisplay,
            default_w: 6, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "personal-ontology-builder".into(),
            display_name: "Personal Ontology Builder".into(),
            element_tag: "personal-ontology-builder".into(),
            icon: "diagram-3".into(),
            category: PaneCategory::DataInput,
            default_w: 6, default_h: 4,
            rdf_type_hash: None,
        },
        PaneDefinition {
            component_id: "hardware-configurator".into(),
            display_name: "Hardware Configurator".into(),
            element_tag: "hardware-configurator".into(),
            icon: "tools".into(),
            category: PaneCategory::DataInput,
            default_w: 8, default_h: 6,
            rdf_type_hash: None,
        },
    ]
}

/// Look up a pane definition by component_id.
pub fn find_pane(component_id: &str) -> Option<PaneDefinition> {
    builtin_pane_definitions().into_iter().find(|p| p.component_id == component_id)
}

/// Category display name for sidebar grouping.
pub fn category_label(cat: &PaneCategory) -> &'static str {
    match cat {
        PaneCategory::DataDisplay => "Data Display",
        PaneCategory::DataInput => "Data Input",
        PaneCategory::Layout => "Layout",
        PaneCategory::Media => "Media",
        PaneCategory::System => "System",
    }
}
