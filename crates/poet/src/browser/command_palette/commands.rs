//! Command entries for the command palette.

pub(super) struct CommandEntry {
    pub icon: &'static str,
    pub label: &'static str,
    pub shortcut: &'static str,
}

pub(super) fn build_command_list() -> Vec<CommandEntry> {
    vec![
        CommandEntry {
            icon: "\u{1F3E0}",
            label: "Open Construct: POET",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2764}",
            label: "Open Construct: Health",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9EA}",
            label: "Open Construct: Research lab",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3A8}",
            label: "Open Construct: Studio",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Open Construct: Rights",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F578}",
            label: "Open Construct: Knowledge",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F91D}",
            label: "Open Construct: Projects",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9B4}",
            label: "Anatomy manifold",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E6}",
            label: "Construct Shelf",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2795}",
            label: "Author manifold",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E6}",
            label: "Author container",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F517}",
            label: "Author nested link",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F331}",
            label: "Author subject",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2191}",
            label: "Pop nested manifold",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F465}",
            label: "Invite participant",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F50C}",
            label: "New Document",
            shortcut: "Ctrl+N",
        },
        CommandEntry {
            icon: "\u{1F4CA}",
            label: "New Sheet",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2728}",
            label: "Auto-Arrange Manifold (Tidy)",
            shortcut: "Alt+A",
        },
        CommandEntry {
            icon: "\u{1F50D}",
            label: "Search Workbench",
            shortcut: "Ctrl+Shift+F",
        },
        CommandEntry {
            icon: "\u{1F3AF}",
            label: "Faceted Search",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9F9}",
            label: "SPARQL Query Builder",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{270F}\u{FE0F}",
            label: "Manual SPARQL Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4BE}",
            label: "Saved Queries",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F50D}",
            label: "Run SPARQL Query",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9E0}",
            label: "Logic Workbench",
            shortcut: "Ctrl+Shift+L",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Deontic Rule Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9E9}",
            label: "N3 Logic Studio",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2705}",
            label: "SHACL Validator",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B50}",
            label: "RDF-Star Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4D6}",
            label: "Ontology Builder",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9E0}",
            label: "Evaluate Modality",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F50E}",
            label: "Symbolic Logic Inference",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Jural Relations",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4AC}",
            label: "Argumentation Framework",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3AF}",
            label: "STIT Agency",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F539}",
            label: "Causal Liability",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Responsibility / Meta-Guard",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CB}",
            label: "Capacity Evaluator",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F517}",
            label: "Delegation Tracker",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4DD}",
            label: "Contract Formation",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2705}",
            label: "Consensus / Partition",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4DC}",
            label: "Meta-Deontic Breach",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B0}",
            label: "Value Flow / Commons",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F6E1}",
            label: "Interaction Governance",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F575}",
            label: "Identity Fabric",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CA}",
            label: "Capability Gap Analyzer",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F510}",
            label: "Legal Compose",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Deontic Compose",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9E0}",
            label: "Epistemic Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{26A1}",
            label: "Paraconsistent Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{23F1}",
            label: "Linear Temporal Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F534}",
            label: "Computation Tree Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4A1}",
            label: "Answer Set Programming",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Defeasible Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B8}",
            label: "Linear Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F50D}",
            label: "Description Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9ED}",
            label: "Dialectical Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F50D}",
            label: "Abductive Reasoning",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4A0}",
            label: "Fuzzy Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B9}",
            label: "Probabilistic Reasoning",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F5C2}",
            label: "Graph Theory",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{23F1}",
            label: "Interval Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F300}",
            label: "Manifold 10D Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F6E1}",
            label: "Epistemic Boundaries",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{25FB}",
            label: "Modal Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F49A}",
            label: "Clinical Risk Scorer",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F5BC}",
            label: "DICOM Viewer",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9EA}",
            label: "Comorbidity Analyzer",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9EA}",
            label: "Chemistry Modeler",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{269B}",
            label: "Physics Simulator",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F501}",
            label: "ODE Solver",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9EC}",
            label: "Bioinformatics Lab",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B9}",
            label: "GBM / VaR Simulator",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F300}",
            label: "Diffusion Controller",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F50D}",
            label: "Bytecode / VM Inspector",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CB}",
            label: "SLG Arena Inspector",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F525}",
            label: "Forge Compute Probe",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4BB}",
            label: "Compute Profile",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F510}",
            label: "Privacy / HE / DP",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E6}",
            label: "Model Lifecycle",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CA}",
            label: "Inference Monitor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9F8}",
            label: "GGUF Tokenizer Inspector",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E6}",
            label: "P64 Weight Inspector",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F501}",
            label: "CRDT / Sync Dashboard",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F511}",
            label: "Agency / Merkle Inspector",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F510}",
            label: "Key Vault Manager",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F6E1}",
            label: "Policy Evaluator",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2705}",
            label: "Consent Manager",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E6}",
            label: "Carrier / Media Binding",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F501}",
            label: "Control Feedback",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CF}",
            label: "Likeliness",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9EE}",
            label: "QUBO Compiler",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F527}",
            label: "OWL Converter",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4D0}",
            label: "Allen / RCC8",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F300}",
            label: "Manifold Logic",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9EE}",
            label: "Calculus",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3DB}",
            label: "Browse Ontology",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4AC}",
            label: "Open Social Graph",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E1}",
            label: "Open Pulse Stream",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F50D}",
            label: "Open Aura (SHACL)",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CD}",
            label: "Open Map (GIS)",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4A1}",
            label: "Open VibeScript Console",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F510}",
            label: "Open Rights & Agreements",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B0}",
            label: "Open Wallet",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Research",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Social",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Knowledge",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Projects",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Rights",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Sanctuary",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Media",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Communications",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Settings",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2B07}",
            label: "Switch Manifold: Vibe",
            shortcut: "",
        },
        // Workstream A — Collaborative/ERP/PM
        CommandEntry {
            icon: "\u{1F4CB}",
            label: "Project Sheet",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C4}",
            label: "Work Items Kanban",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B0}",
            label: "Budget & Finance",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B2}",
            label: "Cost Base & Obligation",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E6}",
            label: "Deliverables & Artifacts",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2705}",
            label: "Reviews & Decisions",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4AC}",
            label: "Discussion",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C5}",
            label: "Roadmap & Phases",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F33F}",
            label: "Commons Publication",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4DC}",
            label: "New Agreement",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4DC}",
            label: "Agreement Builder",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B0}",
            label: "Compensation Model",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4D3}",
            label: "Contribution Ledger",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4DC}",
            label: "License Builder",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Obligation Tracker",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C4}",
            label: "IP Registry",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CA}",
            label: "Data Sources",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Disputes",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Complaints",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{270F}",
            label: "Corrections",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C5}",
            label: "Governance Meetings",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{26A0}",
            label: "Conflict of Interest",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E5}",
            label: "Onboarding",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E4}",
            label: "Bulk Import",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4D6}",
            label: "Knowledge Base",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F916}",
            label: "Agent Console",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C8}",
            label: "Dashboard",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4D8}",
            label: "Wiki",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Governance",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4DC}",
            label: "Credentials",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1FA99}",
            label: "Token Manager",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3C6}",
            label: "Awards",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C5}",
            label: "Gantt Chart",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{23F1}",
            label: "Timeline",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C6}",
            label: "Calendar",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C1}",
            label: "Document Management",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F465}",
            label: "Resource Report",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{23F1}",
            label: "Time Tracking",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F5F3}",
            label: "Voting",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{26A0}",
            label: "Risk Register",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2637}",
            label: "Task List",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F41B}",
            label: "Issues",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4BC}",
            label: "Asset Manager",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3B0}",
            label: "Bounties",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2699}",
            label: "Automation",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C9}",
            label: "Analytics",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C5}",
            label: "Events",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4F0}",
            label: "News",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4BC}",
            label: "Portfolio",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F517}",
            label: "Integrations",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F501}",
            label: "Retrospective",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Evaluate Deontic Contract",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2696}",
            label: "Jural Relations Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CB}",
            label: "Breach Log",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2705}",
            label: "Grant Consent",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B3}",
            label: "Send Payment",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4B3}",
            label: "Receive Payment",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4F0}",
            label: "Tax Suite Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{26A1}",
            label: "Compute Cost Receipts",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3E5}",
            label: "Health Overview",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4D0}",
            label: "Clinical Calculators",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9EA}",
            label: "Compound Evidence Explorer",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3E5}",
            label: "Conditions",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CB}",
            label: "Clinical Reports",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9EA}",
            label: "Lab Results",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F48A}",
            label: "Medications",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F49A}",
            label: "Vitals",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9E0}",
            label: "Mental Wellbeing",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4AC}",
            label: "Therapy Notes",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4A4}",
            label: "Sleep",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F354}",
            label: "Diet",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3C3}",
            label: "Physical Activity",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F489}",
            label: "Immunizations",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1FA7A}",
            label: "Procedures",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F46A}",
            label: "Family History",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9EC}",
            label: "Hypotheses",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F575}",
            label: "Biometrics",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C1}",
            label: "Health Documents",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F91D}",
            label: "Welfare Support",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4DC}",
            label: "Life Records",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4DC}",
            label: "Authority Attestations",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F6E1}",
            label: "Safeguards",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F441}",
            label: "Disclosure Log",
            shortcut: "",
        },
        // Studio manifold
        CommandEntry {
            icon: "\u{1F3A8}",
            label: "Scene View",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{23F1}",
            label: "Animation Timeline",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3A7}",
            label: "Desk Surface",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{23F5}",
            label: "Transport",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F39F}",
            label: "Routing Matrix",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F30A}",
            label: "Spatial Audio",
            shortcut: "",
        },
        // Datasets manifold
        CommandEntry {
            icon: "\u{1F4CA}",
            label: "Dataset Registry",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E5}",
            label: "Dataset Importer",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C4}",
            label: "Presentation Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F5BC}",
            label: "View Canvas",
            shortcut: "",
        },
        // Studio P1
        CommandEntry {
            icon: "\u{1F4CD}",
            label: "Scene Graph",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3A8}",
            label: "Material Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4A1}",
            label: "Lighting Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F321}",
            label: "Tensor Inspector",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4C1}",
            label: "Asset Library",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F3A7}",
            label: "Channel Strip",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4CA}",
            label: "Meter Bridge",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{23F1}",
            label: "Automation Lanes",
            shortcut: "",
        },
        // Datasets P1
        CommandEntry {
            icon: "\u{1F4DD}",
            label: "Annotation Panel",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F5C8}",
            label: "Lineage Graph",
            shortcut: "",
        },
        // Studio P2
        CommandEntry {
            icon: "\u{1F4CF}",
            label: "LOD Chain",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4A1}",
            label: "Shadow Settings",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F5FA}",
            label: "GIS Maps",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F9B6}",
            label: "Ragdoll / Skin",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E4}",
            label: "Animation Export",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4BE}",
            label: "Desk Persistence",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F442}",
            label: "HRTF Personalization",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F309}",
            label: "Manifold Transition Audio",
            shortcut: "",
        },
        // Datasets P2
        CommandEntry {
            icon: "\u{1F3AC}",
            label: "Video View",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E3}",
            label: "Presentation Publish",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F50D}",
            label: "Super-Resolve Curation",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4D0}",
            label: "CAD Curation",
            shortcut: "",
        },
        // Ontology Workbench P0
        CommandEntry {
            icon: "\u{1F5FA}",
            label: "Semantic Graph Canvas",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4D6}",
            label: "Ontology Library",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4D5}",
            label: "Vocabulary Mapper",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F517}",
            label: "Relation Builder",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2705}",
            label: "SHACL Shapes",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4DD}",
            label: "N3 Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4E6}",
            label: "ShEx Editor",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{21C4}",
            label: "Ontology Compare",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{2611}",
            label: "Project Ontology Selector",
            shortcut: "",
        },
        // Device Workbench P0
        CommandEntry {
            icon: "\u{1F5A5}",
            label: "Device Manager",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4FA}",
            label: "Display Layout",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F504}",
            label: "Workspace Sync",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4F2}",
            label: "Device Role Assigner",
            shortcut: "",
        },
        CommandEntry {
            icon: "\u{1F4F1}",
            label: "Remote Control",
            shortcut: "",
        },
    ]
}
