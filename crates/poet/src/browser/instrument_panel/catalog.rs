//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Instrument-panel tools keyed by selected container type.

use super::ribbon::RibbonTool;

pub(super) fn tools_for_type(container_type: &str) -> Vec<RibbonTool> {
    match container_type {
        "doc" => vec![
            RibbonTool {
                id: "doc:bold",
                icon: "B",
                label: "Bold",
                description: "Bold selected text",
            },
            RibbonTool {
                id: "doc:italic",
                icon: "I",
                label: "Italic",
                description: "Italicize selected text",
            },
            RibbonTool {
                id: "doc:code",
                icon: "</>",
                label: "Code",
                description: "Format as code",
            },
            RibbonTool {
                id: "doc:entity",
                icon: "\u{1F3F7}",
                label: "Entity",
                description: "Tag as RDF entity <q-entity>",
            },
            RibbonTool {
                id: "doc:objective",
                icon: "\u{1F52C}",
                label: "Objective",
                description: "Tag as objective epistemic modality",
            },
            RibbonTool {
                id: "doc:subjective",
                icon: "\u{1F9E0}",
                label: "Subjective",
                description: "Tag as subjective qualia",
            },
            RibbonTool {
                id: "doc:view-md",
                icon: "MD",
                label: "Markdown",
                description: "Switch to markdown source view",
            },
            RibbonTool {
                id: "doc:view-rdf",
                icon: "RDF",
                label: "Triples",
                description: "Switch to RDF triples view",
            },
        ],
        "sheet" => vec![
            RibbonTool {
                id: "sheet:fx",
                icon: "fx",
                label: "Formula",
                description: "Formula bar",
            },
            RibbonTool {
                id: "sheet:sum",
                icon: "\u{03A3}",
                label: "Sum",
                description: "Sum selected cells",
            },
            RibbonTool {
                id: "sheet:avg",
                icon: "\u{00B5}",
                label: "Average",
                description: "Average selected cells",
            },
            RibbonTool {
                id: "sheet:p64",
                icon: "P64",
                label: "Latent",
                description: "EnCodec P64 neural latent",
            },
            RibbonTool {
                id: "sheet:chart",
                icon: "\u{1F4CA}",
                label: "Chart",
                description: "Insert chart",
            },
        ],
        "dual_studio" | "scene_view" => vec![
            RibbonTool {
                id: "scene:create",
                icon: "SCN",
                label: "Scene.create",
                description: "Create a Scene session on the daemon",
            },
            RibbonTool {
                id: "render:gpu_adapter",
                icon: "GPU",
                label: "GPU adapter",
                description: "Query Render.gpu_adapter_info",
            },
            RibbonTool {
                id: "audio:transport_play",
                icon: "\u{25B6}",
                label: "Play",
                description: "Audio.transport play",
            },
            RibbonTool {
                id: "audio:transport_stop",
                icon: "\u{23F9}",
                label: "Stop",
                description: "Audio.transport stop",
            },
        ],
        "health_overview" | "health_calculators" | "health_documents" | "disclosure_log"
        | "conditions" => {
            vec![RibbonTool {
                id: "health:nlp_ingest",
                icon: "NLP",
                label: "NLP ingest",
                description: "Run nlp.analyze + gazetteer + Semantic Library ingest on pasted text",
            }]
        }
        "audio_session" => vec![
            RibbonTool {
                id: "audio:transport_play",
                icon: "\u{25B6}",
                label: "Play",
                description: "Audio.transport play",
            },
            RibbonTool {
                id: "audio:transport_stop",
                icon: "\u{23F9}",
                label: "Stop",
                description: "Audio.transport stop",
            },
            RibbonTool {
                id: "audio:oscillator",
                icon: "Hz",
                label: "Oscillator",
                description: "Audio.oscillator 440 Hz sine",
            },
        ],
        "code" => vec![
            RibbonTool {
                id: "code:run",
                icon: "\u{25B6}",
                label: "Run",
                description: "Run VibeScript",
            },
            RibbonTool {
                id: "code:ast",
                icon: "AST",
                label: "AST",
                description: "Homoiconic AST inspector",
            },
            RibbonTool {
                id: "code:gas",
                icon: "\u{26FD}",
                label: "Gas",
                description: "Gas accounting",
            },
            RibbonTool {
                id: "code:pulse",
                icon: "\u{1F4A3}",
                label: "pulse::emit",
                description: "Insert pulse::emit",
            },
            RibbonTool {
                id: "code:cap",
                icon: "\u{1F511}",
                label: "capability.invoke",
                description: "Insert Poet.manifold_create (author a lens / container / subject)",
            },
        ],
        "ontology" => vec![
            RibbonTool {
                id: "ont:add-row",
                icon: "+",
                label: "Add Alignment",
                description: "Add alignment row",
            },
            RibbonTool {
                id: "ont:shacl",
                icon: "\u{2705}",
                label: "SHACL",
                description: "Validate SHACL shapes",
            },
            RibbonTool {
                id: "ont:classes",
                icon: "\u{1F3DB}",
                label: "Classes",
                description: "Browse class declarations",
            },
            RibbonTool {
                id: "ont:export",
                icon: "\u{1F4E4}",
                label: "Export",
                description: "Export ontology as TTL",
            },
        ],
        "map" => vec![
            RibbonTool {
                id: "map:pin",
                icon: "\u{1F4CD}",
                label: "Pin",
                description: "Place incident pin",
            },
            RibbonTool {
                id: "map:track",
                icon: "\u{1F50D}",
                label: "Track",
                description: "Add UAV track",
            },
            RibbonTool {
                id: "map:flow",
                icon: "\u{1F4A7}",
                label: "Flow",
                description: "Toggle flow layer",
            },
            RibbonTool {
                id: "map:trail",
                icon: "\u{1F6F6}",
                label: "Trail",
                description: "Toggle trail layer",
            },
        ],
        "social" => vec![
            RibbonTool {
                id: "social:connect",
                icon: "\u{1F91D}",
                label: "Connect",
                description: "Send connection request",
            },
            RibbonTool {
                id: "social:chat",
                icon: "\u{1F4AC}",
                label: "Chat",
                description: "New chat session",
            },
            RibbonTool {
                id: "social:agent",
                icon: "\u{1F916}",
                label: "Agent",
                description: "Add AI sub-agent",
            },
            RibbonTool {
                id: "social:graph",
                icon: "\u{1F578}",
                label: "Graph",
                description: "View chat graph",
            },
        ],
        "graph" => vec![
            RibbonTool {
                id: "graph:sparql",
                icon: "\u{1F50D}",
                label: "SPARQL",
                description: "Run SPARQL query",
            },
            RibbonTool {
                id: "graph:expand",
                icon: "\u{1F504}",
                label: "Expand",
                description: "Expand node neighbors",
            },
            RibbonTool {
                id: "graph:collapse",
                icon: "\u{1F4E5}",
                label: "Collapse",
                description: "Collapse node",
            },
            RibbonTool {
                id: "graph:layout",
                icon: "\u{1F4D0}",
                label: "Layout",
                description: "Auto-layout graph",
            },
        ],
        "media" | "3d" => vec![
            RibbonTool {
                id: "3d:orbit",
                icon: "\u{1F504}",
                label: "Orbit",
                description: "Orbit camera",
            },
            RibbonTool {
                id: "3d:pan",
                icon: "\u{270B}",
                label: "Pan",
                description: "Pan camera",
            },
            RibbonTool {
                id: "3d:zoom",
                icon: "\u{1F50D}",
                label: "Zoom",
                description: "Zoom to fit",
            },
            RibbonTool {
                id: "3d:wireframe",
                icon: "\u{1F9F1}",
                label: "Wireframe",
                description: "Toggle wireframe",
            },
        ],
        "health" => vec![
            RibbonTool {
                id: "health:biomarker",
                icon: "\u{1F52C}",
                label: "Biomarkers",
                description: "Pathology biomarker table",
            },
            RibbonTool {
                id: "health:tomography",
                icon: "\u{1F3A4}",
                label: "Tomography",
                description: "Spectral acoustic tomography",
            },
            RibbonTool {
                id: "health:anatomy",
                icon: "\u{1F9B2}",
                label: "Anatomy",
                description: "10D vocal tract resonator",
            },
        ],
        "webrtc" => vec![
            RibbonTool {
                id: "webrtc:mic",
                icon: "\u{1F3A4}",
                label: "Mic",
                description: "Toggle microphone",
            },
            RibbonTool {
                id: "webrtc:cam",
                icon: "\u{1F4F7}",
                label: "Cam",
                description: "Toggle camera",
            },
            RibbonTool {
                id: "webrtc:share",
                icon: "\u{1F4BB}",
                label: "Share",
                description: "Screen share",
            },
        ],
        "webview" => vec![
            RibbonTool {
                id: "webview:back",
                icon: "\u{2B05}",
                label: "Back",
                description: "Navigate back",
            },
            RibbonTool {
                id: "webview:forward",
                icon: "\u{27A1}",
                label: "Forward",
                description: "Navigate forward",
            },
            RibbonTool {
                id: "webview:reload",
                icon: "\u{1F503}",
                label: "Reload",
                description: "Reload page",
            },
            RibbonTool {
                id: "webview:clip",
                icon: "\u{2702}",
                label: "Clip RDF",
                description: "Extract RDF from page",
            },
        ],
        "rights" => vec![
            RibbonTool {
                id: "rights:sign",
                icon: "\u{270D}",
                label: "Sign",
                description: "DID sign document",
            },
            RibbonTool {
                id: "rights:audit",
                icon: "\u{1F4DC}",
                label: "Audit",
                description: "Audit trail",
            },
            RibbonTool {
                id: "rights:consent",
                icon: "\u{2705}",
                label: "Consent",
                description: "Set consent",
            },
        ],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::tools_for_type;

    #[test]
    fn doc_container_exposes_bold() {
        assert!(tools_for_type("doc")
            .iter()
            .any(|tool| tool.id == "doc:bold"));
    }

    #[test]
    fn unknown_container_has_no_ribbon() {
        assert!(tools_for_type("not-a-registered-type").is_empty());
    }
}
