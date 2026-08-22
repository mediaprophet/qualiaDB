//! Semantic Graph Canvas — visual node-and-edge ontology editor (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const NODES: &[(&str, &str, f64, f64, &str)] = &[
    ("Person", "owl:Class", 15.0, 10.0, "rgba(0, 200, 255, 0.15)"),
    (
        "Organization",
        "owl:Class",
        60.0,
        10.0,
        "rgba(0, 200, 255, 0.15)",
    ),
    (
        "Document",
        "owl:Class",
        15.0,
        55.0,
        "rgba(0, 200, 255, 0.15)",
    ),
    (
        "hasMember",
        "owl:ObjectProperty",
        40.0,
        30.0,
        "rgba(255, 165, 0, 0.15)",
    ),
    (
        "authoredBy",
        "owl:ObjectProperty",
        40.0,
        70.0,
        "rgba(255, 165, 0, 0.15)",
    ),
    (
        "NaturalAgent",
        "owl:Class",
        5.0,
        40.0,
        "rgba(100, 200, 100, 0.15)",
    ),
    (
        "SoftwareAgent",
        "owl:Class",
        30.0,
        40.0,
        "rgba(100, 200, 100, 0.15)",
    ),
    (
        "name",
        "owl:DatatypeProperty",
        80.0,
        40.0,
        "rgba(200, 150, 255, 0.15)",
    ),
];

const EDGES: &[(usize, usize, usize)] = &[
    (0, 4, 5), // Person subClassOf NaturalAgent
    (0, 5, 6), // Person subClassOf SoftwareAgent (disjoint — marked)
    (3, 0, 1), // hasMember domain Person range Organization
    (4, 2, 0), // authoredBy domain Document range Person
    (7, 0, 0), // name domain Person
];

// RDFS / OWL constructs
const TOOLBOX_OWL: &[(&str, &str)] = &[
    ("Class", "owl:Class"),
    ("Named Individual", "owl:NamedIndividual"),
    ("Object Property", "owl:ObjectProperty"),
    ("Datatype Property", "owl:DatatypeProperty"),
    ("Annotation Property", "owl:AnnotationProperty"),
    ("subClassOf", "rdfs:subClassOf"),
    ("equivalentClass", "owl:equivalentClass"),
    ("inverseOf", "owl:inverseOf"),
    ("domain", "rdfs:domain"),
    ("range", "rdfs:range"),
    ("someValuesFrom", "owl:someValuesFrom"),
    ("allValuesFrom", "owl:allValuesFrom"),
    ("hasValue", "owl:hasValue"),
    ("minCardinality", "owl:minCardinality"),
    ("unionOf", "owl:unionOf"),
    ("intersectionOf", "owl:intersectionOf"),
    ("disjointWith", "owl:disjointWith"),
    ("complementOf", "owl:complementOf"),
];

// SHACL constructs — shape-based, no owl:Thing root commitment
const TOOLBOX_SHACL: &[(&str, &str)] = &[
    ("Node Shape", "sh:NodeShape"),
    ("Property Shape", "sh:PropertyShape"),
    ("targetClass", "sh:targetClass"),
    ("targetNode", "sh:targetNode"),
    ("targetSubjectsOf", "sh:targetSubjectsOf"),
    ("targetObjectsOf", "sh:targetObjectsOf"),
    ("path", "sh:path"),
    ("minCount", "sh:minCount"),
    ("maxCount", "sh:maxCount"),
    ("datatype", "sh:datatype"),
    ("nodeKind", "sh:nodeKind"),
    ("class", "sh:class"),
    ("minInclusive", "sh:minInclusive"),
    ("maxInclusive", "sh:maxInclusive"),
    ("minLength", "sh:minLength"),
    ("maxLength", "sh:maxLength"),
    ("pattern", "sh:pattern"),
    ("in", "sh:in"),
    ("hasValue", "sh:hasValue"),
    ("qualifiedValueShape", "sh:qualifiedValueShape"),
    ("severity", "sh:severity"),
    ("message", "sh:message"),
    ("deactivated", "sh:deactivated"),
    ("or", "sh:or"),
    ("and", "sh:and"),
    ("not", "sh:not"),
    ("xone", "sh:xone"),
];

// ShEx constructs — shape expressions, grammar-based validation
const TOOLBOX_SHEX: &[(&str, &str)] = &[
    ("Shape", "shex:Shape"),
    ("TripleConstraint", "shex:TripleConstraint"),
    ("EachOf", "shex:EachOf"),
    ("OneOf", "shex:OneOf"),
    ("Group", "shex:Group"),
    ("cardinality", "shex:cardinality"),
    ("min", "shex:min"),
    ("max", "shex:max"),
    ("valueExpr", "shex:valueExpr"),
    ("nodeKind", "shex:nodeKind"),
    ("datatype", "shex:datatype"),
    ("shape", "shex:shape"),
    ("IRI", "shex:IRI"),
    ("BNode", "shex:BNode"),
    ("Literal", "shex:Literal"),
    ("NonLiteral", "shex:NonLiteral"),
    ("extra", "shex:extra"),
    ("closed", "shex:closed"),
    ("annotation", "shex:annotation"),
    ("semActs", "shex:semActs"),
];

// SKOS constructs — knowledge organisation, concept schemes
const TOOLBOX_SKOS: &[(&str, &str)] = &[
    ("Concept", "skos:Concept"),
    ("Concept Scheme", "skos:ConceptScheme"),
    ("Collection", "skos:Collection"),
    ("OrderedCollection", "skos:OrderedCollection"),
    ("prefLabel", "skos:prefLabel"),
    ("altLabel", "skos:altLabel"),
    ("hiddenLabel", "skos:hiddenLabel"),
    ("definition", "skos:definition"),
    ("broader", "skos:broader"),
    ("narrower", "skos:narrower"),
    ("related", "skos:related"),
    ("exactMatch", "skos:exactMatch"),
    ("closeMatch", "skos:closeMatch"),
    ("broadMatch", "skos:broadMatch"),
    ("narrowMatch", "skos:narrowMatch"),
    ("inScheme", "skos:inScheme"),
    ("topConceptOf", "skos:topConceptOf"),
    ("hasTopConcept", "skos:hasTopConcept"),
    ("member", "skos:member"),
    ("notation", "skos:notation"),
];

const PARADIGMS: &[(&str, &str, &str)] = &[
    (
        "OWL/RDFS",
        "owl",
        "Web Ontology Language \u{2014} owl:Thing root, class-based inference",
    ),
    (
        "SHACL",
        "shacl",
        "Shapes Constraint Language \u{2014} shape-based validation, no Thing root",
    ),
    (
        "ShEx",
        "shex",
        "Shape Expressions \u{2014} grammar-based, no Thing root",
    ),
    (
        "SKOS",
        "skos",
        "Knowledge Organisation \u{2014} concept schemes, no Thing root",
    ),
];

pub fn build_ontology_graph_canvas_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    // Toolbar
    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         flex-wrap: wrap; align-items: center;",
    );

    // Paradigm selector
    let paradigm_label = document.create_element("span").unwrap();
    paradigm_label.set_text_content(Some("Paradigm:"));
    let pl_el: HtmlElement = paradigm_label.clone().dyn_into().unwrap();
    pl_el.style().set_css_text(
        "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); margin-right: 2px;",
    );
    toolbar.append_child(&paradigm_label).unwrap();

    for (idx, (name, id, _desc)) in PARADIGMS.iter().enumerate() {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(name));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        let is_active = idx == 0;
        let bg = if is_active {
            "rgba(0, 200, 255, 0.1)"
        } else {
            "transparent"
        };
        let color = if is_active {
            "var(--accent-cyan)"
        } else {
            "var(--text-secondary)"
        };
        b_el.style().set_css_text(&format!(
            "padding: 2px 6px; border: 1px solid {}; \
             background: {}; color: {}; border-radius: 3px; \
             cursor: pointer; font-size: 8px; font-family: var(--font-mono); font-weight: {};",
            if is_active {
                "var(--accent-cyan)"
            } else {
                "var(--border-medium)"
            },
            bg,
            color,
            if is_active { "600" } else { "400" },
        ));
        b_el.set_attribute("data-paradigm", id).unwrap();
        toolbar.append_child(&btn).unwrap();
    }

    let sep = document.create_element("div").unwrap();
    let sep_el: HtmlElement = sep.clone().dyn_into().unwrap();
    sep_el
        .style()
        .set_css_text("width: 1px; height: 16px; background: var(--border-subtle); margin: 0 4px;");
    toolbar.append_child(&sep).unwrap();

    for label in &[
        "+ Node",
        "+ Edge",
        "Layout: Force",
        "Layout: Hierarchical",
        "Export SVG",
        "Compile",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 6px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
        );
        toolbar.append_child(&btn).unwrap();
    }

    let spacer = document.create_element("div").unwrap();
    let sp_el: HtmlElement = spacer.clone().dyn_into().unwrap();
    sp_el.style().set_css_text("flex: 1;");
    toolbar.append_child(&spacer).unwrap();

    let stats = document.create_element("span").unwrap();
    stats.set_text_content(Some("8 nodes | 5 edges"));
    let st_el: HtmlElement = stats.clone().dyn_into().unwrap();
    st_el
        .style()
        .set_css_text("font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);");
    toolbar.append_child(&stats).unwrap();
    wrapper.append_child(&toolbar).unwrap();

    // Main area: toolbox sidebar + canvas
    let main = document.create_element("div").unwrap();
    let m_el: HtmlElement = main.clone().dyn_into().unwrap();
    m_el.style()
        .set_css_text("display: flex; flex: 1; overflow: hidden;");

    // Toolbox sidebar
    let sidebar = document.create_element("div").unwrap();
    let sb_el: HtmlElement = sidebar.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "width: 120px; border-right: 1px solid var(--border-subtle); \
         overflow-y: auto; padding: 4px; flex-shrink: 0;",
    );

    let groups: [(&str, &[(&str, &str)]); 12] = [
        ("\u{1F4D6} OWL Classes", &TOOLBOX_OWL[0..2]),
        ("\u{1F4D6} OWL Properties", &TOOLBOX_OWL[2..5]),
        ("\u{1F4D6} OWL Relations", &TOOLBOX_OWL[5..10]),
        ("\u{1F4D6} OWL Restrictions", &TOOLBOX_OWL[10..14]),
        ("\u{1F4D6} OWL Logic", &TOOLBOX_OWL[14..18]),
        ("\u{2705} SHACL Shapes", &TOOLBOX_SHACL[0..2]),
        ("\u{2705} SHACL Targets", &TOOLBOX_SHACL[2..6]),
        ("\u{2705} SHACL Constraints", &TOOLBOX_SHACL[6..18]),
        ("\u{1F4E6} ShEx Shapes", &TOOLBOX_SHEX[0..6]),
        ("\u{1F4E6} ShEx Types", &TOOLBOX_SHEX[6..12]),
        ("\u{1F4E6} ShEx Modifiers", &TOOLBOX_SHEX[12..20]),
        ("\u{1F4D0} SKOS Concepts", &TOOLBOX_SKOS[0..20]),
    ];

    for (group_name, items) in &groups {
        let glabel = document.create_element("div").unwrap();
        glabel.set_text_content(Some(group_name));
        let gl_el: HtmlElement = glabel.clone().dyn_into().unwrap();
        gl_el.style().set_css_text(
            "font-size: 7px; font-weight: 700; color: var(--text-muted); \
             font-family: var(--font-mono); text-transform: uppercase; \
             margin-top: 6px; margin-bottom: 2px; padding: 0 2px;",
        );
        sidebar.append_child(&glabel).unwrap();

        for (label, iri) in items.iter() {
            let item = document.create_element("div").unwrap();
            item.set_text_content(Some(label));
            let i_el: HtmlElement = item.clone().dyn_into().unwrap();
            i_el.style().set_css_text(
                "padding: 3px 6px; font-size: 8px; color: var(--text-secondary); \
                 font-family: var(--font-mono); cursor: grab; border-radius: 3px; \
                 border: 1px solid transparent;",
            );
            i_el.set_attribute("data-iri", iri).unwrap();
            sidebar.append_child(&item).unwrap();
        }
    }
    main.append_child(&sidebar).unwrap();

    // Canvas area
    let canvas = document.create_element("div").unwrap();
    let cv_el: HtmlElement = canvas.clone().dyn_into().unwrap();
    cv_el.style().set_css_text(
        "flex: 1; position: relative; overflow: hidden; \
         background: var(--surface-bg); \
         background-image: radial-gradient(circle, var(--border-subtle) 1px, transparent 1px); \
         background-size: 20px 20px;",
    );

    // Render edges (SVG mock)
    let svg_ns = "http://www.w3.org/2000/svg";
    let svg = document.create_element_ns(Some(svg_ns), "svg").unwrap();
    let svg_el: HtmlElement = svg.clone().dyn_into().unwrap();
    svg_el.style().set_css_text(
        "position: absolute; top: 0; left: 0; width: 100%; height: 100%; \
         pointer-events: none;",
    );
    svg.set_attribute("viewBox", "0 0 100 100").unwrap();
    svg.set_attribute("preserveAspectRatio", "none").unwrap();

    for (from, to, _rel) in EDGES {
        let (_n1, _, x1, y1, _) = NODES[*from];
        let (_n2, _, x2, y2, _) = NODES[*to];

        let line = document.create_element_ns(Some(svg_ns), "line").unwrap();
        line.set_attribute("x1", &format!("{}", x1 + 5.0)).unwrap();
        line.set_attribute("y1", &format!("{}", y1 + 3.0)).unwrap();
        line.set_attribute("x2", &format!("{}", x2 + 5.0)).unwrap();
        line.set_attribute("y2", &format!("{}", y2 + 3.0)).unwrap();
        line.set_attribute("stroke", "rgba(0, 200, 255, 0.3)")
            .unwrap();
        line.set_attribute("stroke-width", "0.3").unwrap();
        svg.append_child(&line).unwrap();

        // Edge label at midpoint
        let mid_x = (x1 + x2) / 2.0 + 5.0;
        let mid_y = (y1 + y2) / 2.0 + 3.0;
        let text = document.create_element_ns(Some(svg_ns), "text").unwrap();
        text.set_attribute("x", &format!("{}", mid_x)).unwrap();
        text.set_attribute("y", &format!("{}", mid_y)).unwrap();
        text.set_attribute("font-size", "1.5").unwrap();
        text.set_attribute("fill", "rgba(255, 165, 0, 0.6)")
            .unwrap();
        text.set_text_content(Some(match *from {
            0 => "subClassOf",
            1 => "subClassOf",
            2 => "domain",
            3 => "domain",
            _ => "domain",
        }));
        svg.append_child(&text).unwrap();
    }
    canvas.append_child(&svg).unwrap();

    // Render nodes
    for (name, ntype, x, y, color) in NODES {
        let node = document.create_element("div").unwrap();
        let n_el: HtmlElement = node.clone().dyn_into().unwrap();
        let is_property = ntype.contains("Property");
        let border_color = if is_property {
            "rgba(255, 165, 0, 0.5)"
        } else if ntype.contains("Class") {
            "rgba(0, 200, 255, 0.5)"
        } else {
            "rgba(200, 150, 255, 0.5)"
        };
        n_el.style().set_css_text(&format!(
            "position: absolute; left: {}%; top: {}%; \
             padding: 4px 8px; border-radius: 6px; \
             background: {}; border: 2px solid {}; \
             font-size: 9px; color: var(--text-primary); \
             font-family: var(--font-mono); font-weight: 600; \
             cursor: move; user-select: none; white-space: nowrap;",
            x, y, color, border_color,
        ));
        node.set_text_content(Some(name));

        // Type badge
        let badge = document.create_element("div").unwrap();
        badge.set_text_content(Some(ntype));
        let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "font-size: 6px; color: var(--text-muted); font-weight: 400; \
             font-family: var(--font-mono); margin-top: 1px;",
        );
        node.append_child(&badge).unwrap();

        canvas.append_child(&node).unwrap();
    }

    // Zoom controls
    let zoom_in = document.create_element("div").unwrap();
    zoom_in.set_text_content(Some("+"));
    let zi_el: HtmlElement = zoom_in.clone().dyn_into().unwrap();
    zi_el.style().set_css_text(
        "position: absolute; right: 6px; top: 6px; width: 22px; height: 22px; \
         background: var(--surface-panel); border-radius: 4px; display: flex; \
         align-items: center; justify-content: center; font-size: 16px; \
         color: var(--text-secondary); cursor: pointer; border: 1px solid var(--border-medium);",
    );
    canvas.append_child(&zoom_in).unwrap();

    let zoom_out = document.create_element("div").unwrap();
    zoom_out.set_text_content(Some("\u{2212}"));
    let zo_el: HtmlElement = zoom_out.clone().dyn_into().unwrap();
    zo_el.style().set_css_text(
        "position: absolute; right: 6px; top: 32px; width: 22px; height: 22px; \
         background: var(--surface-panel); border-radius: 4px; display: flex; \
         align-items: center; justify-content: center; font-size: 16px; \
         color: var(--text-secondary); cursor: pointer; border: 1px solid var(--border-medium);",
    );
    canvas.append_child(&zoom_out).unwrap();

    main.append_child(&canvas).unwrap();
    wrapper.append_child(&main).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} graph canvas requires ontology_loader + wgpu render engine.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
