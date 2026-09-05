//! Command palette / omnibox — Ctrl+K palette for quick navigation.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

mod commands;
mod placements;

use commands::{build_command_list, CommandEntry};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, KeyboardEvent};

/// Build the command palette overlay (hidden by default).
pub fn build_command_palette(document: &Document) -> Element {
    let overlay = document.create_element("div").unwrap();
    overlay.set_id("command-palette");
    overlay.set_attribute("role", "dialog").ok();
    overlay.set_attribute("aria-modal", "true").ok();
    overlay
        .set_attribute("aria-labelledby", "cmd-palette-title")
        .ok();
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; display: none; \
         align-items: flex-start; justify-content: center; padding-top: 120px;",
    );

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("cmd-palette-panel");
    super::surface_aspects::mark(&panel, "entrance");
    panel
        .set_attribute("aria-describedby", "cmd-palette-hint")
        .ok();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 560px; max-height: 400px; background: var(--glass-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-sm); \
         backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px); \
         box-shadow: 0 8px 32px rgba(0,0,0,0.4); overflow: hidden; \
         display: flex; flex-direction: column;",
    );

    let title = document.create_element("h2").unwrap();
    title.set_id("cmd-palette-title");
    title.set_text_content(Some("Command palette"));
    title.set_attribute("style", "position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap;").ok();
    panel.append_child(&title).unwrap();

    let hint = document.create_element("div").unwrap();
    hint.set_id("cmd-palette-hint");
    hint.set_text_content(Some("Search available commands and destinations."));
    hint.set_attribute("style", "position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap;").ok();
    panel.append_child(&hint).unwrap();

    // Search input
    let input = document.create_element("input").unwrap();
    let input_el: HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_placeholder("Search commands, manifolds, containers\u{2026}");
    input.set_id("cmd-palette-input");
    input
        .set_attribute("aria-label", "Search commands, manifolds, and containers")
        .ok();
    input
        .set_attribute("aria-controls", "cmd-palette-results")
        .ok();
    input.set_attribute("aria-autocomplete", "list").ok();
    input
        .set_attribute(
            "style",
            "width: 100%; box-sizing: border-box; padding: 14px 16px; \
         background: transparent; border: none; border-bottom: 1px solid var(--border-subtle); \
         color: var(--text-primary); font-size: 14px; font-family: var(--font-mono); \
         outline: none;",
        )
        .unwrap();
    panel.append_child(&input).unwrap();

    // Results list
    let results = document.create_element("div").unwrap();
    results.set_id("cmd-palette-results");
    results.set_attribute("role", "listbox").ok();
    results.set_attribute("aria-label", "Command results").ok();
    let results_el: HtmlElement = results.clone().dyn_into().unwrap();
    results_el
        .style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px 0;");

    let commands = build_command_list();
    for (idx, cmd) in commands.iter().enumerate() {
        let item = document.create_element("div").unwrap();
        item.set_class_name("cmd-palette-item");
        item.set_attribute("role", "option").ok();
        item.set_attribute("aria-selected", if idx == 0 { "true" } else { "false" })
            .ok();
        item.set_attribute("data-cmd-label", cmd.label).unwrap();
        if idx == 0 {
            item.class_list().add_1("selected").unwrap();
        }
        let item_el: HtmlElement = item.clone().dyn_into().unwrap();
        item_el.style().set_css_text(
            "padding: 8px 16px; cursor: pointer; display: flex; \
             align-items: center; gap: 8px; font-size: 12px; \
             color: var(--text-secondary); font-family: var(--font-mono);",
        );

        let icon = document.create_element("span").unwrap();
        icon.set_text_content(Some(cmd.icon));
        let icon_el: HtmlElement = icon.clone().dyn_into().unwrap();
        icon_el
            .style()
            .set_css_text("color: var(--accent-cyan); width: 20px; text-align: center;");
        item.append_child(&icon).unwrap();

        let label = document.create_element("span").unwrap();
        label.set_text_content(Some(cmd.label));
        item.append_child(&label).unwrap();

        let shortcut = document.create_element("span").unwrap();
        shortcut.set_text_content(Some(cmd.shortcut));
        let sc_el: HtmlElement = shortcut.clone().dyn_into().unwrap();
        sc_el
            .style()
            .set_css_text("margin-left: auto; color: var(--text-muted); font-size: 10px;");
        item.append_child(&shortcut).unwrap();

        results.append_child(&item).unwrap();
    }

    panel.append_child(&results).unwrap();
    overlay.append_child(&panel).unwrap();

    overlay
}

/// Wire up Ctrl+K to toggle the command palette, Escape to close,
/// Arrow Up/Down to navigate, Enter to execute, and search input filtering.
/// Undo/redo is handled separately in `history::wire_undo_redo`.
pub fn wire_command_palette(document: &Document) {
    let closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        if e.key() == "k" && (e.ctrl_key() || e.meta_key()) {
            e.prevent_default();
            toggle_palette(document);
            return;
        }

        // Check if palette is open
        let palette_open = document
            .get_element_by_id("command-palette")
            .map(|p| {
                let el: HtmlElement = p.dyn_into().unwrap();
                el.style().get_property_value("display").unwrap_or_default() != "none"
            })
            .unwrap_or(false);

        if !palette_open {
            return;
        }

        match e.key().as_str() {
            "Escape" => {
                if let Some(p) = document.get_element_by_id("command-palette") {
                    let p_el: HtmlElement = p.dyn_into().unwrap();
                    p_el.style().set_property("display", "none").unwrap();
                }
            }
            "ArrowDown" => {
                e.prevent_default();
                move_selection(document, 1);
            }
            "ArrowUp" => {
                e.prevent_default();
                move_selection(document, -1);
            }
            "Enter" => {
                e.prevent_default();
                execute_selected(document);
            }
            _ => {}
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);

    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    // Wire search input filtering
    wire_search_filter(document);

    // Wire command item clicks and hover
    wire_command_clicks(document);
    wire_command_hover(document);
}

/// Move the selection in the command palette by `delta` (1 = down, -1 = up).
fn move_selection(document: &Document, delta: i32) {
    let items = document.query_selector_all(".cmd-palette-item").unwrap();
    if items.length() == 0 {
        return;
    }

    // Find current selected index
    let mut current_idx: i32 = -1;
    for i in 0..items.length() {
        let item = items.get(i).unwrap();
        let el: Element = item.dyn_into().unwrap();
        if el.class_list().contains("selected") {
            current_idx = i as i32;
            el.class_list().remove_1("selected").unwrap();
            break;
        }
    }

    // Compute new index with wrap-around
    let count = items.length() as i32;
    let new_idx = if current_idx == -1 {
        if delta > 0 {
            0
        } else {
            count - 1
        }
    } else {
        (current_idx + delta + count) % count
    };

    // Set new selected
    let item = items.get(new_idx as u32).unwrap();
    let el: Element = item.dyn_into().unwrap();
    el.class_list().add_1("selected").unwrap();

    // Scroll into view
    if let Ok(html_el) = el.dyn_into::<HtmlElement>() {
        html_el.scroll_into_view();
    }
}

/// Execute the currently selected command in the palette.
fn execute_selected(document: &Document) {
    let items = document.query_selector_all(".cmd-palette-item").unwrap();
    for i in 0..items.length() {
        let item = items.get(i).unwrap();
        let el: Element = item.dyn_into().unwrap();
        if el.class_list().contains("selected") {
            let label = el.get_attribute("data-cmd-label").unwrap_or_default();
            execute_command(&label);
            // Close palette
            if let Some(p) = document.get_element_by_id("command-palette") {
                let p_el: HtmlElement = p.dyn_into().unwrap();
                p_el.style().set_property("display", "none").unwrap();
            }
            return;
        }
    }
}

/// Wire hover to update the selected item (mouse interaction).
fn wire_command_hover(document: &Document) {
    let items = document.query_selector_all(".cmd-palette-item").unwrap();
    for i in 0..items.length() {
        let item = items.get(i).unwrap();
        let item_el: Element = item.dyn_into().unwrap();
        let item_el_for_listener = item_el.clone();

        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Remove selected from all
            let all = doc.query_selector_all(".cmd-palette-item").unwrap();
            for j in 0..all.length() {
                let n = all.get(j).unwrap();
                let ne: Element = n.dyn_into().unwrap();
                ne.class_list().remove_1("selected").unwrap();
            }
            // Add selected to this
            item_el.class_list().add_1("selected").unwrap();
        }) as Box<dyn FnMut(web_sys::Event)>);

        item_el_for_listener
            .add_event_listener_with_callback("mouseenter", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn toggle_palette(document: &Document) {
    if let Some(p) = document.get_element_by_id("command-palette") {
        let p_el: HtmlElement = p.dyn_into().unwrap();
        let display = p_el
            .style()
            .get_property_value("display")
            .unwrap_or_default();
        if display == "none" {
            p_el.style().set_property("display", "flex").unwrap();
            if let Some(input) = document.get_element_by_id("cmd-palette-input") {
                let input_el: HtmlInputElement = input.dyn_into().unwrap();
                input_el.set_value("");
                input_el.focus().unwrap();
            }
            // Reset to full list
            filter_results(document, "");
        } else {
            p_el.style().set_property("display", "none").unwrap();
        }
    }
}

fn wire_search_filter(document: &Document) {
    if let Some(input) = document.get_element_by_id("cmd-palette-input") {
        let input_el: HtmlInputElement = input.dyn_into().unwrap();
        let doc_clone = document.clone();
        let closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
            let input: HtmlInputElement = e.target().unwrap().dyn_into().unwrap();
            let query = input.value();
            filter_results(&doc_clone, &query);
        }) as Box<dyn FnMut(KeyboardEvent)>);
        input_el
            .add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn filter_results(document: &Document, query: &str) {
    let results = match document.get_element_by_id("cmd-palette-results") {
        Some(r) => r,
        None => return,
    };
    results.set_inner_html("");

    let commands = build_command_list();

    // Fuzzy match: score each command, sort by score descending
    let query_lower = query.to_lowercase();
    let mut scored: Vec<(f32, &CommandEntry)> = if query.is_empty() {
        commands.iter().map(|c| (1.0f32, c)).collect()
    } else {
        commands
            .iter()
            .filter_map(|c| {
                let score = fuzzy_score(&c.label.to_lowercase(), &query_lower);
                if score > 0.0 {
                    Some((score, c))
                } else {
                    None
                }
            })
            .collect()
    };
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (idx, (_, cmd)) in scored.iter().enumerate() {
        let item = document.create_element("div").unwrap();
        item.set_class_name("cmd-palette-item");
        if idx == 0 {
            item.class_list().add_1("selected").unwrap();
        }
        let item_el: HtmlElement = item.clone().dyn_into().unwrap();
        item_el.style().set_css_text(
            "padding: 8px 16px; cursor: pointer; display: flex; \
             align-items: center; gap: 8px; font-size: 12px; \
             color: var(--text-secondary); font-family: var(--font-mono);",
        );
        item.set_attribute("data-cmd-label", cmd.label).unwrap();

        let icon = document.create_element("span").unwrap();
        icon.set_text_content(Some(cmd.icon));
        let icon_el: HtmlElement = icon.clone().dyn_into().unwrap();
        icon_el
            .style()
            .set_css_text("color: var(--accent-cyan); width: 20px; text-align: center;");
        item.append_child(&icon).unwrap();

        let label = document.create_element("span").unwrap();
        label.set_text_content(Some(cmd.label));
        item.append_child(&label).unwrap();

        let shortcut = document.create_element("span").unwrap();
        shortcut.set_text_content(Some(cmd.shortcut));
        let sc_el: HtmlElement = shortcut.clone().dyn_into().unwrap();
        sc_el
            .style()
            .set_css_text("margin-left: auto; color: var(--text-muted); font-size: 10px;");
        item.append_child(&shortcut).unwrap();

        results.append_child(&item).unwrap();
    }

    // Re-wire clicks and hover for the filtered items
    wire_command_clicks(document);
    wire_command_hover(document);
}

// ---------------------------------------------------------------------------
// Fuzzy matching
// ---------------------------------------------------------------------------

/// Fuzzy match score — returns 0.0 if no match, higher is better.
///
/// Algorithm: subsequence matching with bonuses for:
/// - Consecutive character matches (+0.3 per consecutive)
/// - Word boundary starts (space, colon, or start of string) (+0.4)
/// - Early matches (first 3 chars of the target) (+0.2)
///
/// The query characters must appear in order within the target string.
fn fuzzy_score(target: &str, query: &str) -> f32 {
    if query.is_empty() {
        return 1.0;
    }

    let target_chars: Vec<char> = target.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    let mut score = 0.0f32;
    let mut ti = 0usize;
    let mut prev_matched = false;
    let mut matched_at_word_boundary = false;

    for (qi, &qc) in query_chars.iter().enumerate() {
        let mut found = false;
        while ti < target_chars.len() {
            if target_chars[ti] == qc {
                found = true;
                // Base score for matching this character
                score += 1.0;

                // Bonus: consecutive match
                if prev_matched {
                    score += 0.3;
                }

                // Bonus: word boundary (preceded by space, colon, or start)
                let is_word_boundary = ti == 0
                    || target_chars[ti - 1] == ' '
                    || target_chars[ti - 1] == ':'
                    || target_chars[ti - 1] == '-';
                if is_word_boundary {
                    score += 0.4;
                    if qi == 0 {
                        matched_at_word_boundary = true;
                    }
                }

                // Bonus: early match (first 3 chars of target)
                if ti < 3 {
                    score += 0.2;
                }

                prev_matched = true;
                ti += 1;
                break;
            }
            ti += 1;
            prev_matched = false;
        }
        if !found {
            return 0.0; // No match — query char not found in remaining target
        }
    }

    // Bonus: all query chars matched at word boundaries
    if matched_at_word_boundary && query_chars.len() > 1 {
        score += 0.5;
    }

    // Normalize by query length to prefer shorter queries
    score / query_chars.len() as f32
}

#[cfg(test)]
mod fuzzy_tests {
    use super::fuzzy_score;

    #[test]
    fn test_exact_match() {
        assert!(fuzzy_score("new document", "new document") > 0.0);
    }

    #[test]
    fn test_subsequence_match() {
        assert!(fuzzy_score("new document", "ndoc") > 0.0);
    }

    #[test]
    fn test_no_match() {
        assert_eq!(fuzzy_score("new document", "xyz"), 0.0);
    }

    #[test]
    fn test_word_boundary_scores_higher() {
        let boundary = fuzzy_score("switch manifold: research", "smr");
        let mid_word = fuzzy_score("switch manifold: research", "sch");
        assert!(
            boundary > mid_word,
            "word boundary match should score higher: {} vs {}",
            boundary,
            mid_word
        );
    }

    #[test]
    fn test_consecutive_scores_higher() {
        let consecutive = fuzzy_score("sparql", "spar");
        let scattered = fuzzy_score("sparql", "sqrl");
        assert!(
            consecutive > scattered,
            "consecutive match should score higher: {} vs {}",
            consecutive,
            scattered
        );
    }

    #[test]
    fn test_case_insensitive() {
        // The caller lowercases both target and query before calling fuzzy_score
        assert!(fuzzy_score("new document", "newdoc") > 0.0);
    }
}

fn wire_command_clicks(document: &Document) {
    let items = document.query_selector_all(".cmd-palette-item").unwrap();
    for i in 0..items.length() {
        let item = items.get(i).unwrap();
        let item_el: Element = item.dyn_into().unwrap();
        let label = item_el.get_attribute("data-cmd-label").unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            execute_command(&label);
            // Close palette after executing
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(p) = doc.get_element_by_id("command-palette") {
                    let p_el: HtmlElement = p.dyn_into().unwrap();
                    p_el.style().set_property("display", "none").unwrap();
                }
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);

        item_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn execute_command(label: &str) {
    let document = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    // Check for manifold switch commands
    if let Some(manifold_id) = label.strip_prefix("Switch Manifold: ") {
        let id = match manifold_id {
            "Research" => "research",
            "Social" => "social",
            "Knowledge" => "knowledge",
            "Projects" => "projects",
            "Rights" => "rights",
            "Sanctuary" => "sanctuary",
            "Media" => "media",
            "Communications" => "communications",
            "Settings" => "settings",
            "Vibe" => "vibe",
            _ => return,
        };
        // Simulate clicking the manifold tab
        let tabs = document.query_selector_all(".desktop-tab-btn").unwrap();
        for i in 0..tabs.length() {
            let tab = tabs.get(i).unwrap();
            let tab_el: Element = tab.dyn_into().unwrap();
            if tab_el.get_attribute("data-manifold").as_deref() == Some(id) {
                let html_el: HtmlElement = tab_el.dyn_into().unwrap();
                html_el.click();
                break;
            }
        }
        return;
    }

    if let Some(name) = label.strip_prefix("Open Construct: ") {
        let id = match name {
            "POET" => "poet",
            "Health" => "health",
            "Research lab" => "research-lab",
            "Studio" => "studio",
            "Rights" => "rights",
            "Knowledge" => "knowledge",
            "Projects" => "projects",
            _ => return,
        };
        super::open_construct(id, None);
        return;
    }
    match label {
        "Construct Shelf" => {
            super::open_construct("poet", Some("settings"));
            return;
        }
        "Anatomy manifold" => {
            super::dive_nested_manifold("anatomy");
            return;
        }
        "Author manifold" => {
            super::manifold_authoring::open_authoring_dialog_kind(&document, "lens");
            return;
        }
        "Author container" => {
            super::manifold_authoring::open_authoring_dialog_kind(&document, "container");
            return;
        }
        "Author nested link" => {
            super::manifold_authoring::open_authoring_dialog_kind(&document, "nested");
            return;
        }
        "Author subject" => {
            super::manifold_authoring::open_authoring_dialog_kind(&document, "subject");
            return;
        }
        "Pop nested manifold" => {
            super::pop_nested_manifold();
            return;
        }
        "Invite participant" => {
            super::manifold_social::open_invite_dialog(&document);
            return;
        }
        _ => {}
    }

    // Document and canvas placement commands
    match label {
        "New Document" => {
            super::interactions::place_container_via_menu(&document, "doc", "Document");
            return;
        }
        "New Sheet" => {
            super::interactions::place_container_via_menu(&document, "spreadsheet", "Spreadsheet");
            return;
        }
        "Auto-Arrange Manifold (Tidy)" | "Auto-Arrange Manifold" => {
            super::interactions::auto_arrange_manifold(&document);
            return;
        }
        _ => {}
    }

    // Search workbench commands — open the workbench (optionally to a mode)
    match label {
        "Search Workbench" => {
            super::search_workbench::toggle_search_workbench(&document);
            return;
        }
        "Faceted Search" => {
            super::search_workbench::open_to_mode(&document, "faceted");
            return;
        }
        "SPARQL Query Builder" => {
            super::search_workbench::open_to_mode(&document, "builder");
            return;
        }
        "Manual SPARQL Editor" => {
            super::search_workbench::open_to_mode(&document, "sparql");
            return;
        }
        "Saved Queries" => {
            super::search_workbench::open_to_mode(&document, "saved");
            return;
        }
        "Run SPARQL Query" => {
            super::search_workbench::open_to_mode(&document, "sparql");
            return;
        }
        _ => {}
    }

    // Logic workbench commands — dispatch to logic_workbench module
    if super::logic_workbench::dispatch_command(&document, label) {
        return;
    }

    if let Some((container_type, title)) = placements::container_for(label) {
        super::interactions::place_container_via_menu(&document, container_type, title);
        return;
    }

    // Fail closed if a catalogue entry ever drifts away from a real handler.
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 500; max-width: 320px;",
    );
    notif.set_text_content(Some(&format!(
        "Unavailable: `{label}` has no registered command implementation."
    )));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 2500);
    timeout.forget();
}

/// Toggle the visibility of the command palette overlay and focus the search input.
pub fn toggle_command_palette(document: &Document) {
    if let Some(palette) = document.get_element_by_id("command-palette") {
        let p_el: HtmlElement = palette.dyn_into().unwrap();
        let curr = p_el
            .style()
            .get_property_value("display")
            .unwrap_or_default();
        if curr == "none" || curr.is_empty() {
            p_el.style().set_property("display", "flex").unwrap();
            if let Some(input) = document.get_element_by_id("cmd-palette-input") {
                let in_el: HtmlInputElement = input.dyn_into().unwrap();
                let _ = in_el.focus();
                in_el.set_value("");
            }
        } else {
            p_el.style().set_property("display", "none").unwrap();
        }
    }
}
