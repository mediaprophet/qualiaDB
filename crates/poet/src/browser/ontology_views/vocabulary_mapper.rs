//! Vocabulary Mapper — WordNet/synset lookup + ontology vocabulary mapping (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const SYNSETS: &[(&str, &str, &str, &[&str])] = &[
    (
        "obligation#n1",
        "n",
        "The social force that binds you to the courses of action demanded by that force",
        &["duty", "responsibility"],
    ),
    (
        "obligation#n2",
        "n",
        "A legal agreement specifying a payment or action and the penalty for failure to comply",
        &["debt", "liability"],
    ),
    (
        "obligation#n3",
        "n",
        "A personal relation in which one is indebted for a service or favour",
        &["debt of gratitude"],
    ),
    (
        "commitment#n1",
        "n",
        "The act of binding yourself (intellectually or emotionally) to a course of action",
        &["dedication", "allegiance"],
    ),
    (
        "commitment#n2",
        "n",
        "An engagement by contract involving financial obligation",
        &["pledge", "promise"],
    ),
    (
        "duty#n1",
        "n",
        "The social force that binds you to the courses of action demanded by that force",
        &["obligation", "responsibility"],
    ),
    (
        "duty#n2",
        "n",
        "Work that you are obliged to perform for moral or legal reasons",
        &["task", "function"],
    ),
    (
        "responsibility#n1",
        "n",
        "The social force that binds you to the courses of action demanded by that force",
        &["duty", "obligation"],
    ),
    (
        "responsibility#n2",
        "n",
        "A form of trustworthiness; the trait of being answerable to someone",
        &["accountability"],
    ),
    (
        "accountability#n1",
        "n",
        "Responsibility to someone or for some activity",
        &["answerability", "answerableness"],
    ),
];

const MAPPINGS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "obligation",
        "obl:Obligation",
        "owl:equivalentClass",
        "obligations.n3",
        "Confirmed",
    ),
    (
        "duty",
        "doc:DutyOfCare",
        "skos:closeMatch",
        "duty-of-care.n3",
        "Confirmed",
    ),
    (
        "responsibility",
        "obl:Responsibility",
        "owl:equivalentClass",
        "obligations.n3",
        "Confirmed",
    ),
    (
        "commitment",
        "obl:Commitment",
        "skos:exactMatch",
        "obligations.n3",
        "Confirmed",
    ),
    (
        "accountability",
        "obl:Accountability",
        "skos:closeMatch",
        "obligations.n3",
        "Pending",
    ),
    (
        "personhood",
        "per:Personhood",
        "owl:equivalentClass",
        "personhood.n3",
        "Confirmed",
    ),
    (
        "agency",
        "agn:Agency",
        "owl:equivalentClass",
        "agency.n3",
        "Confirmed",
    ),
    (
        "provenance",
        "prov:Provenance",
        "owl:equivalentClass",
        "provenance.n3",
        "Confirmed",
    ),
];

const HYPERNYM_CHAIN: &[(&str, &str)] = &[
    ("obligation", "duty"),
    ("duty", "work"),
    ("work", "activity"),
    ("activity", "act"),
    ("act", "event"),
    ("event", "psychological_feature"),
    ("psychological_feature", "abstraction"),
    ("abstraction", "entity"),
];

pub fn build_vocabulary_mapper_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    // Search bar
    let search_bar = document.create_element("div").unwrap();
    let sb_el: HtmlElement = search_bar.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle);",
    );

    let search_input = document.create_element("input").unwrap();
    search_input.set_attribute("type", "text").unwrap();
    search_input.set_attribute("value", "obligation").unwrap();
    let si_el: HtmlElement = search_input.clone().dyn_into().unwrap();
    si_el.style().set_css_text(
        "flex: 1; padding: 4px 8px; background: var(--surface-bg); \
         border: 1px solid var(--border-medium); border-radius: 3px; \
         font-size: 9px; font-family: var(--font-mono); color: var(--text-primary);",
    );
    search_bar.append_child(&search_input).unwrap();

    for label in &["Search WordNet", "Search Lexicon", "Suggest Mapping"] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 6px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
        );
        search_bar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&search_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Synsets section
    let syn_header = document.create_element("div").unwrap();
    syn_header.set_text_content(Some("WordNet Synsets (10 matches)"));
    let sh_el: HtmlElement = syn_header.clone().dyn_into().unwrap();
    sh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&syn_header).unwrap();

    for (id, pos, gloss, synonyms) in SYNSETS {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "padding: 6px 8px; background: var(--surface-panel); border-radius: 6px; \
             margin-bottom: 4px; border: 1px solid var(--border-subtle);",
        );

        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style()
            .set_css_text("display: flex; align-items: center; gap: 6px;");

        let id_span = document.create_element("span").unwrap();
        id_span.set_text_content(Some(id));
        let is_el: HtmlElement = id_span.clone().dyn_into().unwrap();
        is_el.style().set_css_text(
            "font-size: 9px; color: var(--accent-cyan); font-family: var(--font-mono); \
             font-weight: 600;",
        );
        hdr.append_child(&id_span).unwrap();

        let pos_badge = document.create_element("span").unwrap();
        pos_badge.set_text_content(Some(pos));
        let pb_el: HtmlElement = pos_badge.clone().dyn_into().unwrap();
        pb_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             background: var(--surface-bg); padding: 1px 4px; border-radius: 2px;",
        );
        hdr.append_child(&pos_badge).unwrap();
        card.append_child(&hdr).unwrap();

        let gloss_div = document.create_element("div").unwrap();
        gloss_div.set_text_content(Some(gloss));
        let g_el: HtmlElement = gloss_div.clone().dyn_into().unwrap();
        g_el.style().set_css_text(
            "font-size: 8px; color: var(--text-primary); font-family: var(--font-mono); \
             margin-top: 2px; line-height: 1.4;",
        );
        card.append_child(&gloss_div).unwrap();

        let syn_div = document.create_element("div").unwrap();
        let syn_text: String = synonyms
            .iter()
            .map(|s| format!("\u{25CF} {}", s))
            .collect::<Vec<_>>()
            .join("  ");
        syn_div.set_text_content(Some(&format!("Synonyms: {}", syn_text)));
        let s_el: HtmlElement = syn_div.clone().dyn_into().unwrap();
        s_el.style().set_css_text(
            "font-size: 7px; color: var(--text-secondary); font-family: var(--font-mono); \
             margin-top: 2px;",
        );
        card.append_child(&syn_div).unwrap();

        content.append_child(&card).unwrap();
    }

    // Hypernym chain
    let hyp_header = document.create_element("div").unwrap();
    hyp_header.set_text_content(Some("Hypernym Chain (obligation \u{2192} entity)"));
    let hh_el: HtmlElement = hyp_header.clone().dyn_into().unwrap();
    hh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&hyp_header).unwrap();

    let chain_div = document.create_element("div").unwrap();
    let chain_el: HtmlElement = chain_div.clone().dyn_into().unwrap();
    chain_el.style().set_css_text(
        "padding: 6px 8px; background: var(--surface-panel); border-radius: 6px; \
         margin-bottom: 8px; border: 1px solid var(--border-subtle); \
         font-size: 8px; color: var(--text-secondary); font-family: var(--font-mono); \
         line-height: 1.8;",
    );
    let chain_text: String = HYPERNYM_CHAIN
        .iter()
        .map(|(word, parent)| format!("{} \u{2192} {}", word, parent))
        .collect::<Vec<_>>()
        .join("\n");
    chain_div.set_text_content(Some(&chain_text));
    content.append_child(&chain_div).unwrap();

    // Existing mappings
    let map_header = document.create_element("div").unwrap();
    map_header.set_text_content(Some("Vocabulary Mappings (8)"));
    let mh_el: HtmlElement = map_header.clone().dyn_into().unwrap();
    mh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&map_header).unwrap();

    let map_table = make_table(
        document,
        &["Term", "Ontology Class", "Relation", "Source", "Status"],
    );
    let map_tbody = document.create_element("tbody").unwrap();
    for (term, class, relation, source, status) in MAPPINGS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            term.to_string(),
            class.to_string(),
            relation.to_string(),
            source.to_string(),
            status.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = if *status == "Confirmed" {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "rgba(255, 165, 0, 0.8)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 2 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 8px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        map_tbody.append_child(&tr).unwrap();
    }
    map_table.append_child(&map_tbody).unwrap();
    content.append_child(&map_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} vocabulary mapper requires WordNet FST + qualia_core_db lexicon.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn make_table(document: &Document, headers: &[&str]) -> Element {
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 9px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 3px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
