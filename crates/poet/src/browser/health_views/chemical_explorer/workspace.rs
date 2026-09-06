//! Food/compound evidence explorer UI. Local/offline; no invented chemical claims.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, FileReader, HtmlInputElement, HtmlTextAreaElement};

use super::bind::LoadState;
use super::model::{
    ChemicalExplorerState, ExplorerPhase, CHEBI_LICENCE_CATALOGUE_NOTE, NO_ASSET_GUIDANCE,
    RESEARCH_EVIDENCE_BANNER,
};
use crate::browser::surface_states;

pub fn build(document: &Document) -> Element {
    surface_states::install(document);
    let root = document.create_element("section").unwrap();
    root.set_class_name("health-home health-chemical-explorer");
    root.set_attribute("data-chemical-explorer", "").ok();
    root.set_attribute("data-honesty", "unavailable").ok();
    root.set_attribute("data-chebi-load", LoadState::NoAsset.as_str())
        .ok();
    root.set_inner_html(&shell_html());
    wire(document, &root);
    paint(&root, &ChemicalExplorerState::default());
    root
}

fn shell_html() -> String {
    format!(
        r#"
      <header class="health-hero">
        <div>
          <div class="health-eyebrow">Food &amp; compound evidence</div>
          <h2>Search chemical identity</h2>
          <p>Browse locally imported ChEBI compounds. Associations are research evidence only — never treatment recommendations. Without a local compounds.tsv import (asset import AST-02/AST-03) or a local fixture load, search stays empty and never fetches remote release bytes.</p>
        </div>
        <div class="health-hero-actions">
          <span class="health-privacy-chip">{banner}</span>
          <button class="health-secondary-button" type="button" data-chebi-licence-toggle aria-expanded="false">Licence &amp; source</button>
        </div>
      </header>

      <section class="health-card" data-chebi-licence-drawer hidden aria-labelledby="chebi-licence-title">
        <div class="health-card-heading">
          <div>
            <span class="health-card-kicker">Catalogue note</span>
            <h3 id="chebi-licence-title">ChEBI licence &amp; source</h3>
          </div>
        </div>
        <p data-chebi-licence-body>{licence}</p>
        <p class="health-muted">Official catalogue URL: https://www.ebi.ac.uk/chebi/ — importer candidate only. This drawer does not fetch or redistribute release bytes.</p>
      </section>

      <section class="health-card" aria-labelledby="chebi-asset-title">
        <div class="health-card-heading">
          <div>
            <span class="health-card-kicker">Local only</span>
            <h3 id="chebi-asset-title">Load compounds.tsv</h3>
          </div>
          <span class="health-unit-badge" data-chebi-asset-badge>No asset</span>
        </div>
        <p class="health-muted">Point at a tiny synthetic or imported compounds.tsv (AST-03 header). No network download. Daemon Host invoke is unavailable under the vibe-host freeze — fixture bind only.</p>
        <label class="health-field health-field-wide">
          <span>Release label</span>
          <input type="text" data-chebi-release-label value="local-fixture" autocomplete="off" spellcheck="false">
        </label>
        <label class="health-field health-field-wide">
          <span>Local file</span>
          <input type="file" data-chebi-file accept=".tsv,.txt,text/tab-separated-values,text/plain">
        </label>
        <label class="health-field health-field-wide">
          <span>Or paste TSV</span>
          <textarea data-chebi-paste rows="4" placeholder="ID&#9;STATUS&#9;CHEBI_ACCESSION&#9;…" spellcheck="false"></textarea>
        </label>
        <div class="health-hero-actions">
          <button class="health-secondary-button" type="button" data-chebi-load-paste>Load pasted TSV</button>
          <button class="health-secondary-button" type="button" data-chebi-clear-asset>Clear local asset</button>
        </div>
      </section>

      <section class="health-card" aria-labelledby="chebi-search-title">
        <div class="health-card-heading">
          <div>
            <span class="health-card-kicker">Local asset</span>
            <h3 id="chebi-search-title">Search</h3>
          </div>
        </div>
        <label class="health-field health-field-wide">
          <span>Accession or name</span>
          <input type="search" data-chebi-query placeholder="CHEBI:…" autocomplete="off" spellcheck="false">
        </label>
        <div class="health-status" role="status" aria-live="polite" data-chebi-status></div>
        <ul class="health-chebi-results" data-chebi-results aria-label="Compound search results"></ul>
      </section>

      <section class="health-card" aria-labelledby="chebi-entity-title">
        <div class="health-card-heading">
          <div>
            <span class="health-card-kicker">Entity</span>
            <h3 id="chebi-entity-title">Selected compound</h3>
          </div>
        </div>
        <dl class="health-chebi-entity" data-chebi-entity>
          <div><dt>Accession</dt><dd data-chebi-accession>—</dd></div>
          <div><dt>Name</dt><dd data-chebi-name>—</dd></div>
          <div><dt>Parent</dt><dd data-chebi-parent>—</dd></div>
          <div><dt>Release</dt><dd data-chebi-release>—</dd></div>
        </dl>
      </section>

      <section class="health-card" aria-labelledby="chebi-relations-title">
        <div class="health-card-heading">
          <div>
            <span class="health-card-kicker">{banner}</span>
            <h3 id="chebi-relations-title">Relationships</h3>
          </div>
        </div>
        <p class="health-muted">Parent and child links from the imported ontology. Not medical advice.</p>
        <ul class="health-chebi-relations" data-chebi-relations aria-label="Research relationship evidence"></ul>
      </section>

      <section class="health-card" aria-labelledby="chebi-evidence-title">
        <div class="health-card-heading">
          <div>
            <span class="health-card-kicker">Provenance</span>
            <h3 id="chebi-evidence-title">Evidence</h3>
          </div>
        </div>
        <ul class="health-chebi-evidence" data-chebi-evidence aria-label="Evidence and provenance rows"></ul>
      </section>
    "#,
        banner = RESEARCH_EVIDENCE_BANNER,
        licence = html_escape(CHEBI_LICENCE_CATALOGUE_NOTE),
    )
}

fn html_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn wire(document: &Document, root: &Element) {
    if let Some(input) = root.query_selector("[data-chebi-query]").ok().flatten() {
        let root_for_input = root.clone();
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            let mut state = read_state(&root_for_input);
            let query = field_value(&root_for_input, "[data-chebi-query]");
            state.set_query(&query);
            if state.load.allows_query() {
                state.run_local_search();
            }
            paint(&root_for_input, &state);
            write_state(&root_for_input, &state);
        }) as Box<dyn FnMut(_)>);
        input
            .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }

    if let Some(toggle) = root
        .query_selector("[data-chebi-licence-toggle]")
        .ok()
        .flatten()
    {
        let root_for_toggle = root.clone();
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            let mut state = read_state(&root_for_toggle);
            state.toggle_licence_drawer();
            paint(&root_for_toggle, &state);
            write_state(&root_for_toggle, &state);
        }) as Box<dyn FnMut(_)>);
        toggle
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }

    if let Some(btn) = root
        .query_selector("[data-chebi-load-paste]")
        .ok()
        .flatten()
    {
        let root_for_paste = root.clone();
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            let mut state = read_state(&root_for_paste);
            let text = textarea_value(&root_for_paste, "[data-chebi-paste]");
            let release = field_value(&root_for_paste, "[data-chebi-release-label]");
            state.ingest_fixture_tsv(&text, &release);
            paint(&root_for_paste, &state);
            write_state(&root_for_paste, &state);
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }

    if let Some(btn) = root
        .query_selector("[data-chebi-clear-asset]")
        .ok()
        .flatten()
    {
        let root_for_clear = root.clone();
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            let drawer = read_state(&root_for_clear).licence_drawer_open;
            let mut state = ChemicalExplorerState::default();
            state.licence_drawer_open = drawer;
            if let Some(paste) = root_for_clear
                .query_selector("[data-chebi-paste]")
                .ok()
                .flatten()
            {
                if let Ok(ta) = paste.dyn_into::<HtmlTextAreaElement>() {
                    ta.set_value("");
                }
            }
            if let Some(file) = root_for_clear
                .query_selector("[data-chebi-file]")
                .ok()
                .flatten()
            {
                if let Ok(input) = file.dyn_into::<HtmlInputElement>() {
                    input.set_value("");
                }
            }
            paint(&root_for_clear, &state);
            write_state(&root_for_clear, &state);
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }

    if let Some(file_input) = root.query_selector("[data-chebi-file]").ok().flatten() {
        let root_for_file = root.clone();
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            let Some(input) = root_for_file
                .query_selector("[data-chebi-file]")
                .ok()
                .flatten()
                .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            else {
                return;
            };
            let Some(file) = input.files().and_then(|list| list.get(0)) else {
                return;
            };
            let Ok(reader) = FileReader::new() else {
                return;
            };
            let root_on_load = root_for_file.clone();
            let reader_for_load = reader.clone();
            let load = Closure::wrap(Box::new(move |_event: Event| {
                let text = reader_for_load
                    .result()
                    .ok()
                    .and_then(|value| value.as_string())
                    .unwrap_or_default();
                let mut state = read_state(&root_on_load);
                let release = field_value(&root_on_load, "[data-chebi-release-label]");
                if let Some(paste) = root_on_load
                    .query_selector("[data-chebi-paste]")
                    .ok()
                    .flatten()
                {
                    if let Ok(ta) = paste.dyn_into::<HtmlTextAreaElement>() {
                        ta.set_value(&text);
                    }
                }
                state.ingest_fixture_tsv(&text, &release);
                paint(&root_on_load, &state);
                write_state(&root_on_load, &state);
            }) as Box<dyn FnMut(_)>);
            let _ = reader.add_event_listener_with_callback("load", load.as_ref().unchecked_ref());
            load.forget();
            let _ = reader.read_as_text(&file);
        }) as Box<dyn FnMut(_)>);
        file_input
            .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }

    if let Some(list) = root.query_selector("[data-chebi-results]").ok().flatten() {
        let root_for_list = root.clone();
        let document = document.clone();
        let closure = Closure::wrap(Box::new(move |event: Event| {
            let Some(target) = event.target() else {
                return;
            };
            let Ok(element) = target.dyn_into::<Element>() else {
                return;
            };
            let button = if element.has_attribute("data-chebi-select") {
                element
            } else {
                match element.closest("[data-chebi-select]").ok().flatten() {
                    Some(btn) => btn,
                    None => return,
                }
            };
            let Some(accession) = button.get_attribute("data-chebi-select") else {
                return;
            };
            let mut state = read_state(&root_for_list);
            let _ = state.select_hit(&accession);
            paint(&root_for_list, &state);
            write_state(&root_for_list, &state);
            let _ = document;
        }) as Box<dyn FnMut(_)>);
        list.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
}

fn field_value(root: &Element, selector: &str) -> String {
    let Some(element) = root.query_selector(selector).ok().flatten() else {
        return String::new();
    };
    if let Ok(input) = element.dyn_into::<HtmlInputElement>() {
        return input.value();
    }
    String::new()
}

fn textarea_value(root: &Element, selector: &str) -> String {
    let Some(element) = root.query_selector(selector).ok().flatten() else {
        return String::new();
    };
    if let Ok(ta) = element.dyn_into::<HtmlTextAreaElement>() {
        return ta.value();
    }
    String::new()
}

fn read_state(root: &Element) -> ChemicalExplorerState {
    let encoded = root
        .get_attribute("data-chebi-state")
        .unwrap_or_default();
    let mut state = ChemicalExplorerState::default();
    if !encoded.is_empty() {
        // Compact: load|drawer|query|release
        let mut parts = encoded.splitn(4, '|');
        let load = parts.next().unwrap_or("no_asset");
        let drawer = parts.next().unwrap_or("0");
        let query = parts.next().unwrap_or("").to_string();
        let release = parts.next().unwrap_or("local-fixture").to_string();
        state.load = LoadState::parse(load);
        // Legacy "yes"/"no" from AST-06
        if load == "yes" {
            state.load = LoadState::Ready;
        } else if load == "no" {
            state.load = LoadState::NoAsset;
        }
        state.licence_drawer_open = drawer == "1";
        state.query = query;
        if !release.is_empty() {
            state.release_label = release;
        }
    }
    // Re-bind session from paste buffer when Ready (fixture path; no invent).
    let fixture = textarea_value(root, "[data-chebi-paste]");
    if !fixture.trim().is_empty()
        && matches!(
            state.load,
            LoadState::Ready | LoadState::Loading | LoadState::Fault | LoadState::Denied
        )
    {
        let release = field_value(root, "[data-chebi-release-label]");
        let release = if release.is_empty() {
            state.release_label.clone()
        } else {
            release
        };
        // Preserve query / drawer across re-ingest.
        let query = state.query.clone();
        let drawer = state.licence_drawer_open;
        state.ingest_fixture_tsv(&fixture, &release);
        state.query = query;
        state.licence_drawer_open = drawer;
        if state.load.allows_query() && !state.query.trim().is_empty() {
            state.run_local_search();
        }
    } else if state.load == LoadState::Ready && fixture.trim().is_empty() {
        // Ready without fixture text → cannot query honestly; demote.
        state.load = LoadState::NoAsset;
    }
    state
}

fn write_state(root: &Element, state: &ChemicalExplorerState) {
    let load = state.load.as_str();
    let drawer = if state.licence_drawer_open { "1" } else { "0" };
    let encoded = format!(
        "{load}|{drawer}|{}|{}",
        state.query.replace('|', " "),
        state.release_label.replace('|', " ")
    );
    root.set_attribute("data-chebi-state", &encoded).ok();
    root.set_attribute("data-chebi-load", state.load.as_str()).ok();
}

fn paint(root: &Element, state: &ChemicalExplorerState) {
    let honesty = match state.phase() {
        ExplorerPhase::NoAsset | ExplorerPhase::Denied | ExplorerPhase::Fault => "unavailable",
        ExplorerPhase::Loading => "loading",
        ExplorerPhase::EmptySearch => "empty",
        ExplorerPhase::SelectedEntity => "present",
    };
    root.set_attribute("data-honesty", honesty).ok();
    root.set_attribute("data-chebi-phase", &format!("{:?}", state.phase()))
        .ok();
    root.set_attribute("data-chebi-load", state.load.as_str()).ok();

    if let Some(badge) = root.query_selector("[data-chebi-asset-badge]").ok().flatten() {
        let label = match state.load {
            LoadState::NoAsset => "No asset",
            LoadState::Loading => "Loading…",
            LoadState::Ready => "Local asset",
            LoadState::Denied => "Denied",
            LoadState::Fault => "Fault",
        };
        badge.set_text_content(Some(label));
    }

    if let Some(status) = root.query_selector("[data-chebi-status]").ok().flatten() {
        status.set_text_content(Some(&state.status_message()));
    }

    if let Some(drawer) = root
        .query_selector("[data-chebi-licence-drawer]")
        .ok()
        .flatten()
    {
        if state.licence_drawer_open {
            drawer.remove_attribute("hidden").ok();
        } else {
            drawer.set_attribute("hidden", "").ok();
        }
    }
    if let Some(toggle) = root
        .query_selector("[data-chebi-licence-toggle]")
        .ok()
        .flatten()
    {
        toggle
            .set_attribute(
                "aria-expanded",
                if state.licence_drawer_open {
                    "true"
                } else {
                    "false"
                },
            )
            .ok();
    }

    if let Some(input) = root.query_selector("[data-chebi-query]").ok().flatten() {
        if let Ok(html_input) = input.dyn_into::<HtmlInputElement>() {
            if html_input.value() != state.query {
                html_input.set_value(&state.query);
            }
        }
    }
    if let Some(input) = root
        .query_selector("[data-chebi-release-label]")
        .ok()
        .flatten()
    {
        if let Ok(html_input) = input.dyn_into::<HtmlInputElement>() {
            if html_input.value() != state.release_label {
                html_input.set_value(&state.release_label);
            }
        }
    }

    paint_results(root, state);
    paint_entity(root, state);
    paint_relations(root, state);
    paint_evidence(root, state);
}

fn paint_results(root: &Element, state: &ChemicalExplorerState) {
    let Some(list) = root.query_selector("[data-chebi-results]").ok().flatten() else {
        return;
    };
    list.set_inner_html("");
    let doc = list
        .owner_document()
        .unwrap_or_else(|| web_sys::window().unwrap().document().unwrap());
    match state.load {
        LoadState::NoAsset => {
            list.append_child(&create_li(&doc, NO_ASSET_GUIDANCE)).ok();
            return;
        }
        LoadState::Loading => {
            list.append_child(&create_li(&doc, "Loading local compounds…"))
                .ok();
            return;
        }
        LoadState::Denied | LoadState::Fault => {
            list.append_child(&create_li(&doc, &state.status_message()))
                .ok();
            return;
        }
        LoadState::Ready => {}
    }
    if state.hits.is_empty() {
        let msg = if state.query.trim().is_empty() {
            "Results list is empty. No compounds are shown until a search against a local asset returns hits."
        } else {
            "No matches. Empty results are honest — this view does not invent compound rows."
        };
        list.append_child(&create_li(&doc, msg)).ok();
        return;
    }
    for hit in &state.hits {
        let li = doc.create_element("li").unwrap();
        let button = doc.create_element("button").unwrap();
        button.set_attribute("type", "button").ok();
        button
            .set_attribute("data-chebi-select", &hit.accession)
            .ok();
        button.set_class_name("health-chebi-result-btn");
        button.set_text_content(Some(&format!("{} — {}", hit.accession, hit.name)));
        li.append_child(&button).ok();
        list.append_child(&li).ok();
    }
}

fn paint_entity(root: &Element, state: &ChemicalExplorerState) {
    let set = |attr: &str, value: &str| {
        if let Some(el) = root.query_selector(attr).ok().flatten() {
            el.set_text_content(Some(value));
        }
    };
    match &state.selected {
        Some(hit) => {
            set("[data-chebi-accession]", &hit.accession);
            set("[data-chebi-name]", &hit.name);
            set(
                "[data-chebi-parent]",
                hit.parent_accession.as_deref().unwrap_or("—"),
            );
            set("[data-chebi-release]", &hit.release_label);
        }
        None => {
            set("[data-chebi-accession]", "—");
            set("[data-chebi-name]", "—");
            set("[data-chebi-parent]", "—");
            set("[data-chebi-release]", "—");
        }
    }
}

fn paint_relations(root: &Element, state: &ChemicalExplorerState) {
    let Some(list) = root.query_selector("[data-chebi-relations]").ok().flatten() else {
        return;
    };
    list.set_inner_html("");
    let doc = list
        .owner_document()
        .unwrap_or_else(|| web_sys::window().unwrap().document().unwrap());
    if state.selected.is_none() {
        list.append_child(&create_li(
            &doc,
            "Select a compound to inspect parent/child research links.",
        ))
        .ok();
        return;
    }
    if state.relations.is_empty() {
        list.append_child(&create_li(
            &doc,
            "No relationship rows loaded. Empty is honest — links appear only from the local asset.",
        ))
        .ok();
        return;
    }
    for rel in &state.relations {
        let text = format!(
            "{} → {} · {} · line {} · {} · {}",
            rel.child_accession,
            rel.parent_accession,
            rel.release_label,
            rel.source_line,
            rel.uncertainty.as_str(),
            RESEARCH_EVIDENCE_BANNER
        );
        list.append_child(&create_li(&doc, &text)).ok();
    }
}

fn paint_evidence(root: &Element, state: &ChemicalExplorerState) {
    let Some(list) = root.query_selector("[data-chebi-evidence]").ok().flatten() else {
        return;
    };
    list.set_inner_html("");
    let doc = list
        .owner_document()
        .unwrap_or_else(|| web_sys::window().unwrap().document().unwrap());
    if state.selected.is_none() {
        list.append_child(&create_li(
            &doc,
            "Provenance appears after an entity is selected from local results.",
        ))
        .ok();
        return;
    }
    if state.evidence.is_empty() {
        list.append_child(&create_li(
            &doc,
            "No provenance rows yet. Licence obligations stay visible in the licence drawer.",
        ))
        .ok();
        return;
    }
    for row in &state.evidence {
        let text = format!(
            "{} · {} · line {} · {} · {}",
            row.accession,
            row.release_label,
            row.source_line,
            row.uncertainty.as_str(),
            row.licence_note
        );
        list.append_child(&create_li(&doc, &text)).ok();
    }
}

fn create_li(document: &Document, text: &str) -> Element {
    let li = document.create_element("li").unwrap();
    li.set_text_content(Some(text));
    li
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::health_views::chemical_explorer::bind::synthetic_compounds_tsv;
    use crate::browser::health_views::chemical_explorer::model::{
        ChemicalHitView, UncertaintyLabel,
    };

    #[test]
    fn shell_mentions_research_evidence_and_import_path() {
        let html = shell_html();
        assert!(html.contains(RESEARCH_EVIDENCE_BANNER));
        assert!(html.contains("Licence"));
        assert!(html.contains("CC BY 4.0"));
        assert!(html.contains("compounds.tsv"));
        assert!(html.contains("data-chebi-file"));
        assert!(html.contains("data-chebi-paste"));
        assert!(!html.contains("data-chebi-download"));
        assert!(!html.contains("Fetch release"));
        assert!(!html.contains("<a href=\"http"));
    }

    #[test]
    fn empty_state_paint_helpers_do_not_require_hits() {
        let state = ChemicalExplorerState::default();
        assert_eq!(state.phase(), ExplorerPhase::NoAsset);
        assert!(state.hits.is_empty());
        assert!(state.selected.is_none());
    }

    #[test]
    fn selecting_from_fixture_hits_is_local_only() {
        let mut state = ChemicalExplorerState::default();
        state.mark_asset_available();
        state.apply_search_hits(vec![ChemicalHitView {
            accession: "CHEBI:00000".into(),
            name: "fixture-only".into(),
            parent_accession: None,
            release_label: "unit-test".into(),
            source_line: 0,
            uncertainty: UncertaintyLabel::Unknown,
            licence_note: CHEBI_LICENCE_CATALOGUE_NOTE.into(),
        }]);
        assert!(state.select_hit("CHEBI:00000"));
        assert_eq!(state.phase(), ExplorerPhase::SelectedEntity);
    }

    #[test]
    fn live_fixture_bind_drives_search_entity_relations() {
        let mut state = ChemicalExplorerState::default();
        state.ingest_fixture_tsv(synthetic_compounds_tsv(), "ui-test");
        assert_eq!(state.load, LoadState::Ready);
        state.set_query("16236");
        state.run_local_search();
        assert_eq!(state.hits.len(), 1);
        assert!(state.select_hit("CHEBI:16236"));
        assert!(!state.relations.is_empty());
        assert!(!state.evidence.is_empty());
        assert!(state.status_message().contains("not medical advice"));
    }
}
