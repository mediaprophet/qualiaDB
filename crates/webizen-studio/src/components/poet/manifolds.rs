//! Seed desks from `C:\Projects\NLP\Canvas_Workbench\manifolds\` and `POET-SPEC-002`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::kinds::{CanvasNode, ContainerKind, Epistemic, ManifoldId, Strata, Wire};

pub struct ManifoldSeed {
    pub id: ManifoldId,
    pub title: &'static str,
    pub graph_iri: &'static str,
    pub strata: Vec<Strata>,
    pub nodes: Vec<CanvasNode>,
    pub wires: Vec<Wire>,
}

pub fn load_manifold(id: ManifoldId) -> ManifoldSeed {
    match id {
        ManifoldId::Research => research(),
        ManifoldId::Social => social(),
        ManifoldId::Knowledge => knowledge(),
        ManifoldId::Projects => projects(),
        ManifoldId::Rights => rights(),
        ManifoldId::Sanctuary => sanctuary(),
        ManifoldId::Media => media(),
        ManifoldId::Communications => communications(),
        ManifoldId::Settings => settings(),
        ManifoldId::Vibe => vibe(),
    }
}

fn node(
    id: &str,
    kind: ContainerKind,
    title: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    z: f64,
    d: f64,
    strata: Strata,
    epistemic: Epistemic,
) -> CanvasNode {
    CanvasNode {
        id: id.into(),
        kind,
        title: title.into(),
        x,
        y,
        width: w,
        height: h,
        z,
        d,
        strata,
        epistemic,
    }
}

fn wire(id: &str, from: &str, to: &str, kind: &str, label: &str) -> Wire {
    Wire {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        kind: kind.into(),
        label: label.into(),
    }
}

fn research() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Research,
        title: ManifoldId::Research.title(),
        graph_iri: ManifoldId::Research.graph_iri(),
        strata: vec![
            Strata::Environmental,
            Strata::Social,
            Strata::Legal,
            Strata::Technical,
        ],
        nodes: vec![
            node(
                "container-doc-01",
                ContainerKind::Doc,
                "North Spring Catchment Dossier & CML HyperDoc",
                60.0,
                70.0,
                460.0,
                340.0,
                0.0,
                1.0,
                Strata::Environmental,
                Epistemic::Objective,
            ),
            node(
                "container-map-01",
                ContainerKind::Map,
                "Geospatial & Spatiotemporal Catchment Map",
                540.0,
                70.0,
                440.0,
                340.0,
                0.0,
                1.1,
                Strata::Environmental,
                Epistemic::Objective,
            ),
            node(
                "container-health-01",
                ContainerKind::Health,
                "Bio-Acoustic & Health Telemetry",
                1000.0,
                70.0,
                420.0,
                340.0,
                0.0,
                1.0,
                Strata::Environmental,
                Epistemic::Objective,
            ),
        ],
        wires: vec![
            wire(
                "wire-r1",
                "container-doc-01",
                "container-map-01",
                "epistemic-link",
                "qualia:groundsGeospatialObservation",
            ),
            wire(
                "wire-r2",
                "container-map-01",
                "container-health-01",
                "data-pipe",
                "qualia:streamsTelemetryFeed",
            ),
        ],
    }
}

fn media() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Media,
        title: ManifoldId::Media.title(),
        graph_iri: ManifoldId::Media.graph_iri(),
        strata: vec![Strata::Environmental, Strata::Social, Strata::Technical],
        nodes: vec![
            node(
                "container-mesh-01",
                ContainerKind::Mesh3d,
                "CCF Anatomical Heart 3D Mesh (.10d)",
                80.0,
                70.0,
                460.0,
                340.0,
                0.0,
                1.2,
                Strata::Technical,
                Epistemic::Objective,
            ),
            node(
                "container-media-01",
                ContainerKind::Media,
                "EnCodec P64 Acoustic Spectrogram & Audio Studio",
                560.0,
                70.0,
                440.0,
                340.0,
                0.0,
                1.0,
                Strata::Social,
                Epistemic::Subjective,
            ),
        ],
        wires: vec![wire(
            "wire-m1",
            "container-mesh-01",
            "container-media-01",
            "cross-modal",
            "qualia:synchronizesKinematics",
        )],
    }
}

fn social() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Social,
        title: ManifoldId::Social.title(),
        graph_iri: ManifoldId::Social.graph_iri(),
        strata: vec![Strata::Social, Strata::Legal, Strata::Financial],
        nodes: vec![
            node(
                "container-kanban-01",
                ContainerKind::ErpKanban,
                "Cooperative ERP & Workstream A Kanban",
                80.0,
                70.0,
                480.0,
                340.0,
                100.0,
                1.0,
                Strata::Social,
                Epistemic::Intersubjective,
            ),
            node(
                "container-social-01",
                ContainerKind::Social,
                "Project Team & AI Sub-Agent Chat Graph",
                580.0,
                70.0,
                420.0,
                340.0,
                100.0,
                1.0,
                Strata::Social,
                Epistemic::Intersubjective,
            ),
            node(
                "container-webrtc-01",
                ContainerKind::WebRtc,
                "Encrypted WebRTC P2P Data Mesh",
                1020.0,
                70.0,
                380.0,
                340.0,
                100.0,
                1.1,
                Strata::Social,
                Epistemic::Intersubjective,
            ),
        ],
        wires: vec![
            wire(
                "wire-s1",
                "container-kanban-01",
                "container-social-01",
                "social-link",
                "qualia:coordinatesDeliverable",
            ),
            wire(
                "wire-s2",
                "container-social-01",
                "container-webrtc-01",
                "data-pipe",
                "qualia:transmitsMedia",
            ),
        ],
    }
}

fn knowledge() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Knowledge,
        title: ManifoldId::Knowledge.title(),
        graph_iri: ManifoldId::Knowledge.graph_iri(),
        strata: vec![Strata::Environmental, Strata::Social, Strata::Technical],
        nodes: vec![
            node(
                "container-ont-01",
                ContainerKind::Ontology,
                "SHACL / RDF-Star Knowledge Graph & Ontology Browser",
                80.0,
                70.0,
                480.0,
                340.0,
                0.0,
                1.0,
                Strata::Technical,
                Epistemic::Objective,
            ),
            node(
                "container-solid-01",
                ContainerKind::SolidHub,
                "W3C Solid Pod LDP Storage & Type Indexes",
                580.0,
                70.0,
                440.0,
                340.0,
                0.0,
                1.0,
                Strata::Technical,
                Epistemic::Objective,
            ),
        ],
        wires: vec![wire(
            "wire-ont-1",
            "container-ont-01",
            "container-solid-01",
            "data-pipe",
            "qualia:exportsSolidLdp",
        )],
    }
}

fn projects() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Projects,
        title: ManifoldId::Projects.title(),
        graph_iri: ManifoldId::Projects.graph_iri(),
        strata: vec![Strata::Social, Strata::Financial, Strata::Technical],
        nodes: vec![
            node(
                "container-kanban-01",
                ContainerKind::ErpKanban,
                "Poet Agile Sprint Board & Task Tracker",
                80.0,
                70.0,
                460.0,
                340.0,
                0.0,
                1.0,
                Strata::Social,
                Epistemic::Intersubjective,
            ),
            node(
                "container-git-02",
                ContainerKind::GitForge,
                "Git Pull Requests & Change Pipeline",
                560.0,
                70.0,
                440.0,
                340.0,
                0.0,
                1.0,
                Strata::Technical,
                Epistemic::Objective,
            ),
        ],
        wires: vec![wire(
            "wire-prj-1",
            "container-kanban-01",
            "container-git-02",
            "data-pipe",
            "qualia:tracksDelivery",
        )],
    }
}

fn rights() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Rights,
        title: ManifoldId::Rights.title(),
        graph_iri: ManifoldId::Rights.graph_iri(),
        strata: vec![Strata::Legal, Strata::Social, Strata::Technical],
        nodes: vec![
            node(
                "container-rights-01",
                ContainerKind::Doc,
                "Fiduciary Custody L3 & Inalienable WebID Agreement",
                80.0,
                70.0,
                480.0,
                340.0,
                0.0,
                1.0,
                Strata::Legal,
                Epistemic::Normative,
            ),
            node(
                "container-econ-01",
                ContainerKind::Economics,
                "Cooperative Micro-Commons & Resource Allocation",
                580.0,
                70.0,
                440.0,
                340.0,
                0.0,
                1.0,
                Strata::Financial,
                Epistemic::Intersubjective,
            ),
        ],
        wires: vec![wire(
            "wire-rg-1",
            "container-rights-01",
            "container-econ-01",
            "social-link",
            "qualia:allocatesRightsBudget",
        )],
    }
}

fn sanctuary() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Sanctuary,
        title: ManifoldId::Sanctuary.title(),
        graph_iri: ManifoldId::Sanctuary.graph_iri(),
        strata: vec![Strata::Environmental, Strata::Legal, Strata::Technical],
        nodes: vec![
            node(
                "container-sanctuary-doc",
                ContainerKind::Doc,
                "Sanctuary Intimacy Vault (Zero-Telemetry Airgap)",
                80.0,
                70.0,
                480.0,
                340.0,
                0.0,
                1.0,
                Strata::Legal,
                Epistemic::Subjective,
            ),
            node(
                "container-sanctuary-health",
                ContainerKind::Health,
                "Private Biometric & Holistic Health Archive",
                580.0,
                70.0,
                440.0,
                340.0,
                0.0,
                1.0,
                Strata::Social,
                Epistemic::Subjective,
            ),
        ],
        wires: vec![wire(
            "wire-sanc-1",
            "container-sanctuary-doc",
            "container-sanctuary-health",
            "data-pipe",
            "qualia:vaultEncrypted",
        )],
    }
}

fn communications() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Communications,
        title: ManifoldId::Communications.title(),
        graph_iri: ManifoldId::Communications.graph_iri(),
        strata: vec![Strata::Social, Strata::Legal, Strata::Technical],
        nodes: vec![
            node(
                "container-mail-01",
                ContainerKind::Mail,
                "Inalienable Domain Inboxes (inquiry@, research@)",
                80.0,
                70.0,
                480.0,
                340.0,
                100.0,
                1.0,
                Strata::Social,
                Epistemic::Intersubjective,
            ),
            node(
                "container-doc-mail",
                ContainerKind::Doc,
                "CML Mail Composer & DID Attestation Signer",
                580.0,
                70.0,
                440.0,
                340.0,
                100.0,
                1.0,
                Strata::Legal,
                Epistemic::Normative,
            ),
        ],
        wires: vec![wire(
            "wire-mail-1",
            "container-mail-01",
            "container-doc-mail",
            "data-pipe",
            "qualia:composesReply",
        )],
    }
}

fn vibe() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Vibe,
        title: ManifoldId::Vibe.title(),
        graph_iri: ManifoldId::Vibe.graph_iri(),
        strata: vec![Strata::Technical, Strata::Environmental],
        nodes: vec![
            node(
                "container-vibe-ide",
                ContainerKind::Code,
                "VibeScript Live Compiler & Reactive Visual Cell",
                80.0,
                70.0,
                520.0,
                360.0,
                0.0,
                1.0,
                Strata::Technical,
                Epistemic::Objective,
            ),
            node(
                "container-vibe-shaders",
                ContainerKind::Shaders,
                "WGSL Forge & GPU Shader Pipelines",
                620.0,
                70.0,
                440.0,
                360.0,
                0.0,
                1.0,
                Strata::Technical,
                Epistemic::Objective,
            ),
        ],
        wires: vec![wire(
            "wire-vibe-1",
            "container-vibe-ide",
            "container-vibe-shaders",
            "data-pipe",
            "vibe:compilesShaderKernel",
        )],
    }
}

fn settings() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Settings,
        title: ManifoldId::Settings.title(),
        graph_iri: ManifoldId::Settings.graph_iri(),
        strata: vec![Strata::Technical, Strata::Financial, Strata::Legal],
        nodes: vec![
            node(
                "container-code-01",
                ContainerKind::Code,
                "VibeScript IDE & Fiduciary Sentinel VM",
                80.0,
                70.0,
                460.0,
                340.0,
                400.0,
                1.1,
                Strata::Technical,
                Epistemic::Objective,
            ),
            node(
                "container-git-01",
                ContainerKind::GitForge,
                "Distributed Git Forge & P2P Swarm Remotes",
                560.0,
                70.0,
                460.0,
                340.0,
                100.0,
                1.0,
                Strata::Technical,
                Epistemic::Objective,
            ),
            node(
                "container-subcanvas-01",
                ContainerKind::Subcanvas,
                "Recursive Sub-Manifold & Inalienable Commons Hub",
                1040.0,
                70.0,
                400.0,
                340.0,
                300.0,
                1.5,
                Strata::Financial,
                Epistemic::Intersubjective,
            ),
        ],
        wires: vec![wire(
            "wire-st1",
            "container-code-01",
            "container-git-01",
            "data-pipe",
            "vibe:versionsProjectArtifacts",
        )],
    }
}
