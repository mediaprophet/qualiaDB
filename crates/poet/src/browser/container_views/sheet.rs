//! Spreadsheet container, cell editing, and formula evaluation.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Spreadsheet container — polymorphic <q-view-switcher> + formula bar + reactive grid.
pub fn build_sheet_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    // Polymorphic Projection Switcher (<q-view-switcher>)
    let view_switcher =
        crate::browser::projections::build_view_switcher(document, "mode.spreadsheet");
    wrapper.append_child(&view_switcher).unwrap();

    // Formula bar
    let formula = document.create_element("div").unwrap();
    formula.set_class_name("vibe-toolbar");
    let fx_label = document.create_element("span").unwrap();
    fx_label.set_text_content(Some("fx"));
    let fx_label_el: HtmlElement = fx_label.clone().dyn_into().unwrap();
    fx_label_el.style().set_css_text("color: var(--accent-cyan); font-family: var(--font-mono); font-size: 11px; font-weight: 700;");
    formula.append_child(&fx_label).unwrap();

    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_placeholder("=SUM(A1:A10)");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 4px 8px; color: var(--accent-emerald); font-family: var(--font-mono); font-size: 11px;").unwrap();
    formula.append_child(&input).unwrap();
    wrapper.append_child(&formula).unwrap();

    // Grid
    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el.style().set_css_text(
        "flex: 1; overflow: auto; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); font-family: var(--font-mono); font-size: 10px;"
    );

    // Header row
    let header_row = document.create_element("div").unwrap();
    let header_el: HtmlElement = header_row.clone().dyn_into().unwrap();
    header_el
        .style()
        .set_css_text("display: flex; border-bottom: 1px solid var(--border-subtle);");
    for col in &["", "A", "B", "C", "D", "E"] {
        let cell = document.create_element("div").unwrap();
        let cell_el: HtmlElement = cell.clone().dyn_into().unwrap();
        cell_el.style().set_css_text("min-width: 60px; padding: 3px 6px; text-align: center; color: var(--text-muted); border-right: 1px solid var(--border-subtle);");
        cell.set_text_content(Some(col));
        header_row.append_child(&cell).unwrap();
    }
    grid.append_child(&header_row).unwrap();

    // Data rows — cells are editable with formula support
    for row_idx in 1..=6 {
        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el
            .style()
            .set_css_text("display: flex; border-bottom: 1px solid var(--border-subtle);");
        let row_label = document.create_element("div").unwrap();
        let rl_el: HtmlElement = row_label.clone().dyn_into().unwrap();
        rl_el.style().set_css_text("min-width: 60px; padding: 3px 6px; text-align: center; color: var(--text-muted); border-right: 1px solid var(--border-subtle);");
        row_label.set_text_content(Some(&row_idx.to_string()));
        row.append_child(&row_label).unwrap();
        for col_idx in 0..5 {
            let cell = document.create_element("div").unwrap();
            cell.set_class_name("sheet-cell");
            cell.set_attribute("data-row", &row_idx.to_string())
                .unwrap();
            let col_letter = (b'A' + col_idx as u8) as char;
            cell.set_attribute("data-col", &col_letter.to_string())
                .unwrap();
            cell.set_attribute("data-cell-ref", &format!("{}{}", col_letter, row_idx))
                .unwrap();
            let cell_el: HtmlElement = cell.clone().dyn_into().unwrap();
            cell_el.style().set_css_text(
                "min-width: 60px; padding: 3px 6px; color: var(--text-secondary); \
                 border-right: 1px solid var(--border-subtle); cursor: text; \
                 transition: var(--trans-fast);",
            );
            // Seed some initial data
            if row_idx == 1 && col_idx == 0 {
                cell.set_text_content(Some("42"));
            } else if row_idx == 2 && col_idx == 0 {
                cell.set_text_content(Some("18"));
            } else if row_idx == 3 && col_idx == 0 {
                cell.set_text_content(Some("60"));
                cell.set_attribute("data-formula", "=A1+A2").unwrap();
            }
            row.append_child(&cell).unwrap();
        }
        grid.append_child(&row).unwrap();
    }
    wrapper.append_child(&grid).unwrap();

    // Wire cell editing and formula evaluation
    wire_sheet_cells(document);

    wrapper
}

/// Wire sheet cell click-to-edit and formula evaluation.
fn wire_sheet_cells(document: &Document) {
    let cells = document.query_selector_all(".sheet-cell").unwrap();
    for i in 0..cells.length() {
        let cell = cells.get(i).unwrap();
        let cell_el: Element = cell.dyn_into().unwrap();
        let cell_el_for_listener = cell_el.clone();

        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let cell_ref = cell_el.get_attribute("data-cell-ref").unwrap_or_default();
            let cell_ref_for_blur = cell_ref.clone();
            let formula = cell_el.get_attribute("data-formula").unwrap_or_default();

            // Don't re-edit if already editing
            if cell_el.class_list().contains("editing") {
                return;
            }
            cell_el.class_list().add_1("editing").unwrap();

            // Replace cell content with an input
            let current_text = cell_el.text_content().unwrap_or_default();
            let input = doc.create_element("input").unwrap();
            let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
            // Show formula if present, otherwise show value
            input_el.set_value(if !formula.is_empty() {
                &formula
            } else {
                &current_text
            });
            input.set_attribute("style",
                "width: 100%; box-sizing: border-box; background: var(--surface-panel-elevated); \
                 border: 1px solid var(--accent-cyan); border-radius: 2px; padding: 2px 4px; \
                 color: var(--accent-emerald); font-family: var(--font-mono); font-size: 10px; \
                 outline: none;"
            ).unwrap();

            cell_el.set_text_content(Some(""));
            cell_el.append_child(&input).unwrap();
            input_el.focus().unwrap();
            input_el.select();

            // On Enter or blur, commit the value
            let cell_el_for_commit = cell_el.clone();
            let input_for_commit = input.clone();
            let doc_for_commit = doc.clone();
            let commit_closure = Closure::wrap(Box::new(move |ev: web_sys::Event| {
                let ke: Option<web_sys::KeyboardEvent> = ev.dyn_into().ok();
                if let Some(ke) = &ke {
                    if ke.key() != "Enter" && ke.key() != "Tab" {
                        return;
                    }
                }

                let input_el: web_sys::HtmlInputElement =
                    input_for_commit.clone().dyn_into().unwrap();
                let new_val = input_el.value();

                // Remove the input
                input_for_commit.remove();
                cell_el_for_commit.class_list().remove_1("editing").unwrap();

                // Check if it's a formula
                if let Some(expr) = new_val.strip_prefix('=') {
                    // Store formula
                    cell_el_for_commit
                        .set_attribute("data-formula", &new_val)
                        .unwrap();
                    // Evaluate
                    let result = evaluate_formula(&doc_for_commit, expr);
                    cell_el_for_commit.set_text_content(Some(&result));
                    // Style as formula result
                    let cell_html: HtmlElement = cell_el_for_commit.clone().dyn_into().unwrap();
                    cell_html
                        .style()
                        .set_property("color", "var(--accent-emerald)")
                        .unwrap();
                } else {
                    // Plain value
                    cell_el_for_commit.remove_attribute("data-formula").unwrap();
                    cell_el_for_commit.set_text_content(Some(&new_val));
                    let cell_html: HtmlElement = cell_el_for_commit.clone().dyn_into().unwrap();
                    cell_html
                        .style()
                        .set_property("color", "var(--text-secondary)")
                        .unwrap();
                }

                // Re-evaluate any cells that reference this cell
                reevaluate_dependents(&doc_for_commit, &cell_ref);
            }) as Box<dyn FnMut(web_sys::Event)>);

            input_el
                .add_event_listener_with_callback(
                    "keydown",
                    commit_closure.as_ref().unchecked_ref(),
                )
                .unwrap();
            commit_closure.forget();

            let input_for_blur = input.clone();
            let cell_el_for_blur = cell_el.clone();
            let doc_for_blur = doc.clone();
            let blur_closure = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
                let input_el: web_sys::HtmlInputElement =
                    input_for_blur.clone().dyn_into().unwrap();
                let new_val = input_el.value();
                input_for_blur.remove();
                cell_el_for_blur.class_list().remove_1("editing").unwrap();

                if let Some(expr) = new_val.strip_prefix('=') {
                    cell_el_for_blur
                        .set_attribute("data-formula", &new_val)
                        .unwrap();
                    let result = evaluate_formula(&doc_for_blur, expr);
                    cell_el_for_blur.set_text_content(Some(&result));
                    let cell_html: HtmlElement = cell_el_for_blur.clone().dyn_into().unwrap();
                    cell_html
                        .style()
                        .set_property("color", "var(--accent-emerald)")
                        .unwrap();
                } else {
                    cell_el_for_blur.remove_attribute("data-formula").unwrap();
                    cell_el_for_blur.set_text_content(Some(&new_val));
                    let cell_html: HtmlElement = cell_el_for_blur.clone().dyn_into().unwrap();
                    cell_html
                        .style()
                        .set_property("color", "var(--text-secondary)")
                        .unwrap();
                }
                reevaluate_dependents(&doc_for_blur, &cell_ref_for_blur);
            }) as Box<dyn FnMut(web_sys::Event)>);

            input_el
                .add_event_listener_with_callback("blur", blur_closure.as_ref().unchecked_ref())
                .unwrap();
            blur_closure.forget();
        }) as Box<dyn FnMut(web_sys::Event)>);

        cell_el_for_listener
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Evaluate a simple spreadsheet formula. Supports:
/// - Cell references: A1, B2, etc.
/// - Addition: =A1+A2
/// - Subtraction: =A1-A2
/// - Multiplication: =A1*A2
/// - Division: =A1/A2
/// - SUM range: =SUM(A1:A3)
/// - Numbers: =42+8
fn evaluate_formula(document: &Document, expr: &str) -> String {
    let expr = expr.trim();

    // SUM(range) — e.g. SUM(A1:A3)
    if let Some(rest) = expr.strip_prefix("SUM(") {
        if let Some(range) = rest.strip_suffix(')') {
            if let Some((start, end)) = range.split_once(':') {
                let sum = sum_range(document, start, end);
                return format!("{}", sum);
            }
        }
        return "#NAME?".to_string();
    }

    // Simple arithmetic: split on + - * /
    // Try + and - first (lower precedence)
    if let Some((left, right)) = split_top_level(expr, '+') {
        let lv = resolve_operand(document, left.trim());
        let rv = resolve_operand(document, right.trim());
        if let (Some(a), Some(b)) = (lv, rv) {
            return format!("{}", a + b);
        }
    }
    if let Some((left, right)) = split_top_level(expr, '-') {
        let lv = resolve_operand(document, left.trim());
        let rv = resolve_operand(document, right.trim());
        if let (Some(a), Some(b)) = (lv, rv) {
            return format!("{}", a - b);
        }
    }
    // Then * and /
    if let Some((left, right)) = split_top_level(expr, '*') {
        let lv = resolve_operand(document, left.trim());
        let rv = resolve_operand(document, right.trim());
        if let (Some(a), Some(b)) = (lv, rv) {
            return format!("{}", a * b);
        }
    }
    if let Some((left, right)) = split_top_level(expr, '/') {
        let lv = resolve_operand(document, left.trim());
        let rv = resolve_operand(document, right.trim());
        if let (Some(a), Some(b)) = (lv, rv) {
            if b == 0.0 {
                return "#DIV/0!".to_string();
            }
            return format!("{}", a / b);
        }
    }

    // Single operand (cell ref or number)
    if let Some(v) = resolve_operand(document, expr) {
        return format!("{}", v);
    }

    "#VALUE!".to_string()
}

/// Split an expression at the top-level operator (not inside parentheses).
fn split_top_level(expr: &str, op: char) -> Option<(String, String)> {
    let mut depth = 0;
    for (i, c) in expr.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 && c == op => {
                return Some((expr[..i].to_string(), expr[i + c.len_utf8()..].to_string()));
            }
            _ => {}
        }
    }
    None
}

/// Resolve an operand: either a number or a cell reference (e.g. "A1").
fn resolve_operand(document: &Document, token: &str) -> Option<f64> {
    let token = token.trim();
    // Try parsing as a number
    if let Ok(n) = token.parse::<f64>() {
        return Some(n);
    }
    // Try as a cell reference
    get_cell_value(document, token)
}

/// Get the numeric value of a cell by reference (e.g. "A1").
fn get_cell_value(document: &Document, cell_ref: &str) -> Option<f64> {
    let selector = format!(".sheet-cell[data-cell-ref=\"{}\"]", cell_ref);
    if let Some(cell) = document.query_selector(&selector).unwrap() {
        let text = cell.text_content().unwrap_or_default();
        return text.trim().parse::<f64>().ok();
    }
    None
}

/// Sum a range of cells (e.g. from "A1" to "A3").
fn sum_range(document: &Document, start: &str, end: &str) -> f64 {
    // Parse cell refs: column letter + row number
    let (start_col, start_row) = parse_cell_ref(start);
    let (end_col, end_row) = parse_cell_ref(end);

    let mut sum = 0.0;
    for row in start_row..=end_row {
        for col in start_col..=end_col {
            let col_letter = (b'A' + col) as char;
            let ref_str = format!("{}{}", col_letter, row);
            if let Some(v) = get_cell_value(document, &ref_str) {
                sum += v;
            }
        }
    }
    sum
}

/// Parse a cell reference like "A1" into (col_index, row_number).
fn parse_cell_ref(ref_str: &str) -> (u8, u32) {
    let mut col = 0u8;
    let mut row_str = String::new();
    for c in ref_str.chars() {
        if c.is_ascii_alphabetic() {
            col = c.to_ascii_uppercase() as u8 - b'A';
        } else if c.is_ascii_digit() {
            row_str.push(c);
        }
    }
    let row = row_str.parse::<u32>().unwrap_or(1);
    (col, row)
}

/// Re-evaluate cells that have formulas and might depend on the changed cell.
fn reevaluate_dependents(document: &Document, _changed_cell: &str) {
    let cells = document
        .query_selector_all(".sheet-cell[data-formula]")
        .unwrap();
    for i in 0..cells.length() {
        let cell = cells.get(i).unwrap();
        let cell_el: Element = cell.dyn_into().unwrap();
        let formula = cell_el.get_attribute("data-formula").unwrap_or_default();
        if let Some(expr) = formula.strip_prefix('=') {
            let result = evaluate_formula(document, expr);
            cell_el.set_text_content(Some(&result));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_cell_ref, split_top_level};

    #[test]
    fn parse_cell_ref_reads_column_and_row() {
        assert_eq!(parse_cell_ref("A1"), (0, 1));
        assert_eq!(parse_cell_ref("C12"), (2, 12));
    }

    #[test]
    fn split_top_level_ignores_parens() {
        assert_eq!(
            split_top_level("A1+(B2+C3)", '+')
                .as_ref()
                .map(|(l, r)| (l.as_str(), r.as_str())),
            Some(("A1", "(B2+C3)"))
        );
        assert!(split_top_level("(A1+B2)", '+').is_none());
    }
}
