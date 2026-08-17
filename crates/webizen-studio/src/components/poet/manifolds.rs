//! Seed desks from `C:\Projects\NLP\Canvas_Workbench\manifolds\`.

use super::kinds::{
    CanvasNode, ContainerKind, Epistemic, ManifoldId, Strata, Wire,
};

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
        ManifoldId::Media => media(),
        ManifoldId::Social => social(),
        ManifoldId::Settings => settings(),
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
                "container-map-01",
                ContainerKind::Map,
                "Geospatial & Spatiotemporal Catchment Map",
                80.0,
                70.0,
                440.0,
                310.0,
                0.0,
                1.1,
                Strata::Environmental,
                Epistemic::Objective,
            ),
            node(
                "container-health-01",
                ContainerKind::Health,
                "Bio-Acoustic & Health Telemetry",
                550.0,
                70.0,
                420.0,
                310.0,
                0.0,
                1.0,
                Strata::Environmental,
                Epistemic::Objective,
            ),
            node(
                "container-ontology-01",
                ContainerKind::Ontology,
                "Hydrology ↔ Water-Rights Alignment",
                1000.0,
                70.0,
                380.0,
                310.0,
                200.0,
                1.2,
                Strata::Legal,
                Epistemic::Normative,
            ),
        ],
        wires: vec![
            wire(
                "wire-r1",
                "container-map-01",
                "container-ontology-01",
                "objective",
                "geo:triggersLegalRight",
            ),
            wire(
                "wire-r2",
                "container-health-01",
                "container-ontology-01",
                "objective",
                "health:groundsBioTelemetry",
            ),
        ],
    }
}

fn media() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Media,
        title: ManifoldId::Media.title(),
        graph_iri: ManifoldId::Media.graph_iri(),
        strata: vec![Strata::Social, Strata::Technical],
        nodes: vec![
            node(
                "container-3d-01",
                ContainerKind::Mesh3d,
                "3D Virtual Vocal Tract Kinematics (.d10)",
                100.0,
                80.0,
                400.0,
                290.0,
                100.0,
                1.3,
                Strata::Social,
                Epistemic::Objective,
            ),
            node(
                "container-media-01",
                ContainerKind::Media,
                "Visual Grapheme & Artwork Layer",
                540.0,
                80.0,
                380.0,
                290.0,
                100.0,
                1.0,
                Strata::Social,
                Epistemic::Subjective,
            ),
            node(
                "container-sheet-01",
                ContainerKind::Sheet,
                "EnCodec P64 Neural Acoustic Latents",
                950.0,
                80.0,
                420.0,
                280.0,
                0.0,
                1.0,
                Strata::Technical,
                Epistemic::Objective,
            ),
        ],
        wires: vec![
            wire(
                "wire-m1",
                "container-sheet-01",
                "container-3d-01",
                "tensor",
                "p64:drivesKinematics",
            ),
            wire(
                "wire-m2",
                "container-3d-01",
                "container-media-01",
                "subjective",
                "qualia:articulatesVisualGrapheme",
            ),
        ],
    }
}

fn social() -> ManifoldSeed {
    ManifoldSeed {
        id: ManifoldId::Social,
        title: ManifoldId::Social.title(),
        graph_iri: ManifoldId::Social.graph_iri(),
        strata: vec![Strata::Social, Strata::Legal],
        nodes: vec![
            node(
                "container-social-01",
                ContainerKind::Social,
                "Project Team & AI Sub-Agent Chat Graph",
                100.0,
                70.0,
                420.0,
                320.0,
                100.0,
                1.0,
                Strata::Social,
                Epistemic::Intersubjective,
            ),
            node(
                "container-webrtc-01",
                ContainerKind::WebRtc,
                "Encrypted WebRTC Audio/Video Stream",
                550.0,
                70.0,
                380.0,
                290.0,
                100.0,
                1.1,
                Strata::Social,
                Epistemic::Intersubjective,
            ),
            node(
                "container-doc-01",
                ContainerKind::Doc,
                "Catchment Field Notes & Observation Record",
                960.0,
                70.0,
                400.0,
                280.0,
                100.0,
                1.0,
                Strata::Social,
                Epistemic::Subjective,
            ),
        ],
        wires: vec![
            wire(
                "wire-s1",
                "container-social-01",
                "container-webrtc-01",
                "social-link",
                "qualia:transmitsMedia",
            ),
            wire(
                "wire-s2",
                "container-social-01",
                "container-doc-01",
                "subjective",
                "qualia:groundsSubjectiveQualia",
            ),
        ],
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
                "VibeScript: Fiduciary Sentinel VM",
                80.0,
                70.0,
                420.0,
                310.0,
                400.0,
                1.1,
                Strata::Technical,
                Epistemic::Objective,
            ),
            node(
                "container-subcanvas-01",
                ContainerKind::Subcanvas,
                "Recursive Sub-Manifold Component (Research Manifold)",
                530.0,
                70.0,
                440.0,
                310.0,
                100.0,
                1.0,
                Strata::Technical,
                Epistemic::Objective,
            ),
            node(
                "container-portal-01",
                ContainerKind::Portal,
                "Sovereign Commons Economic Wormhole",
                1000.0,
                70.0,
                360.0,
                260.0,
                300.0,
                1.5,
                Strata::Financial,
                Epistemic::Intersubjective,
            ),
        ],
        wires: vec![wire(
            "wire-st1",
            "container-code-01",
            "container-subcanvas-01",
            "data-pipe",
            "vibe:orchestratesSubCanvas",
        )],
    }
}
