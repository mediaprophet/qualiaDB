//! QApp catalog data: categories, templates, catalog entries, card data.

mod academic_arts;
mod academic_critical;
mod academic_sciences;
mod academic_social;
mod academic_specialised;
mod ai;
mod data;
mod developer;
mod financial;
mod knowledge;
mod medical;
mod network;
mod platform;
mod quantum;
mod scientific;
mod security;

// ── Category ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Cat {
    All,
    Platform,
    Ai,
    Knowledge,
    Scientific,
    Quantum,
    Medical,
    Financial,
    Security,
    Data,
    Network,
    Developer,
    Academic,
}

impl Cat {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Cat::All => "All",
            Cat::Platform => "Platform",
            Cat::Ai => "AI & Inference",
            Cat::Knowledge => "Knowledge",
            Cat::Scientific => "Scientific",
            Cat::Quantum => "Quantum",
            Cat::Medical => "Medical",
            Cat::Financial => "Financial",
            Cat::Security => "Security",
            Cat::Data => "Data",
            Cat::Network => "Network",
            Cat::Developer => "Dev Tools",
            Cat::Academic => "Liberal Arts",
        }
    }
}

pub(crate) fn cat_list() -> Vec<Cat> {
    vec![
        Cat::All,
        Cat::Platform,
        Cat::Ai,
        Cat::Knowledge,
        Cat::Scientific,
        Cat::Quantum,
        Cat::Medical,
        Cat::Financial,
        Cat::Security,
        Cat::Data,
        Cat::Network,
        Cat::Developer,
        Cat::Academic,
    ]
}

// ── App model ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Stat {
    Active,
    Beta,
    Soon,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum AppRoute {
    ContextStudio,
    QAppStudio,
    Nexus,
}

pub(crate) struct QApp {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) tagline: &'static str,
    pub(crate) desc: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) route: Option<AppRoute>,
    pub(crate) stat: Stat,
    pub(crate) cat: Cat,
}

// ── Template model ──────────────────────────────────────────────────────────

pub(crate) struct Template {
    pub(crate) name: &'static str,
    pub(crate) desc: &'static str,
    pub(crate) icons: Vec<&'static str>,
}

pub(crate) fn featured_templates() -> Vec<Template> {
    vec![
        Template {
            name: "Scientific Research Bench",
            desc: "Physics, chemistry, ODE solver, statistics, and matrix lab in one composable workspace.",
            icons: vec![
                "lightning-charge",
                "droplet",
                "activity",
                "bar-chart-line",
                "grid-3x3",
            ],
        },
        Template {
            name: "Personal Knowledge Hub",
            desc: "Semantic graph, ontology builder, SPARQL console, N3 logic editor, and Solid LDP browser.",
            icons: vec![
                "diagram-3",
                "node-plus",
                "code-slash",
                "braces",
                "folder-symlink",
            ],
        },
        Template {
            name: "Clinical Decision Support",
            desc: "Health vitals, clinical risk scoring, DICOM viewer, and comorbidity analysis.",
            icons: vec![
                "heart-pulse",
                "clipboard2-pulse",
                "image-alt",
                "shield-plus",
            ],
        },
        Template {
            name: "Quantum Finance Lab",
            desc: "Portfolio analyser, QPU optimiser, GBM simulator, and VaR risk engine.",
            icons: vec!["currency-exchange", "cpu", "shuffle", "graph-up-arrow"],
        },
        Template {
            name: "Governance Console",
            desc: "Agreements & rights, deontic logic editor, SHACL validator, ZK proofs, and key vault.",
            icons: vec![
                "file-earmark-check",
                "journal-text",
                "check2-all",
                "eye-slash",
                "key",
            ],
        },
        Template {
            name: "AI Research Bench",
            desc: "LLM harness, LoRA adapter manager, neuro-symbolic chat, and MCP tool inspector.",
            icons: vec!["cpu-fill", "layers-half", "chat-dots", "plugin"],
        },
    ]
}

// ── Full app catalog ────────────────────────────────────────────────────────

pub(crate) fn qapp_catalog() -> Vec<QApp> {
    let mut apps = Vec::new();
    apps.extend(platform::apps());
    apps.extend(ai::apps());
    apps.extend(knowledge::apps());
    apps.extend(scientific::apps());
    apps.extend(quantum::apps());
    apps.extend(medical::apps());
    apps.extend(financial::apps());
    apps.extend(security::apps());
    apps.extend(data::apps());
    apps.extend(network::apps());
    apps.extend(developer::apps());
    apps.extend(academic_social::apps());
    apps.extend(academic_sciences::apps());
    apps.extend(academic_specialised::apps());
    apps.extend(academic_arts::apps());
    apps.extend(academic_critical::apps());
    apps
}

// ── Pre-computed card data ──────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum BtnKind {
    LaunchContext,
    LaunchQAppStudio,
    LaunchNexus,
    OpenInStudio,
    ComingSoon,
}

pub(crate) struct CardData {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) tagline: &'static str,
    pub(crate) desc: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) status_label: &'static str,
    pub(crate) status_color: &'static str,
    pub(crate) opacity: &'static str,
    pub(crate) btn: BtnKind,
}
