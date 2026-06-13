use dioxus::prelude::*;
use crate::Route;

// ── Category ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
enum Cat {
    All, Platform, Ai, Knowledge, Scientific,
    Quantum, Medical, Financial, Security, Data, Network, Developer,
}

impl Cat {
    fn label(self) -> &'static str {
        match self {
            Cat::All       => "All",
            Cat::Platform  => "Platform",
            Cat::Ai        => "AI & Inference",
            Cat::Knowledge => "Knowledge",
            Cat::Scientific => "Scientific",
            Cat::Quantum   => "Quantum",
            Cat::Medical   => "Medical",
            Cat::Financial => "Financial",
            Cat::Security  => "Security",
            Cat::Data      => "Data",
            Cat::Network   => "Network",
            Cat::Developer => "Dev Tools",
        }
    }
}

fn cat_list() -> Vec<Cat> {
    vec![
        Cat::All, Cat::Platform, Cat::Ai, Cat::Knowledge, Cat::Scientific,
        Cat::Quantum, Cat::Medical, Cat::Financial, Cat::Security,
        Cat::Data, Cat::Network, Cat::Developer,
    ]
}

// ── App model ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Stat { Active, Beta, Soon }

#[derive(Clone, Copy, PartialEq)]
enum AppRoute { ContextStudio, QAppStudio, Nexus }

struct QApp {
    id: &'static str,
    name: &'static str,
    tagline: &'static str,
    desc: &'static str,
    icon: &'static str,
    route: Option<AppRoute>,
    stat: Stat,
    cat: Cat,
}

// ── Template model ────────────────────────────────────────────────────────────

struct Template {
    name: &'static str,
    desc: &'static str,
    icons: Vec<&'static str>,
}

fn featured_templates() -> Vec<Template> {
    vec![
        Template {
            name: "Scientific Research Bench",
            desc: "Physics, chemistry, ODE solver, statistics, and matrix lab in one composable workspace.",
            icons: vec!["lightning-charge", "droplet", "activity", "bar-chart-line", "grid-3x3"],
        },
        Template {
            name: "Personal Knowledge Hub",
            desc: "Semantic graph, ontology builder, SPARQL console, N3 logic editor, and Solid LDP browser.",
            icons: vec!["diagram-3", "node-plus", "code-slash", "braces", "folder-symlink"],
        },
        Template {
            name: "Clinical Decision Support",
            desc: "Health vitals, clinical risk scoring, DICOM viewer, and comorbidity analysis.",
            icons: vec!["heart-pulse", "clipboard2-pulse", "image-alt", "shield-plus"],
        },
        Template {
            name: "Quantum Finance Lab",
            desc: "Portfolio analyser, QPU optimiser, GBM simulator, and VaR risk engine.",
            icons: vec!["currency-exchange", "cpu", "shuffle", "graph-up-arrow"],
        },
        Template {
            name: "Governance Console",
            desc: "Agreements & rights, deontic logic editor, SHACL validator, ZK proofs, and key vault.",
            icons: vec!["file-earmark-check", "journal-text", "check2-all", "eye-slash", "key"],
        },
        Template {
            name: "AI Research Bench",
            desc: "LLM harness, LoRA adapter manager, neuro-symbolic chat, and MCP tool inspector.",
            icons: vec!["cpu-fill", "layers-half", "chat-dots", "plugin"],
        },
    ]
}

// ── Full app catalog ──────────────────────────────────────────────────────────

fn qapp_catalog() -> Vec<QApp> {
    vec![
        // ── Platform ──────────────────────────────────────────────────────────
        QApp {
            id: "context-studio", name: "Context Studio", tagline: "Semantic Workspace",
            desc: "Node-graph canvas with Selfhood/Personhood zone segregation, Inforg assistant, \
                   Commons Gateway, Contextual Lenses, and temporal scrubber for AS OF queries over the live NQuin graph.",
            icon: "diagram-3", route: Some(AppRoute::ContextStudio), stat: Stat::Active, cat: Cat::Platform,
        },
        QApp {
            id: "qapp-studio", name: "QApp Studio", tagline: "Layout Builder",
            desc: "Drag-and-drop Shoelace + Qualia pane composer. Arrange, resize, and wire panes \
                   into custom dashboards — output is a signed QApp manifest written to the WAL.",
            icon: "layers", route: Some(AppRoute::QAppStudio), stat: Stat::Active, cat: Cat::Platform,
        },
        QApp {
            id: "profile-identity", name: "Profile & Identity", tagline: "DID Management",
            desc: "Manage did:q42 identifiers, ed25519 keypairs, Verifiable Credentials, and \
                   Principal-scoped capability grants via key_vault, profiles, and identifier modules.",
            icon: "person-vcard", route: None, stat: Stat::Beta, cat: Cat::Platform,
        },
        QApp {
            id: "hardware-config", name: "Hardware Configurator", tagline: "Device Management",
            desc: "Configure GPU backend (DirectML / Vulkan / Metal / WebGPU), ZNS/NVMe storage \
                   zones, thermal governor thresholds, QPU provider credentials, and NPU FFI bindings.",
            icon: "tools", route: None, stat: Stat::Beta, cat: Cat::Platform,
        },
        QApp {
            id: "notification-center", name: "Notification Center", tagline: "Alerts & Events",
            desc: "Unified event stream from WAL mutations, deontic violations, QPU job completions, \
                   and governance alerts — each surfaced as a signed NQuin notification quin.",
            icon: "bell", route: None, stat: Stat::Soon, cat: Cat::Platform,
        },

        // ── AI & Inference ────────────────────────────────────────────────────
        QApp {
            id: "chat", name: "Neuro-Symbolic Chat", tagline: "Conversational AI",
            desc: "Phase 8 bifurcated LLM with real-time Webizen Sentinel oversight. \
                   LogitStream → ControlStream ring buffers gate every token; all output requires ≥1 NQuin provenance citation.",
            icon: "chat-dots", route: None, stat: Stat::Beta, cat: Cat::Ai,
        },
        QApp {
            id: "llm-harness", name: "LLM Model Harness", tagline: "Model Testing",
            desc: "Load GGUF models via memmap2, run autoregressive inference on the wgpu shader, \
                   inspect logit vectors, measure throughput, and compare quantisation tiers (Q4_K_M vs Q8_0).",
            icon: "cpu-fill", route: None, stat: Stat::Beta, cat: Cat::Ai,
        },
        QApp {
            id: "lora-manager", name: "LoRA Adapter Manager", tagline: "Neural Adaptation",
            desc: "Zero-copy LoRA multiplexing: load adapters from NQuin bits 63–48, blend up to 8 \
                   adapters per token via the fused WGSL shader, and manage the LRU adapter cache.",
            icon: "layers-half", route: None, stat: Stat::Beta, cat: Cat::Ai,
        },
        QApp {
            id: "agent-config", name: "Agent Configuration", tagline: "Inference Runtime",
            desc: "Configure AgentBackend (Local / Remote / Hybrid), ModelLifecycle state machine, \
                   128 MB RAM cap enforcement, and Nym mixnet routing for Remote-mode API calls.",
            icon: "robot", route: None, stat: Stat::Beta, cat: Cat::Ai,
        },
        QApp {
            id: "inference-monitor", name: "Inference Monitor", tagline: "Real-time Telemetry",
            desc: "Live dashboard for ThermalGovernor readings, VRAM utilisation, tokens/sec \
                   throughput, Sentinel anomaly events, and DenyRollback injection counts per session.",
            icon: "activity", route: None, stat: Stat::Beta, cat: Cat::Ai,
        },
        QApp {
            id: "model-lifecycle", name: "Model Lifecycle", tagline: "Model Management",
            desc: "Download, cache, swap, and retire GGUF models. Verify SHA-256 checksums, manage \
                   resident-model LRU policy, and track version compatibility with the LoRA adapter registry.",
            icon: "arrow-repeat", route: None, stat: Stat::Beta, cat: Cat::Ai,
        },

        // ── Knowledge & Semantics ─────────────────────────────────────────────
        QApp {
            id: "ontology-builder", name: "Ontology Builder", tagline: "Knowledge Engineering",
            desc: "Interactive ontology workbench: define SHACL shapes, link OWL concepts, compile \
                   N3 rules to SlgOpcode bytecode, and seed finished ontologies to the WebTorrent DHT as .c.q42 artifacts.",
            icon: "node-plus", route: None, stat: Stat::Beta, cat: Cat::Knowledge,
        },
        QApp {
            id: "sparql-explorer", name: "SPARQL Explorer", tagline: "Graph Queries",
            desc: "Federated SPARQL console over the local Qualia daemon (port 4242). Supports AS OF \
                   temporal queries, GeoSPARQL spatial filters, SHACL validation, and multi-modal results.",
            icon: "code-slash", route: None, stat: Stat::Beta, cat: Cat::Knowledge,
        },
        QApp {
            id: "n3-logic-studio", name: "N3 Logic Studio", tagline: "Rule Engineering",
            desc: "Author and test N3Logic rules that feed the Webizen VM. Compile to SlgOpcode \
                   bytecode, run against the 42 MB SLG Arena, and inspect derivation traces step-by-step.",
            icon: "braces", route: None, stat: Stat::Beta, cat: Cat::Knowledge,
        },
        QApp {
            id: "rdf-star-editor", name: "RDF-Star Editor", tagline: "Triple Annotations",
            desc: "Edit and query RDF-Star nested triples. Maps annotations to NQuin provenance bits; \
                   supports PROV-O, DC Terms, and ODRL metadata inline on any statement.",
            icon: "diagram-2", route: None, stat: Stat::Beta, cat: Cat::Knowledge,
        },
        QApp {
            id: "solid-browser", name: "Solid LDP Browser", tagline: "Solid Protocol",
            desc: "Browse Solid Pods via the local LDP proxy. Read/write Turtle and JSON-LD resources, \
                   manage WebACL permissions, and import pod data into the Q42 semantic graph.",
            icon: "folder-symlink", route: None, stat: Stat::Beta, cat: Cat::Knowledge,
        },

        // ── Scientific Computing ──────────────────────────────────────────────
        QApp {
            id: "physics-sim", name: "Physics Simulator", tagline: "Physical Modelling",
            desc: "Clifford geometric algebra, Lorentz vectors, Voronoi tessellations, Burgers-equation \
                   CFD, and MCMC thermodynamic sampling — all zero-copy via physics_simulation and thermodynamics libraries.",
            icon: "lightning-charge", route: None, stat: Stat::Beta, cat: Cat::Scientific,
        },
        QApp {
            id: "chemistry-modeler", name: "Chemistry Modeler", tagline: "Molecular Science",
            desc: "SMILES parsing, Lipinski/Veber/Ghose drug-likeness filters, LogP/TPSA, Morgan \
                   fingerprints, functional group detection, and thermochemistry via the organic_chemistry domain engine.",
            icon: "droplet", route: None, stat: Stat::Beta, cat: Cat::Scientific,
        },
        QApp {
            id: "ode-lab", name: "ODE & Calculus Lab", tagline: "Numerical Methods",
            desc: "Runge-Kutta 4th-order integrator, shooting-method BVP solver, Simpson quadrature, \
                   and symbolic ODE parsing — each result stamped with a tensor-provenance NQuin.",
            icon: "graph-up", route: None, stat: Stat::Beta, cat: Cat::Scientific,
        },
        QApp {
            id: "matrix-lab", name: "Matrix & Linear Algebra", tagline: "Numerical Computing",
            desc: "Hardware-sympathetic zero-copy matrix ops: Lanczos eigensolver, LU decomposition, \
                   tensor contraction, and CSD-accelerated dense kernels via the linear_algebra library.",
            icon: "grid-3x3", route: None, stat: Stat::Beta, cat: Cat::Scientific,
        },
        QApp {
            id: "stats-lab", name: "Statistical Analysis Lab", tagline: "Data Science",
            desc: "Privacy-preserving statistics with ML-DSA fiduciary signatures: distributions, \
                   hypothesis tests, regression, Bayesian inference, and Monte Carlo sampling.",
            icon: "bar-chart-line", route: None, stat: Stat::Beta, cat: Cat::Scientific,
        },
        QApp {
            id: "bioinformatics-lab", name: "Bioinformatics Lab", tagline: "Sequence Analysis",
            desc: "SIMD-accelerated Smith-Waterman and Needleman-Wunsch alignment, k-mer indexing, \
                   metabolite fingerprinting, and phylogenetic tree construction via the bioinformatics domain.",
            icon: "bezier2", route: None, stat: Stat::Beta, cat: Cat::Scientific,
        },

        // ── Quantum Computing ─────────────────────────────────────────────────
        QApp {
            id: "qpu-optimizer", name: "QPU Optimizer", tagline: "Quantum Optimisation",
            desc: "Formulate QUBO/QAOA problems and dispatch to 8 QPU providers (IBM / D-Wave / IonQ \
                   / Rigetti / Azure / Braket / Google / Quantinuum) via the in-process QPU dispatcher.",
            icon: "cpu", route: None, stat: Stat::Beta, cat: Cat::Quantum,
        },
        QApp {
            id: "quantum-dft", name: "Quantum DFT Lab", tagline: "Quantum Transforms",
            desc: "Discrete Fourier Transform on quantum gate circuits. Integrates with IBM Quantum \
                   API; results back-annotated onto NQuin provenance chains for full auditability.",
            icon: "soundwave", route: None, stat: Stat::Beta, cat: Cat::Quantum,
        },
        QApp {
            id: "qaoa-explorer", name: "QAOA Explorer", tagline: "Variational Algorithms",
            desc: "Interactive QAOA angle optimiser using SPSA gradient descent. Visualise energy \
                   landscapes, convergence curves, and compare classical vs. quantum solution quality.",
            icon: "sliders", route: None, stat: Stat::Beta, cat: Cat::Quantum,
        },
        QApp {
            id: "qpu-providers", name: "QPU Provider Manager", tagline: "Quantum Infrastructure",
            desc: "Manage credentials, job quotas, and connectivity for all 8 QPU backends. Monitor \
                   queue depths, gate error rates, and fidelity metrics across providers.",
            icon: "cloud-check", route: None, stat: Stat::Beta, cat: Cat::Quantum,
        },
        QApp {
            id: "nexus", name: "Nexus", tagline: "Quantum Research Cooperative",
            desc: "Living Research Timeline with LTL causal provenance, cooperative knowledge graph \
                   canvas, epistemic modal-logic claim threads (OP_KNOWS/OP_BELIEVES), native dispatch \
                   for SW alignment, DFT, MCMC, and RK4 — all attribution via DID-signed NQuins.",
            icon: "radioactive", route: Some(AppRoute::Nexus), stat: Stat::Active, cat: Cat::Quantum,
        },

        // ── Medical & Life Sciences ───────────────────────────────────────────
        QApp {
            id: "health-vitals", name: "Health Vital Monitor", tagline: "Biosignals",
            desc: "Real-time biosignal monitoring via the biosciences SHACL engine. IoT sensor \
                   ingestion, standardised ontology mapping (HL7 FHIR, SNOMED CT), and anomaly alerting.",
            icon: "heart-pulse", route: None, stat: Stat::Beta, cat: Cat::Medical,
        },
        QApp {
            id: "clinical-risk", name: "Clinical Risk Scorer", tagline: "Decision Support",
            desc: "Framingham, APACHE-II, SOFA, and custom prognosis models via clinical_engine. \
                   Gene expression evaluation, guideline cross-referencing, and a signed audit trail.",
            icon: "clipboard2-pulse", route: None, stat: Stat::Beta, cat: Cat::Medical,
        },
        QApp {
            id: "dicom-viewer", name: "DICOM Viewer", tagline: "Medical Imaging",
            desc: "DICOM file ingestion and rendering via dicom_ingest. Slice navigation, \
                   window/level adjustment, annotation overlay, and export to NQuin-tagged graph nodes.",
            icon: "image-alt", route: None, stat: Stat::Beta, cat: Cat::Medical,
        },
        QApp {
            id: "anatomy-browser", name: "Anatomy Context Browser", tagline: "Reference Atlas",
            desc: "Interactive anatomical reference powered by anatomy_context. Link structures to \
                   clinical risk scores, DICOM regions of interest, and bioinformatics datasets.",
            icon: "person-bounding-box", route: None, stat: Stat::Beta, cat: Cat::Medical,
        },
        QApp {
            id: "comorbidity", name: "Comorbidity Analyzer", tagline: "Multi-condition Risk",
            desc: "Multi-condition risk assessment via comorbidity_eval. Surfaces drug-interaction \
                   risks, contraindication flags, and population-level co-occurrence patterns.",
            icon: "shield-plus", route: None, stat: Stat::Beta, cat: Cat::Medical,
        },

        // ── Financial & Economics ─────────────────────────────────────────────
        QApp {
            id: "portfolio", name: "Portfolio Analyzer", tagline: "Asset Management",
            desc: "Markowitz optimisation, Sharpe/Sortino ratios, and factor exposure analysis with \
                   ML-DSA fiduciary signatures — all zero-copy via the financial_modeling specialized library.",
            icon: "currency-exchange", route: None, stat: Stat::Beta, cat: Cat::Financial,
        },
        QApp {
            id: "risk-engine", name: "Risk Engine", tagline: "Quantitative Risk",
            desc: "Value-at-Risk, Conditional VaR, stress testing, and Monte Carlo scenario generation. \
                   Results provenance-stamped and signed via the financial_modeling library.",
            icon: "graph-up-arrow", route: None, stat: Stat::Beta, cat: Cat::Financial,
        },
        QApp {
            id: "gbm-sim", name: "GBM Simulator", tagline: "Stochastic Modelling",
            desc: "Geometric Brownian Motion and jump-diffusion price path simulation via \
                   domains::economics. Parameterise drift, volatility, and correlation matrices interactively.",
            icon: "shuffle", route: None, stat: Stat::Beta, cat: Cat::Financial,
        },
        QApp {
            id: "tax-schema", name: "Tax Schema Editor", tagline: "Compliance",
            desc: "Define and evaluate tax rules via domains::tax_schema. Jurisdiction-specific rule \
                   trees, ODRL-linked obligation sets, and automated compliance reporting.",
            icon: "receipt", route: None, stat: Stat::Beta, cat: Cat::Financial,
        },
        QApp {
            id: "ilp-dashboard", name: "ILP Routing Dashboard", tagline: "Interledger",
            desc: "Monitor Interledger Protocol streaming micropayments via ilp_dispatcher. Track \
                   Remote inference metering and ontology seeding transactions in real time.",
            icon: "arrow-left-right", route: None, stat: Stat::Soon, cat: Cat::Financial,
        },

        // ── Security & Governance ─────────────────────────────────────────────
        QApp {
            id: "agreements", name: "Agreements & Rights", tagline: "Governance",
            desc: "ODRL policy manager for data-sharing agreements. Sign and verify fiduciary \
                   obligations using the deontic_logic engine and agency module's ed25519 Author-scoped Merkle roots.",
            icon: "file-earmark-check", route: None, stat: Stat::Beta, cat: Cat::Security,
        },
        QApp {
            id: "key-vault", name: "Key Vault Manager", tagline: "Cryptographic Keys",
            desc: "ML-DSA post-quantum keypair management, ed25519 signing, SubgraphKey \
                   (AES-GCM + HKDF) generation, X25519 ECDH encapsulation, and VC credential issuance.",
            icon: "key", route: None, stat: Stat::Beta, cat: Cat::Security,
        },
        QApp {
            id: "zk-studio", name: "ZK Proof Studio", tagline: "Privacy Proofs",
            desc: "Author, compile, and verify zero-knowledge proofs via the zk_proofs module. \
                   Privacy-preserving selective disclosure of semantic subgraph data without revealing raw quins.",
            icon: "eye-slash", route: None, stat: Stat::Beta, cat: Cat::Security,
        },
        QApp {
            id: "deontic-editor", name: "Deontic Logic Editor", tagline: "Normative Rules",
            desc: "Visual editor for N3Logic Rights Ontology rules. Author obligations, permissions, \
                   and prohibitions; evaluate them against intent graphs in the validate_intent() pre-flight gate.",
            icon: "journal-text", route: None, stat: Stat::Beta, cat: Cat::Security,
        },
        QApp {
            id: "shacl-validator", name: "SHACL Validator", tagline: "Shape Constraints",
            desc: "Compile SHACL shapes to SlgOpcodes, validate RDF graphs interactively, inspect \
                   constraint violations, and link shapes to ODRL policies for automated enforcement.",
            icon: "check2-all", route: None, stat: Stat::Beta, cat: Cat::Security,
        },
        QApp {
            id: "credential-manager", name: "Credential Manager", tagline: "Verifiable Credentials",
            desc: "Issue, hold, verify, and revoke W3C Verifiable Credentials. Manage SubgraphLayer \
                   unlock keys and Principal consent scopes for credential-gated subgraph access.",
            icon: "patch-check", route: None, stat: Stat::Soon, cat: Cat::Security,
        },

        // ── Data & Storage ────────────────────────────────────────────────────
        QApp {
            id: "wal-inspector", name: "WAL Inspector", tagline: "Write-Ahead Log",
            desc: "Browse the 32-byte WAL header, prev_dag_hash chains, buffered_count, and DagNode \
                   Merkle tree. Trigger checkpoint_to_dag() and verify ed25519 conduct-violation quins.",
            icon: "journal-code", route: None, stat: Stat::Beta, cat: Cat::Data,
        },
        QApp {
            id: "q42-volume", name: "Q42 Volume Manager", tagline: "Graph Archives",
            desc: "Manage .q42.bidx block-range index files, header-first boot partitions, OPFS \
                   auto-cache, and multi-file Q42 volume manifests with cryptographic checksums.",
            icon: "database", route: None, stat: Stat::Beta, cat: Cat::Data,
        },
        QApp {
            id: "provenance-graph", name: "Provenance Graph", tagline: "Audit Trails",
            desc: "Visualise PROV-O provenance chains over the temporal graph. Navigate derivation \
                   edges between NQuin citations, WAL entries, and Merkle-DAG checkpoints.",
            icon: "diagram-2-fill", route: None, stat: Stat::Beta, cat: Cat::Data,
        },
        QApp {
            id: "storage-config", name: "Storage Driver Config", tagline: "Storage Abstraction",
            desc: "Configure the cross-platform StorageDriver (ZnsDriver / WinNvmeDriver / \
                   MmapApfsDriver / MmapDriver). Set ZNS zone limits, mmap parameters, and WSL2 auto-detect.",
            icon: "hdd-stack", route: None, stat: Stat::Beta, cat: Cat::Data,
        },
        QApp {
            id: "crdt-sync", name: "CRDT Sync Dashboard", tagline: "Distributed Sync",
            desc: "Monitor conflict-free replicated data type convergence across Webizen nodes. \
                   Visualise merge histories, vector clocks, and delta-state gossip round-trips.",
            icon: "arrow-repeat", route: None, stat: Stat::Soon, cat: Cat::Data,
        },

        // ── Network & Distribution ────────────────────────────────────────────
        QApp {
            id: "webtorrent", name: "WebTorrent Seeder", tagline: "Ontology Distribution",
            desc: "Seed .c.q42 ontology artifacts to the WebTorrent DHT. Manage magnet links, \
                   announce to trackers, monitor peer connections, and verify integrity against NQuin hashes.",
            icon: "share", route: None, stat: Stat::Beta, cat: Cat::Network,
        },
        QApp {
            id: "p2p-dashboard", name: "P2P Node Dashboard", tagline: "Gossip Network",
            desc: "Monitor the gossip/DHT overlay: peer table, routing buckets, message throughput, \
                   and DaemonSwarm coordination. View live Webizen node topology.",
            icon: "diagram-3-fill", route: None, stat: Stat::Beta, cat: Cat::Network,
        },
        QApp {
            id: "ebpf-filter", name: "eBPF Filter Manager", tagline: "Network Control",
            desc: "Platform-aware packet filtering via open_platform_filter(): Linux eBPF, Windows WFP, \
                   macOS NEFilter/XPC. Define rules, inspect matched flows, and audit egress.",
            icon: "funnel", route: None, stat: Stat::Beta, cat: Cat::Network,
        },
        QApp {
            id: "acoustic-ble", name: "Acoustic BLE Mesh", tagline: "Zero-Infrastructure Net",
            desc: "Configure and monitor the acoustic/BLE mesh for offline-first Webizen clustering. \
                   No infrastructure required — peer discovery via acoustic and Bluetooth signals.",
            icon: "broadcast", route: None, stat: Stat::Beta, cat: Cat::Network,
        },
        QApp {
            id: "nym-gateway", name: "Nym Privacy Gateway", tagline: "Mixnet Routing",
            desc: "Route Remote inference API calls through the Nym mixnet via nym_adapter. Configure \
                   anonymity set size, latency budget, and ILP metering for privacy-preserving egress.",
            icon: "shield-lock", route: None, stat: Stat::Soon, cat: Cat::Network,
        },

        // ── Developer Tools ───────────────────────────────────────────────────
        QApp {
            id: "mcp-inspector", name: "MCP Tool Inspector", tagline: "Protocol Debugging",
            desc: "Browse, invoke, and test all 41 MCP tools from mcp_server.rs. Inspect \
                   request/response JSON, trace call latency, and verify NQuin citations in results.",
            icon: "plugin", route: None, stat: Stat::Beta, cat: Cat::Developer,
        },
        QApp {
            id: "benchmark", name: "Benchmark Harness", tagline: "Performance Testing",
            desc: "Run the benchmarks/qualia/runner.py harness against the local daemon: point / \
                   two-hop / filter query latency, graph insert throughput, and inference tokens/sec.",
            icon: "stopwatch", route: None, stat: Stat::Beta, cat: Cat::Developer,
        },
        QApp {
            id: "cli-bridge", name: "CLI Bridge", tagline: "Command Line",
            desc: "GUI wrapper over qualia-cli: ingest RDF/Turtle, run SPARQL queries, invoke \
                   solve(ode/quantum/symbolic), trigger science runners, and browse ETL pipeline state.",
            icon: "terminal", route: None, stat: Stat::Beta, cat: Cat::Developer,
        },
        QApp {
            id: "extension-bus", name: "Extension Bus", tagline: "FFI Extensions",
            desc: "Manage heavy computational extensions (QPU, PINN, SNN, fluid dynamics) via the \
                   extension_bus FFI bridge. Load, unload, and inspect extension manifests.",
            icon: "puzzle", route: None, stat: Stat::Beta, cat: Cat::Developer,
        },
        QApp {
            id: "marketplace", name: "QApp Marketplace", tagline: "Community Extensions",
            desc: "Browse and install community QApps distributed over WebTorrent. Each app is \
                   sandboxed by the Webizen VM; N3Logic permission declarations are auditable before install.",
            icon: "shop", route: None, stat: Stat::Soon, cat: Cat::Developer,
        },
    ]
}

// ── Pre-computed card data ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum BtnKind { LaunchContext, LaunchQAppStudio, LaunchNexus, OpenInStudio, ComingSoon }

struct CardData {
    id: &'static str,
    name: &'static str,
    tagline: &'static str,
    desc: &'static str,
    icon: &'static str,
    status_label: &'static str,
    status_color: &'static str,
    opacity: &'static str,
    btn: BtnKind,
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn QApps() -> Element {
    let all_apps = qapp_catalog();
    let mut selected_cat = use_signal(|| Cat::All);
    let current_cat = selected_cat();
    let cats = cat_list();

    let n_active = all_apps.iter().filter(|a| a.stat == Stat::Active).count();
    let n_beta   = all_apps.iter().filter(|a| a.stat == Stat::Beta).count();
    let n_soon   = all_apps.iter().filter(|a| a.stat == Stat::Soon).count();

    let cards: Vec<CardData> = all_apps.iter()
        .filter(|a| current_cat == Cat::All || a.cat == current_cat)
        .map(|a| {
            let (status_label, status_color, opacity) = match a.stat {
                Stat::Active => ("Active", "#10b981", "1"),
                Stat::Beta   => ("Beta",   "#f59e0b", "1"),
                Stat::Soon   => ("Soon",   "#9ca3af", "0.60"),
            };
            let btn = match (a.stat, a.route) {
                (Stat::Active, Some(AppRoute::ContextStudio)) => BtnKind::LaunchContext,
                (Stat::Active, Some(AppRoute::QAppStudio))    => BtnKind::LaunchQAppStudio,
                (Stat::Active, Some(AppRoute::Nexus))         => BtnKind::LaunchNexus,
                (Stat::Soon, _)                               => BtnKind::ComingSoon,
                _                                             => BtnKind::OpenInStudio,
            };
            CardData {
                id: a.id, name: a.name, tagline: a.tagline, desc: a.desc, icon: a.icon,
                status_label, status_color, opacity, btn,
            }
        })
        .collect();

    let templates = featured_templates();

    rsx! {
        div {
            style: "width: 100%; height: 100%; overflow-y: auto; padding: 2rem 2rem 4rem;",

            // ── Header ─────────────────────────────────────────────────────────
            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; margin-bottom: 1.25rem;",
                div {
                    h1 {
                        style: "margin: 0 0 0.25rem 0; font-size: 1.4rem; font-weight: 700; color: var(--qualia-text); letter-spacing: -0.025em;",
                        "QApps"
                    }
                    p {
                        style: "margin: 0; font-size: 0.82rem; color: var(--qualia-text-muted);",
                        "All applications running in your Webizen node — governed, provenance-tracked, and fiduciary-safe."
                    }
                }
                div { style: "display: flex; gap: 0.4rem; flex-shrink: 0; margin-top: 0.2rem;",
                    span {
                        style: "font-size: 0.69rem; font-weight: 600; color: #10b981; background: rgba(16,185,129,0.1); border: 1px solid rgba(16,185,129,0.25); border-radius: 12px; padding: 0.2rem 0.55rem;",
                        "{n_active} Active"
                    }
                    span {
                        style: "font-size: 0.69rem; font-weight: 600; color: #f59e0b; background: rgba(245,158,11,0.1); border: 1px solid rgba(245,158,11,0.25); border-radius: 12px; padding: 0.2rem 0.55rem;",
                        "{n_beta} Beta"
                    }
                    span {
                        style: "font-size: 0.69rem; font-weight: 600; color: #9ca3af; background: rgba(156,163,175,0.1); border: 1px solid rgba(156,163,175,0.25); border-radius: 12px; padding: 0.2rem 0.55rem;",
                        "{n_soon} Soon"
                    }
                }
            }

            // ── Featured Templates ──────────────────────────────────────────────
            div { style: "margin-bottom: 1.75rem;",
                div {
                    style: "font-size: 0.7rem; font-weight: 700; color: var(--qualia-text-muted); letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 0.75rem;",
                    "Featured Templates"
                }
                div {
                    style: "display: flex; gap: 0.875rem; overflow-x: auto; padding-bottom: 0.625rem; scrollbar-width: thin;",
                    for tmpl in templates.iter() {
                        div {
                            key: "{tmpl.name}",
                            class: "panel-card",
                            style: "flex-shrink: 0; width: 210px; background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 14px; padding: 1rem; backdrop-filter: blur(20px); box-shadow: 0 4px 20px rgba(0,0,0,0.06);",

                            div { style: "display: flex; gap: 0.35rem; margin-bottom: 0.7rem; flex-wrap: wrap;",
                                for icon in tmpl.icons.iter() {
                                    div {
                                        key: "{icon}",
                                        style: "width: 26px; height: 26px; border-radius: 7px; background: var(--qualia-accent-glow); display: flex; align-items: center; justify-content: center;",
                                        sl-icon { "name": "{icon}", style: "font-size: 0.75rem; color: var(--qualia-accent);" }
                                    }
                                }
                            }
                            div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.3rem; line-height: 1.3;", "{tmpl.name}" }
                            p { style: "margin: 0 0 0.7rem; font-size: 0.7rem; color: var(--qualia-text-muted); line-height: 1.45;", "{tmpl.desc}" }
                            Link {
                                to: Route::StudioRoute {},
                                style: "display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.72rem; font-weight: 600; color: var(--qualia-accent); text-decoration: none;",
                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                "Open in Studio"
                            }
                        }
                    }
                }
            }

            // ── Category filter ─────────────────────────────────────────────────
            div {
                style: "display: flex; gap: 0.375rem; margin-bottom: 1.25rem; flex-wrap: wrap;",
                for cat in cats.iter() {
                    {
                        let c = *cat;
                        let is_active = current_cat == c;
                        let bg = if is_active { "var(--qualia-accent)" } else { "rgba(128,128,128,0.08)" };
                        let col = if is_active { "white" } else { "var(--qualia-text-muted)" };
                        let border = if is_active { "var(--qualia-accent)" } else { "var(--qualia-border)" };
                        rsx! {
                            button {
                                key: "{c.label()}",
                                onclick: move |_| selected_cat.set(c),
                                style: "background: {bg}; color: {col}; border: 1px solid {border}; border-radius: 20px; padding: 0.28rem 0.7rem; font-size: 0.74rem; font-weight: 500; font-family: 'Inter', sans-serif; cursor: pointer; transition: all 0.15s; white-space: nowrap;",
                                "{c.label()}"
                            }
                        }
                    }
                }
            }

            // ── App grid ────────────────────────────────────────────────────────
            div {
                style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(288px, 1fr)); gap: 1rem;",
                for card in cards.iter() {
                    div {
                        key: "{card.id}",
                        class: "panel-card",
                        style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1.2rem; backdrop-filter: blur(24px); box-shadow: 0 6px 28px rgba(0,0,0,0.07); display: flex; flex-direction: column; gap: 0.65rem; opacity: {card.opacity};",

                        // Icon + status badge
                        div { style: "display: flex; align-items: flex-start; justify-content: space-between;",
                            div {
                                style: "width: 40px; height: 40px; border-radius: 12px; background: var(--qualia-accent-glow); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                                sl-icon { "name": "{card.icon}", style: "font-size: 1.15rem; color: var(--qualia-accent);" }
                            }
                            span {
                                style: "font-size: 0.66rem; font-weight: 600; color: {card.status_color}; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 20px; padding: 0.18rem 0.5rem; letter-spacing: 0.04em; flex-shrink: 0; margin-top: 2px;",
                                "{card.status_label}"
                            }
                        }

                        // Name + tagline
                        div {
                            div { style: "font-size: 0.875rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.1rem;", "{card.name}" }
                            div { style: "font-size: 0.69rem; color: var(--qualia-accent); font-weight: 500; letter-spacing: 0.01em;", "{card.tagline}" }
                        }

                        // Description
                        p {
                            style: "margin: 0; font-size: 0.745rem; color: var(--qualia-text-muted); line-height: 1.52; flex: 1;",
                            "{card.desc}"
                        }

                        // Action buttons
                        {
                            let app_id_str = card.id.to_string();
                            let app_id_str2 = card.id.to_string();
                            rsx! {
                                div { style: "display: flex; gap: 0.45rem; margin-top: auto; padding-top: 0.2rem; flex-wrap: wrap;",
                                    match card.btn {
                                        BtnKind::LaunchContext => rsx! {
                                            Link {
                                                to: Route::ContextStudioRoute {},
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                                "Launch"
                                            }
                                            Link {
                                                to: Route::StudioEditRoute { app_id: app_id_str },
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: rgba(128,128,128,0.1); color: var(--qualia-text-muted); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 500; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "pencil", style: "font-size: 0.68rem;" }
                                                "Edit"
                                            }
                                        },
                                        BtnKind::LaunchQAppStudio => rsx! {
                                            Link {
                                                to: Route::StudioRoute {},
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                                "Launch"
                                            }
                                        },
                                        BtnKind::LaunchNexus => rsx! {
                                            Link {
                                                to: Route::NexusRoute {},
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                                "Launch"
                                            }
                                            Link {
                                                to: Route::StudioEditRoute { app_id: app_id_str },
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: rgba(128,128,128,0.1); color: var(--qualia-text-muted); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 500; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "pencil", style: "font-size: 0.68rem;" }
                                                "Edit"
                                            }
                                        },
                                        BtnKind::OpenInStudio => rsx! {
                                            Link {
                                                to: Route::StudioEditRoute { app_id: app_id_str2 },
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "layers", style: "font-size: 0.68rem;" }
                                                "Open in Studio"
                                            }
                                        },
                                        BtnKind::ComingSoon => rsx! {
                                            button {
                                                disabled: true,
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: rgba(128,128,128,0.08); color: var(--qualia-text-muted); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 500; font-family: 'Inter', sans-serif; cursor: not-allowed;",
                                                sl-icon { "name": "clock", style: "font-size: 0.68rem;" }
                                                "Coming Soon"
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
