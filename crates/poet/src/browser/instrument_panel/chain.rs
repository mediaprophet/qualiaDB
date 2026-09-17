//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Tool-chain activation on a focused container or manifold.

use web_sys::Document;

use super::ribbon::RibbonTool;

/// Activate a tool-chain on the currently focused surface (container or manifold).
/// If a container is selected, the chain's tools appear in the instrument panel for that container.
/// If no container is selected, the chain's tools appear as manifold-level tools.
pub fn activate_chain(document: &Document, chain_id: &str) {
    let tools = tools_for_chain(chain_id);
    if tools.is_empty() {
        return;
    }

    super::panel::hide(document);

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("contextual-instrument-panel");
    panel.set_attribute("data-chain-id", chain_id).unwrap();

    // Context label — shows which chain is active
    let label = document.create_element("span").unwrap();
    label.set_class_name("instrument-panel-context-label");
    label.set_text_content(Some(&format!(
        "\u{2630} {} \u{2192} focused surface",
        chain_label(chain_id)
    )));
    panel.append_child(&label).unwrap();

    // Tool buttons
    for tool in &tools {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("instrument-panel-tool-btn");
        btn.set_attribute("data-tool", tool.id).unwrap();
        btn.set_attribute("title", tool.description).unwrap();
        super::panel::configure_tool_button(&btn, tool);

        let icon = document.create_element("span").unwrap();
        icon.set_class_name("instrument-panel-tool-icon");
        icon.set_text_content(Some(tool.icon));
        btn.append_child(&icon).unwrap();

        let label = document.create_element("span").unwrap();
        label.set_class_name("instrument-panel-tool-label");
        label.set_text_content(Some(tool.label));
        btn.append_child(&label).unwrap();

        panel.append_child(&btn).unwrap();
    }

    // Close button
    let close = document.create_element("button").unwrap();
    close.set_class_name("instrument-panel-close-btn");
    close.set_text_content(Some("\u{2715}"));
    panel.append_child(&close).unwrap();

    // Insert instrument panel between control bar and workspace
    if let Some(workspace) = document.query_selector(".main-workspace").unwrap() {
        workspace
            .parent_element()
            .unwrap()
            .insert_before(&panel, Some(&workspace))
            .unwrap();
    }

    super::panel::wire_instrument_panel(document);
}

/// Activate a tool-chain on a specific container (via drag-drop).
/// This selects the container and shows the chain's tools in the instrument panel.
pub fn activate_chain_on_container(document: &Document, chain_id: &str) {
    // Find the selected container
    let selected = document
        .query_selector(".canvas-container-node.selected")
        .unwrap();
    let container_type = if let Some(ref el) = selected {
        el.get_attribute("data-container-type").unwrap_or_default()
    } else {
        String::new()
    };

    let tools = if container_type.is_empty() {
        tools_for_chain(chain_id)
    } else {
        // Merge container-type tools with chain tools
        let mut t = super::catalog::tools_for_type(&container_type);
        t.extend(tools_for_chain(chain_id));
        t
    };

    if tools.is_empty() {
        return;
    }

    super::panel::hide(document);

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("contextual-instrument-panel");
    panel.set_attribute("data-chain-id", chain_id).unwrap();
    if !container_type.is_empty() {
        panel
            .set_attribute("data-container-type", &container_type)
            .unwrap();
    }

    // Context label
    let label = document.create_element("span").unwrap();
    label.set_class_name("instrument-panel-context-label");
    if container_type.is_empty() {
        label.set_text_content(Some(&format!(
            "\u{2630} {} \u{2192} manifold",
            chain_label(chain_id)
        )));
    } else {
        label.set_text_content(Some(&format!(
            "\u{2630} {} \u{2192} {}",
            chain_label(chain_id),
            container_type
        )));
    }
    panel.append_child(&label).unwrap();

    // Tool buttons
    for tool in &tools {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("instrument-panel-tool-btn");
        btn.set_attribute("data-tool", tool.id).unwrap();
        btn.set_attribute("title", tool.description).unwrap();
        super::panel::configure_tool_button(&btn, tool);

        let icon = document.create_element("span").unwrap();
        icon.set_class_name("instrument-panel-tool-icon");
        icon.set_text_content(Some(tool.icon));
        btn.append_child(&icon).unwrap();

        let label = document.create_element("span").unwrap();
        label.set_class_name("instrument-panel-tool-label");
        label.set_text_content(Some(tool.label));
        btn.append_child(&label).unwrap();

        panel.append_child(&btn).unwrap();
    }

    // Close button
    let close = document.create_element("button").unwrap();
    close.set_class_name("instrument-panel-close-btn");
    close.set_text_content(Some("\u{2715}"));
    panel.append_child(&close).unwrap();

    if let Some(workspace) = document.query_selector(".main-workspace").unwrap() {
        workspace
            .parent_element()
            .unwrap()
            .insert_before(&panel, Some(&workspace))
            .unwrap();
    }

    super::panel::wire_instrument_panel(document);
}

/// Deactivate the current tool-chain (clear the instrument panel).
pub fn deactivate_chain(document: &Document) {
    super::panel::hide(document);
}

/// Get a human-readable label for a chain id.
fn chain_label(chain_id: &str) -> &str {
    match chain_id {
        "epistemic:modalities" => "Epistemic Modalities",
        "office:containers" => "Office Containers",
        "image:tools" => "Image Tools",
        "sheet:tools" => "Sheet Tools",
        "spatial:tools" => "Spatial Tools",
        "comm:containers" => "Communication Containers",
        "rights:tools" => "Rights Tools",
        "health:tools" => "Health Tools",
        "code:tools" => "Code Tools",
        "ai:tools" => "AI Tools",
        _ => chain_id,
    }
}

/// Get tools for a specific tool-chain id.
fn tools_for_chain(chain_id: &str) -> Vec<RibbonTool> {
    match chain_id {
        "epistemic:modalities" => vec![
            RibbonTool {
                id: "epi:objective",
                icon: "\u{1F52C}",
                label: "Objective",
                description: "Tag as objective",
            },
            RibbonTool {
                id: "epi:subjective",
                icon: "\u{1F9E0}",
                label: "Subjective",
                description: "Tag as subjective",
            },
            RibbonTool {
                id: "epi:inter",
                icon: "\u{1F91D}",
                label: "Intersubj.",
                description: "Tag as intersubjective",
            },
            RibbonTool {
                id: "epi:normative",
                icon: "\u{2696}",
                label: "Normative",
                description: "Tag as normative",
            },
        ],
        "office:containers" => vec![
            RibbonTool {
                id: "office:doc",
                icon: "\u{1F4C4}",
                label: "+ Doc",
                description: "Place document container",
            },
            RibbonTool {
                id: "office:ont",
                icon: "\u{1F4D6}",
                label: "+ Ontology",
                description: "Place ontology node",
            },
            RibbonTool {
                id: "office:slide",
                icon: "\u{1F4CA}",
                label: "+ Slide",
                description: "Place slide",
            },
        ],
        "image:tools" => vec![
            RibbonTool {
                id: "img:media",
                icon: "\u{1F3A8}",
                label: "+ Media",
                description: "Place media container",
            },
            RibbonTool {
                id: "img:marker",
                icon: "\u{1F4CD}",
                label: "Marker",
                description: "Draw marker",
            },
            RibbonTool {
                id: "img:heatmap",
                icon: "\u{1F525}",
                label: "Heatmap",
                description: "Spectral heatmap",
            },
        ],
        "sheet:tools" => vec![
            RibbonTool {
                id: "sheet:place",
                icon: "\u{1F4CA}",
                label: "+ Sheet",
                description: "Place sheet",
            },
            RibbonTool {
                id: "sheet:import",
                icon: "\u{21E9}",
                label: "Import",
                description: "Import CSV/CBOR",
            },
        ],
        "spatial:tools" => vec![
            RibbonTool {
                id: "spatial:map",
                icon: "\u{1F5FA}",
                label: "+ Map",
                description: "Place map",
            },
            RibbonTool {
                id: "spatial:3d",
                icon: "\u{1F3AF}",
                label: "+ 3D",
                description: "Place 3D viewport",
            },
            RibbonTool {
                id: "spatial:pin",
                icon: "\u{1F4CC}",
                label: "Pin",
                description: "Place pin",
            },
            RibbonTool {
                id: "spatial:track",
                icon: "\u{1F50D}",
                label: "Track",
                description: "Add track",
            },
        ],
        "comm:containers" => vec![
            RibbonTool {
                id: "comm:social",
                icon: "\u{1F4AC}",
                label: "+ Social",
                description: "Place social graph",
            },
            RibbonTool {
                id: "comm:webrtc",
                icon: "\u{1F4F7}",
                label: "+ WebRTC",
                description: "Place WebRTC",
            },
            RibbonTool {
                id: "comm:webview",
                icon: "\u{1F310}",
                label: "+ Webview",
                description: "Place webview",
            },
        ],
        "rights:tools" => vec![
            RibbonTool {
                id: "rights:group",
                icon: "\u{1F465}",
                label: "Authors",
                description: "Authors group",
            },
            RibbonTool {
                id: "rights:sign",
                icon: "\u{270D}",
                label: "Fiduciary",
                description: "Fiduciary sign",
            },
            RibbonTool {
                id: "rights:did",
                icon: "\u{1F194}",
                label: "DID",
                description: "DID sign",
            },
        ],
        "health:tools" => vec![
            RibbonTool {
                id: "health:place",
                icon: "\u{1FA7A}",
                label: "+ Health",
                description: "Place health container",
            },
            RibbonTool {
                id: "health:path",
                icon: "\u{1F52C}",
                label: "Pathology",
                description: "Pathology",
            },
            RibbonTool {
                id: "health:anat",
                icon: "\u{1F9B2}",
                label: "10D Anatomy",
                description: "10D anatomy",
            },
        ],
        "code:tools" => vec![
            RibbonTool {
                id: "code:vibe",
                icon: "\u{1F4BB}",
                label: "+ Vibe",
                description: "Place Vibe cell",
            },
            RibbonTool {
                id: "code:quin",
                icon: "\u{1F9EC}",
                label: "quin.statement",
                description: "Insert quin.statement",
            },
        ],
        "ai:tools" => vec![
            RibbonTool {
                id: "ai:coauthor",
                icon: "\u{1F9D1}",
                label: "Co-Author",
                description: "Co-author agent",
            },
            RibbonTool {
                id: "ai:extractor",
                icon: "\u{26CF}",
                label: "Extractor",
                description: "Extractor agent",
            },
            RibbonTool {
                id: "ai:sentinel",
                icon: "\u{1F6E1}",
                label: "Sentinel",
                description: "Sentinel guard",
            },
            RibbonTool {
                id: "ai:triad",
                icon: "\u{1F3A8}",
                label: "Triad",
                description: "Triad q42/p64/d10",
            },
        ],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::{chain_label, tools_for_chain};

    #[test]
    fn epistemic_chain_has_a_readable_label() {
        assert_eq!(chain_label("epistemic:modalities"), "Epistemic Modalities");
    }

    #[test]
    fn unknown_chain_has_no_tools() {
        assert!(tools_for_chain("not-a-chain").is_empty());
    }
}
