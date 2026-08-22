//! HyperCanvas vocabulary from `C:\Projects\NLP\Canvas_Workbench` and `POET-SPEC-001..023`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ManifoldId {
    Research,
    Media,
    Social,
    Mail,
    Chora,
    Settings,
}

impl ManifoldId {
    pub const ALL: [Self; 6] = [
        Self::Research,
        Self::Media,
        Self::Social,
        Self::Mail,
        Self::Chora,
        Self::Settings,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Research => "manifold-research-01",
            Self::Media => "manifold-media-01",
            Self::Social => "manifold-social-01",
            Self::Mail => "manifold-mail-01",
            Self::Chora => "manifold-chora-01",
            Self::Settings => "manifold-settings-01",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Research => "Research & Epistemological Mindware Manifold",
            Self::Media => "Media Production & Creative 3D Studio",
            Self::Social => "Social Governance & Multi-Agent Collaboration",
            Self::Mail => "Inalienable Communications & Domain Presence",
            Self::Chora => "Chora 4D Spatio-Temporal Commons",
            Self::Settings => "Webizen Node Admin & Sentinel Governance",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Research => "Research",
            Self::Media => "Media",
            Self::Social => "Social ERP",
            Self::Mail => "Mail",
            Self::Chora => "Chora",
            Self::Settings => "Settings",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Research => "🔬",
            Self::Media => "🎨",
            Self::Social => "👥",
            Self::Mail => "✉️",
            Self::Chora => "🌐",
            Self::Settings => "⚙️",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Self::Research => "Hypermedia document synthesis, ontology graphs, and clinical calculators.",
            Self::Media => "3D CCF anatomy meshes, EnCodec P64 audio streams, and graphics editing.",
            Self::Social => "Cooperative ERP, Workstream A deliverable cards, and voting ballots.",
            Self::Mail => "Domain-first inalienable email, CML composer, and WebID publishing.",
            Self::Chora => "Dialectical web reader, 4D vision point clouds, and spatial commons.",
            Self::Settings => "42MB Sentinel watchdog, local GGUF models, and P2P SDN swarms.",
        }
    }

    pub fn graph_iri(self) -> &'static str {
        match self {
            Self::Research => "graph:manifold:research_epistemology_01",
            Self::Media => "graph:manifold:media_production_01",
            Self::Social => "graph:manifold:social_governance_01",
            Self::Mail => "graph:manifold:domain_communications_01",
            Self::Chora => "graph:manifold:chora_commons_01",
            Self::Settings => "graph:manifold:system_settings_01",
        }
    }

    pub fn default_dim(self) -> DimMode {
        match self {
            Self::Media | Self::Chora => DimMode::D3,
            _ => DimMode::D2,
        }
    }

    pub fn default_toolboxes(self) -> &'static [ToolboxId] {
        match self {
            Self::Research => &[
                ToolboxId::Epistemic,
                ToolboxId::Office,
                ToolboxId::Spatial,
                ToolboxId::Health,
                ToolboxId::Scientific,
            ],
            Self::Media => &[
                ToolboxId::Image,
                ToolboxId::Spatial,
                ToolboxId::Audio,
                ToolboxId::Code,
                ToolboxId::Ai,
            ],
            Self::Social => &[
                ToolboxId::Erp,
                ToolboxId::Communication,
                ToolboxId::Rights,
                ToolboxId::Sdn,
            ],
            Self::Mail => &[
                ToolboxId::Mail,
                ToolboxId::Office,
                ToolboxId::Communication,
                ToolboxId::Rights,
            ],
            Self::Chora => &[
                ToolboxId::Spatial,
                ToolboxId::Epistemic,
                ToolboxId::Image,
                ToolboxId::Sdn,
            ],
            Self::Settings => &[
                ToolboxId::Rights,
                ToolboxId::Code,
                ToolboxId::Sdn,
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
    Mail,
    Chora,
    ErpKanban,
    GitForge,
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
            Self::Mail => "mail",
            Self::Chora => "chora",
            Self::ErpKanban => "kanban",
            Self::GitForge => "git",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Subcanvas => "Sub-manifold",
            Self::Social => "Social graph",
            Self::WebRtc => "WebRTC",
            Self::Health => "Health & Clinical",
            Self::Webview => "Webview",
            Self::Map => "Map & GIS",
            Self::Doc => "HyperDoc",
            Self::Ontology => "Ontology",
            Self::Code => "VibeScript IDE",
            Self::Sheet => "Spreadsheet",
            Self::Portal => "Portal",
            Self::Media => "Media Studio",
            Self::Mesh3d => "3D Anatomy Mesh",
            Self::Mail => "Inalienable Mail",
            Self::Chora => "Chora Dialectical",
            Self::ErpKanban => "Cooperative Kanban",
            Self::GitForge => "Distributed Git",
        }
    }

    pub fn honesty(self) -> &'static str {
        match self {
            Self::Code
            | Self::Doc
            | Self::Sheet
            | Self::Map
            | Self::Media
            | Self::Social
            | Self::Health
            | Self::Ontology
            | Self::Mail
            | Self::ErpKanban
            | Self::GitForge => "live",
            Self::Subcanvas | Self::Chora => "partial",
            Self::WebRtc | Self::Webview | Self::Portal | Self::Mesh3d => "present",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolboxId {
    Epistemic,
    Office,
    Image,
    Sheet,
    Spatial,
    Audio,
    Code,
    Erp,
    Mail,
    Scientific,
    Ai,
    Rights,
    Communication,
    Health,
    Sdn,
}

impl ToolboxId {
    pub const ALL: [Self; 12] = [
        Self::Office,
        Self::Sheet,
        Self::Image,
        Self::Spatial,
        Self::Audio,
        Self::Code,
        Self::Erp,
        Self::Mail,
        Self::Scientific,
        Self::Ai,
        Self::Rights,
        Self::Sdn,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Epistemic => "epistemic",
            Self::Office => "office",
            Self::Image => "image",
            Self::Sheet => "sheet",
            Self::Spatial => "spatial",
            Self::Audio => "audio",
            Self::Code => "code",
            Self::Erp => "erp",
            Self::Mail => "mail",
            Self::Scientific => "scientific",
            Self::Ai => "ai",
            Self::Rights => "rights",
            Self::Communication => "communication",
            Self::Health => "health",
            Self::Sdn => "sdn",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Epistemic => "Epistemic & Qualia Modalities",
            Self::Office => "Word Processor & CML Authoring",
            Self::Image => "Graphics, Vector Drawing & Image",
            Self::Sheet => "Spreadsheets & Vibe Formulas",
            Self::Spatial => "3D Kinematics & CCF Anatomy",
            Self::Audio => "Audio, Triad Synth & Speech",
            Self::Code => "VibeScript IDE & Shader Forge",
            Self::Erp => "Cooperative ERP & Workstream",
            Self::Mail => "Inalienable Mail & Web Publisher",
            Self::Scientific => "Scientific Labs & Physics",
            Self::Ai => "AI Mindware & Deontic Co-Pilot",
            Self::Rights => "Governance, DID Keys & Sanctuary",
            Self::Communication => "Social, WebRTC & Webview",
            Self::Health => "Health, Clinical & Sensory",
            Self::Sdn => "SDN & Cooperative Economics",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Epistemic => "🎭",
            Self::Office => "📝",
            Self::Image => "🎨",
            Self::Sheet => "📊",
            Self::Spatial => "🧊",
            Self::Audio => "🫀",
            Self::Code => "⚡",
            Self::Erp => "📋",
            Self::Mail => "✉️",
            Self::Scientific => "🧪",
            Self::Ai => "🧠",
            Self::Rights => "🔒",
            Self::Communication => "📞",
            Self::Health => "🩺",
            Self::Sdn => "🌐",
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

#[derive(Clone, Debug, PartialEq)]
pub struct ContainerInstance {
    pub id: String,
    pub on: ManifoldId,
    pub kind: ContainerKind,
    pub title: String,
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
