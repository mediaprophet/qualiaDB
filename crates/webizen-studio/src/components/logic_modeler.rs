use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
struct OntologicalDefinition {
    id: String,
    friendly_name: String,
    logical_name: String, // e.g., "q42:hasGuardian" or "shacl:NodeShape"
    description: String,
    kind: String, // "concept", "predicate", "constraint", "rule"
}

#[derive(Clone, Debug, PartialEq)]
struct LogicNode {
    id: usize,
    x: f64,
    y: f64,
    def: OntologicalDefinition,
}

#[derive(Clone, Debug, PartialEq)]
struct LogicEdge {
    id: usize,
    from: usize,
    to: usize,
    def: Option<OntologicalDefinition>, // The predicate defining the edge
    status: String,                     // "valid", "contradiction", "pending"
}

fn get_toolbox_items() -> Vec<OntologicalDefinition> {
    vec![
        OntologicalDefinition {
            id: "entity".into(),
            friendly_name: "Entity / Class".into(),
            logical_name: "rdfs:Class".into(),
            description: "A generic class or entity in the knowledge graph.".into(),
            kind: "concept".into(),
        },
        OntologicalDefinition {
            id: "user".into(),
            friendly_name: "Webizen User".into(),
            logical_name: "q42:Webizen".into(),
            description: "A sovereign user entity in the QualiaDB network.".into(),
            kind: "concept".into(),
        },
        OntologicalDefinition {
            id: "hasGuardian".into(),
            friendly_name: "Has Guardian".into(),
            logical_name: "q42:hasGuardian".into(),
            description: "Deontic relation specifying a guardianship connection.".into(),
            kind: "predicate".into(),
        },
        OntologicalDefinition {
            id: "subClassOf".into(),
            friendly_name: "Subclass Of".into(),
            logical_name: "rdfs:subClassOf".into(),
            description: "Taxonomic relation indicating inheritance.".into(),
            kind: "predicate".into(),
        },
        OntologicalDefinition {
            id: "shaclNode".into(),
            friendly_name: "SHACL Shape".into(),
            logical_name: "shacl:NodeShape".into(),
            description: "Defines constraints for a specific node type.".into(),
            kind: "constraint".into(),
        },
        OntologicalDefinition {
            id: "n3Defeater".into(),
            friendly_name: "N3 Defeater Rule".into(),
            logical_name: "n3:defeater".into(),
            description: "A rule that actively defeats a matching defeasible rule.".into(),
            kind: "rule".into(),
        },
    ]
}

#[component]
pub fn LogicModeler() -> Element {
    let mut nodes = use_signal(|| {
        let items = get_toolbox_items();
        vec![
            LogicNode {
                id: 1,
                x: 300.0,
                y: 150.0,
                def: items[1].clone(),
            }, // User
            LogicNode {
                id: 2,
                x: 700.0,
                y: 150.0,
                def: items[0].clone(),
            }, // Entity
        ]
    });

    let mut edges = use_signal(|| {
        let items = get_toolbox_items();
        vec![LogicEdge {
            id: 101,
            from: 1,
            to: 2,
            def: Some(items[3].clone()),
            status: "valid".into(),
        }]
    });

    let toolbox_items = use_signal(get_toolbox_items);

    // Drag and drop state for nodes
    let mut dragging_node = use_signal(|| None::<usize>);

    // Wiring state (drawing an edge)
    let mut drawing_edge_from = use_signal(|| None::<usize>);
    let mut cursor_pos = use_signal(|| (0.0, 0.0));

    // Selected item for the Inspector panel
    let mut selected_node = use_signal(|| None::<usize>);
    let mut selected_edge = use_signal(|| None::<usize>);

    let mut next_id = use_signal(|| 1000);

    let handle_mouse_move = move |evt: Event<MouseData>| {
        let coords = evt.client_coordinates();
        cursor_pos.set((coords.x, coords.y));

        if let Some(id) = dragging_node.read().clone() {
            let mut current_nodes = nodes.read().clone();
            if let Some(node) = current_nodes.iter_mut().find(|n| n.id == id) {
                // Offset calculation (approximate since we don't track initial click delta here for brevity)
                node.x = coords.x - 300.0;
                node.y = coords.y - 100.0;
            }
            nodes.set(current_nodes);
        }
    };

    let handle_mouse_up = move |_: Event<MouseData>| {
        dragging_node.set(None);
        drawing_edge_from.set(None); // Drop wire if released in empty space
    };

    let mut start_wiring = move |evt: Event<MouseData>, node_id: usize| {
        evt.stop_propagation();
        drawing_edge_from.set(Some(node_id));
        selected_node.set(Some(node_id));
        selected_edge.set(None);
    };

    let mut finish_wiring = move |evt: Event<MouseData>, target_id: usize| {
        evt.stop_propagation();
        if let Some(from_id) = drawing_edge_from.read().clone() {
            if from_id != target_id {
                let mut e = edges.read().clone();
                let new_id = next_id.read().clone();
                next_id.set(new_id + 1);

                e.push(LogicEdge {
                    id: new_id,
                    from: from_id,
                    to: target_id,
                    def: None, // Starts as generic untyped connection
                    status: "pending".into(),
                });
                edges.set(e);
            }
        }
        drawing_edge_from.set(None);
    };

    let mut select_edge = move |evt: Event<MouseData>, edge_id: usize| {
        evt.stop_propagation();
        selected_edge.set(Some(edge_id));
        selected_node.set(None);
    };

    // For applying a dragged toolbox predicate to an edge
    let mut apply_predicate_to_edge = move |edge_id: usize, def: OntologicalDefinition| {
        let mut e = edges.read().clone();
        if let Some(edge) = e.iter_mut().find(|ed| ed.id == edge_id) {
            edge.def = Some(def);
        }
        edges.set(e);
    };

    let mut add_node_from_toolbox = move |def: OntologicalDefinition| {
        let mut n = nodes.read().clone();
        let new_id = next_id.read().clone();
        next_id.set(new_id + 1);
        n.push(LogicNode {
            id: new_id,
            x: 400.0, // Drop in center roughly
            y: 300.0,
            def,
        });
        nodes.set(n);
    };

    let evaluate_logic = move |_| {
        let mut e = edges.read().clone();
        for edge in e.iter_mut() {
            // Simple mock logic: if it's untyped, it's a contradiction.
            if edge.def.is_none() {
                edge.status = "contradiction".into();
            } else {
                edge.status = "valid".into();
            }
        }
        edges.set(e);
    };

    rsx! {
        div {
            style: "position: relative; flex: 1; min-height: 800px; display: flex; background: radial-gradient(circle at center, #1a1a24 0%, #08080c 100%); overflow: hidden; font-family: 'Inter', sans-serif; color: #E0E0FF;",
            onmousemove: handle_mouse_move,
            onmouseup: handle_mouse_up,
            onmouseleave: handle_mouse_up,

            // Sidebar: Toolbox
            div {
                style: "width: 280px; background: rgba(20, 20, 30, 0.85); backdrop-filter: blur(20px); border-right: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; z-index: 10; padding: 20px; box-shadow: 5px 0 20px rgba(0,0,0,0.5);",
                h2 {
                    style: "margin: 0 0 20px 0; font-weight: 700; font-size: 1.1rem; color: #fff; text-transform: uppercase; letter-spacing: 1px;",
                    "Ontology Toolbox"
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 12px; overflow-y: auto;",
                    for item in toolbox_items.read().iter() {
                        {
                            let item_clone = item.clone();
                            let bg = match item.kind.as_str() {
                                "concept" => "rgba(0, 184, 255, 0.1)",
                                "predicate" => "rgba(255, 100, 0, 0.1)",
                                "constraint" => "rgba(255, 0, 127, 0.1)",
                                "rule" => "rgba(138, 43, 226, 0.1)",
                                _ => "rgba(255,255,255,0.1)"
                            };
                            let border = match item.kind.as_str() {
                                "concept" => "rgba(0, 184, 255, 0.4)",
                                "predicate" => "rgba(255, 100, 0, 0.4)",
                                "constraint" => "rgba(255, 0, 127, 0.4)",
                                "rule" => "rgba(138, 43, 226, 0.4)",
                                _ => "rgba(255,255,255,0.4)"
                            };

                            rsx! {
                                div {
                                    key: "{item.id}",
                                    style: "padding: 12px; border-radius: 8px; background: {bg}; border: 1px solid {border}; cursor: pointer; transition: transform 0.2s, box-shadow 0.2s;",
                                    onclick: move |_| {
                                        if item_clone.kind == "predicate" {
                                            if let Some(edge_id) = selected_edge.read().clone() {
                                                apply_predicate_to_edge(edge_id, item_clone.clone());
                                            }
                                        } else {
                                            add_node_from_toolbox(item_clone.clone());
                                        }
                                    },
                                    div { style: "font-size: 0.7rem; text-transform: uppercase; color: {border}; font-weight: 700; margin-bottom: 4px;", "{item.kind}" }
                                    div { style: "font-weight: 600; font-size: 0.95rem; margin-bottom: 2px;", "{item.friendly_name}" }
                                    div { style: "font-family: 'Fira Code', monospace; font-size: 0.75rem; color: #888;", "{item.logical_name}" }
                                }
                            }
                        }
                    }
                }

                div {
                    style: "margin-top: auto; padding-top: 20px;",
                    button {
                        style: "width: 100%; padding: 12px; background: linear-gradient(135deg, #FF007F, #8A2BE2); color: white; border: none; border-radius: 8px; font-weight: 700; cursor: pointer; box-shadow: 0 4px 15px rgba(138, 43, 226, 0.4);",
                        onclick: evaluate_logic,
                        "Evaluate Logic Matrix"
                    }
                }
            }

            // Canvas Area
            div {
                style: "flex: 1; position: relative;",

                // SVG Layer for Edges
                svg {
                    style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; z-index: 1;",

                    {
                        edges.read().clone().into_iter().filter_map(|edge| {
                            if let (Some(n1), Some(n2)) = (nodes.read().iter().find(|n| n.id == edge.from).cloned(), nodes.read().iter().find(|n| n.id == edge.to).cloned()) {
                                let is_selected = selected_edge.read().map_or(false, |id| id == edge.id);
                                let color = match edge.status.as_str() {
                                    "contradiction" => "#FF007F",
                                    "valid" => "#00FF88",
                                    _ => "#00B8FF"
                                };
                                let stroke_w = if is_selected { "4" } else { "2" };
                                let dash = if edge.status == "contradiction" { "6,6" } else { "none" };
                                let start_x = n1.x + 100.0;
                                let start_y = n1.y + 40.0;
                                let end_x = n2.x - 10.0; // Point to left side port
                                let end_y = n2.y + 40.0;
                                let label_text = edge.def.as_ref().map(|d| d.friendly_name.clone()).unwrap_or_else(|| "untyped (click to set)".into());

                                Some(rsx! {
                                    g {
                                        style: "pointer-events: bounding-box; cursor: pointer;",
                                        onclick: move |e| select_edge(e, edge.id),
                                        path {
                                            d: "M {start_x} {start_y} C {start_x + 80.0} {start_y}, {end_x - 80.0} {end_y}, {end_x} {end_y}",
                                            fill: "none",
                                            stroke: "{color}",
                                            stroke_width: "{stroke_w}",
                                            stroke_dasharray: "{dash}",
                                            style: "filter: drop-shadow(0 0 5px {color}); transition: all 0.3s ease;",
                                        }
                                        rect {
                                            x: (start_x + end_x) / 2.0 - 60.0,
                                            y: (start_y + end_y) / 2.0 - 12.0,
                                            width: 120.0,
                                            height: 24.0,
                                            fill: "rgba(20,20,30,0.9)",
                                            rx: 4.0,
                                            stroke: "{color}",
                                            stroke_width: "1"
                                        }
                                        text {
                                            x: (start_x + end_x) / 2.0,
                                            y: (start_y + end_y) / 2.0 + 4.0,
                                            fill: "#FFF",
                                            font_size: "11px",
                                            font_weight: "600",
                                            text_anchor: "middle",
                                            "{label_text}"
                                        }
                                    }
                                })
                            } else {
                                None
                            }
                        })
                    }

                    // Render drawing edge
                    {
                        let mut result = None;
                        if let Some(from_id) = drawing_edge_from.read().clone() {
                            if let Some(n1) = nodes.read().iter().find(|n| n.id == from_id) {
                                let start_x = n1.x + 100.0;
                                let start_y = n1.y + 40.0;
                                // Need to offset cursor based on sidebar width roughly (280px)
                                let end_x = cursor_pos.read().0 - 280.0;
                                let end_y = cursor_pos.read().1;

                                result = Some(rsx! {
                                    path {
                                        d: "M {start_x} {start_y} C {start_x + 80.0} {start_y}, {end_x - 80.0} {end_y}, {end_x} {end_y}",
                                        fill: "none",
                                        stroke: "rgba(255,255,255,0.5)",
                                        stroke_width: "2",
                                        stroke_dasharray: "4,4",
                                    }
                                });
                            }
                        }
                        result
                    }
                }

                // Render Nodes
                {
                    nodes.read().clone().into_iter().map(|node| {
                        let is_selected = selected_node.read().map_or(false, |id| id == node.id);
                        let border_glow = if is_selected { "0 0 20px rgba(0, 184, 255, 0.8)" } else { "0 8px 32px rgba(0,0,0,0.3)" };
                        let bg_color = match node.def.kind.as_str() {
                            "concept" => "rgba(0, 184, 255, 0.1)",
                            "rule" => "rgba(138, 43, 226, 0.15)",
                            "constraint" => "rgba(255, 0, 127, 0.1)",
                            _ => "rgba(255, 255, 255, 0.05)"
                        };
                        let border_color = match node.def.kind.as_str() {
                            "concept" => "rgba(0, 184, 255, 0.5)",
                            "rule" => "rgba(138, 43, 226, 0.6)",
                            "constraint" => "rgba(255, 0, 127, 0.6)",
                            _ => "rgba(255, 255, 255, 0.2)"
                        };

                        rsx! {
                            div {
                                key: "{node.id}",
                                style: "position: absolute; left: {node.x}px; top: {node.y}px; width: 200px; padding: 16px; background: {bg_color}; backdrop-filter: blur(12px); border: 1px solid {border_color}; border-radius: 12px; cursor: grab; z-index: 2; transition: transform 0.1s, box-shadow 0.2s; box-shadow: {border_glow};",
                                onmousedown: move |_| {
                                    selected_node.set(Some(node.id));
                                    selected_edge.set(None);
                                    dragging_node.set(Some(node.id));
                                },

                                // Incoming Port
                                div {
                                    style: "position: absolute; left: -8px; top: 32px; width: 16px; height: 16px; background: #222; border: 2px solid {border_color}; border-radius: 50%; cursor: crosshair; z-index: 3;",
                                    onmouseup: move |e| finish_wiring(e, node.id),
                                }

                                // Outgoing Port
                                div {
                                    style: "position: absolute; right: -8px; top: 32px; width: 16px; height: 16px; background: {border_color}; border: 2px solid #222; border-radius: 50%; cursor: crosshair; z-index: 3; box-shadow: 0 0 8px {border_color};",
                                    onmousedown: move |e| start_wiring(e, node.id),
                                }

                                div {
                                    style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 1px; color: {border_color}; font-weight: 700; margin-bottom: 8px;",
                                    "{node.def.kind}"
                                }
                                div {
                                    style: "color: #FFFFFF; font-weight: 600; font-size: 1.1rem; line-height: 1.2;",
                                    "{node.def.friendly_name}"
                                }
                                div {
                                    style: "margin-top: 6px; font-family: 'Fira Code', monospace; font-size: 0.7rem; color: rgba(255,255,255,0.5); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                    "{node.def.logical_name}"
                                }
                            }
                        }
                    })
                }

                // Right Inspector Panel
                if selected_node.read().is_some() || selected_edge.read().is_some() {
                    div {
                        style: "position: absolute; right: 20px; top: 20px; width: 300px; background: rgba(15, 15, 20, 0.9); backdrop-filter: blur(20px); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; padding: 20px; z-index: 10; box-shadow: 0 10px 30px rgba(0,0,0,0.5);",
                        h3 { style: "margin: 0 0 16px 0; font-size: 1rem; color: #fff;", "Inspector" }

                        if let Some(id) = selected_node.read().clone() {
                            if let Some(node) = nodes.read().iter().find(|n| n.id == id) {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 12px;",
                                    div { style: "color: #00B8FF; font-weight: 700; font-size: 0.8rem; text-transform: uppercase;", "Node Definition" }
                                    div { style: "font-size: 1.2rem; font-weight: 600;", "{node.def.friendly_name}" }
                                    div { style: "font-family: 'Fira Code', monospace; font-size: 0.85rem; color: #A0A0C0; padding: 8px; background: #000; border-radius: 6px;", "{node.def.logical_name}" }
                                    div { style: "font-size: 0.9rem; color: #D0D0E0; line-height: 1.5;", "{node.def.description}" }
                                }
                            }
                        } else if let Some(id) = selected_edge.read().clone() {
                            if let Some(edge) = edges.read().iter().find(|e| e.id == id) {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 12px;",
                                    div { style: "color: #FF007F; font-weight: 700; font-size: 0.8rem; text-transform: uppercase;", "Edge / Predicate" }
                                    if let Some(def) = &edge.def {
                                        div { style: "font-size: 1.2rem; font-weight: 600;", "{def.friendly_name}" }
                                        div { style: "font-family: 'Fira Code', monospace; font-size: 0.85rem; color: #A0A0C0; padding: 8px; background: #000; border-radius: 6px;", "{def.logical_name}" }
                                        div { style: "font-size: 0.9rem; color: #D0D0E0; line-height: 1.5;", "{def.description}" }
                                    } else {
                                        div { style: "font-size: 0.9rem; color: #FF007F;", "Untyped edge! Select a predicate from the toolbox to apply it." }
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
