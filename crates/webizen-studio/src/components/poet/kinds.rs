//! HyperCanvas vocabulary from `C:\Projects\NLP\Canvas_Workbench`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifoldId {
    Research,
    Media,
    Social,
    Settings,
}

impl ManifoldId {
    pub const ALL: [Self; 4] = [Self::Research, Self::Media, Self::Social, Self::Settings];

    pub fn id(self) -> &'static str {
        match self {
            Self::Research => "manifold-research-01",
            Self::Media => "manifold-media-01",
            Self::Social => "manifold-social-01",
            Self::Settings => "manifold-settings-01",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Research => "Research & Epistemological Mindware Manifold",
            Self::Media => "Media Production & Creative 3D Studio",
            Self::Social => "Social Governance & Multi-Agent Collaboration",
            Self::Settings => "Cybernetic Settings & Sub-Manifold Hub",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Research => "Research",
            Self::Media => "Media",
            Self::Social => "Social",
            Self::Settings => "Settings",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Research => "🔬",
            Self::Media => "🎨",
            Self::Social => "👥",
            Self::Settings => "⚙️",
        }
    }

    pub fn graph_iri(self) -> &'static str {
        match self {
            Self::Research => "graph:manifold:research_epistemology_01",
            Self::Media => "graph:manifold:media_production_01",
            Self::Social => "graph:manifold:social_governance_01",
            Self::Settings => "graph:manifold:system_settings_01",
        }
    }

    pub fn default_dim(self) -> DimMode {
        match self {
            Self::Media => DimMode::D3,
            _ => DimMode::D2,
        }
    }

    pub fn default_toolboxes(self) -> &'static [ToolboxId] {
        match self {
            Self::Research => &[
                ToolboxId::Epistemic,
                ToolboxId::Spatial,
                ToolboxId::Health,
                ToolboxId::Rights,
                ToolboxId::Office,
            ],
            Self::Media => &[
                ToolboxId::Image,
                ToolboxId::Spatial,
                ToolboxId::Sheet,
                ToolboxId::Code,
                ToolboxId::Ai,
            ],
            Self::Social => &[
                ToolboxId::Communication,
                ToolboxId::Rights,
                ToolboxId::Office,
                ToolboxId::Ai,
            ],
            Self::Settings => &[
                ToolboxId::Rights,
                ToolboxId::Code,
                ToolboxId::Communication,
                ToolboxId::Office,
                ToolboxId::Ai,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerKind {
    Subcanvas,
    Social,
    WebRtc,
    Health,
    Webview,
    Map,
    Doc,
    Ontology,
    Code,
    Sheet,
    Portal,
    Media,
    Mesh3d,
}

impl ContainerKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Subcanvas => "subcanvas",
            Self::Social => "social",
            Self::WebRtc => "webrtc",
            Self::Health => "health",
            Self::Webview => "webview",
            Self::Map => "map",
            Self::Doc => "doc",
            Self::Ontology => "ontology",
            Self::Code => "code",
            Self::Sheet => "sheet",
            Self::Portal => "portal",
            Self::Media => "media",
            Self::Mesh3d => "3d",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Subcanvas => "Sub-manifold",
            Self::Social => "Social graph",
            Self::WebRtc => "WebRTC",
            Self::Health => "Health",
            Self::Webview => "Webview",
            Self::Map => "Map",
            Self::Doc => "Document",
            Self::Ontology => "Ontology",
            Self::Code => "VibeScript",
            Self::Sheet => "Sheet",
            Self::Portal => "Portal",
            Self::Media => "Media",
            Self::Mesh3d => "3D mesh",
        }
    }

    pub fn honesty(self) -> &'static str {
        match self {
            Self::Code | Self::Doc | Self::Sheet | Self::Map | Self::Media | Self::Social
            | Self::Health | Self::Ontology => "live",
            Self::Subcanvas => "partial",
            Self::WebRtc | Self::Webview | Self::Portal | Self::Mesh3d => "present",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolboxId {
    Epistemic,
    Office,
    Image,
    Sheet,
    Spatial,
    Communication,
    Rights,
    Health,
    Code,
    Ai,
}

impl ToolboxId {
    pub const ALL: [Self; 10] = [
        Self::Epistemic,
        Self::Office,
        Self::Image,
        Self::Sheet,
        Self::Spatial,
        Self::Communication,
        Self::Rights,
        Self::Health,
        Self::Code,
        Self::Ai,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Epistemic => "epistemic",
            Self::Office => "office",
            Self::Image => "image",
            Self::Sheet => "sheet",
            Self::Spatial => "spatial",
            Self::Communication => "communication",
            Self::Rights => "rights",
            Self::Health => "health",
            Self::Code => "code",
            Self::Ai => "ai",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Epistemic => "Epistemic & Qualia Modalities",
            Self::Office => "Document & Office Toolbox",
            Self::Image => "Image, Visual & Media Toolbox",
            Self::Sheet => "Spreadsheets & Tensor Toolbox",
            Self::Spatial => "Mapping, GIS & 3D Design",
            Self::Communication => "Social, WebRTC & Webview",
            Self::Rights => "Rights, Fiduciary & Permissions",
            Self::Health => "Health, Clinical & Anatomy",
            Self::Code => "Coding & VibeScript Toolbox",
            Self::Ai => "AI Mindware & Triad Toolboxes",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Epistemic => "🎭",
            Self::Office => "📝",
            Self::Image => "🎨",
            Self::Sheet => "📊",
            Self::Spatial => "🗺️",
            Self::Communication => "📞",
            Self::Rights => "🔒",
            Self::Health => "🩺",
            Self::Code => "⚡",
            Self::Ai => "🧠",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strata {
    Environmental,
    Social,
    Legal,
    Financial,
    Technical,
}

impl Strata {
    pub const ALL: [Self; 5] = [
        Self::Environmental,
        Self::Social,
        Self::Legal,
        Self::Financial,
        Self::Technical,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Environmental => "environmental",
            Self::Social => "social",
            Self::Legal => "legal",
            Self::Financial => "financial",
            Self::Technical => "technical",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Environmental => "🌿 Env",
            Self::Social => "👥 Social",
            Self::Legal => "⚖️ Legal",
            Self::Financial => "💰 Fin",
            Self::Technical => "⚙️ Tech",
        }
    }

    pub fn css(self) -> &'static str {
        match self {
            Self::Environmental => "strata-env",
            Self::Social => "strata-social",
            Self::Legal => "strata-legal",
            Self::Financial => "strata-fin",
            Self::Technical => "strata-tech",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Epistemic {
    All,
    Objective,
    Subjective,
    Intersubjective,
    Normative,
}

impl Epistemic {
    pub fn id(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Objective => "objective",
            Self::Subjective => "subjective",
            Self::Intersubjective => "intersubjective",
            Self::Normative => "normative",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Objective => "🔬",
            Self::Subjective => "🧠",
            Self::Intersubjective => "🌊",
            Self::Normative => "⚖️",
        }
    }

    pub fn css(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Objective => "modality-objective",
            Self::Subjective => "modality-subjective",
            Self::Intersubjective => "modality-intersubjective",
            Self::Normative => "modality-normative",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimMode {
    D2,
    D3,
    D4,
}

impl DimMode {
    pub fn id(self) -> &'static str {
        match self {
            Self::D2 => "2D",
            Self::D3 => "3D",
            Self::D4 => "4D",
        }
    }

    pub fn css(self) -> &'static str {
        match self {
            Self::D2 => "",
            Self::D3 => "mode-3d",
            Self::D4 => "mode-4d",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPos {
    Left,
    Top,
    Right,
    Bottom,
}

impl DockPos {
    pub fn id(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasNode {
    pub id: String,
    pub kind: ContainerKind,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z: f64,
    pub d: f64,
    pub strata: Strata,
    pub epistemic: Epistemic,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Wire {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ToolSpec {
    pub label: &'static str,
    pub places: Option<ContainerKind>,
    pub honesty: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypercanvas_ids_match_nlp_registry() {
        assert_eq!(ManifoldId::Research.id(), "manifold-research-01");
        assert_eq!(ContainerKind::Mesh3d.id(), "3d");
        assert_eq!(ToolboxId::Communication.icon(), "📞");
        assert_eq!(ContainerKind::Code.honesty(), "live");
        assert_eq!(ContainerKind::WebRtc.honesty(), "present");
    }
}
