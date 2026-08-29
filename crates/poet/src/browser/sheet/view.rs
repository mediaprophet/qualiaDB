//! Browser rendering and interaction wiring for the spreadsheet container.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{ClipboardEvent, Document, Element, HtmlInputElement, KeyboardEvent, MouseEvent};

use super::{
    formula::display_value,
    model::{cell_ref, col_label, parse_cell_ref, SheetState},
    ui::{button, element, focus_cell, persist, refresh_values, status_text, update_selection},
};

pub fn build(document: &Document, settings: &BTreeMap<String, String>) -> Element {
    let state = Rc::new(RefCell::new(SheetState::from_settings(settings)));
    let active = Rc::new(RefCell::new(String::from("A1")));
    let root = element(document, "div", "sheet-workspace");
    root.set_attribute("data-sheet-root", "true").unwrap();

    root.append_child(&super::super::projections::build_view_switcher(
        document,
        "mode.spreadsheet",
    ))
    .unwrap();

    let toolbar = element(document, "div", "sheet-toolbar");
    let add_row = button(document, "+ Row", "Add a row");
    let add_col = button(document, "+ Column", "Add a column");
    let clear = button(document, "Clear", "Clear the selected cell");
    let help = element(document, "span", "sheet-help");
    help.set_text_content(Some("Enter/Tab moves · paste tables from a spreadsheet"));
    for control in [&add_row, &add_col, &clear, &help] {
        toolbar.append_child(control).unwrap();
    }
    root.append_child(&toolbar).unwrap();

    let formula_row = element(document, "div", "sheet-formula-row");
    let name = document.create_element("input").unwrap();
    let name_input: HtmlInputElement = name.clone().dyn_into().unwrap();
    name_input.set_value("A1");
    name_input.set_read_only(true);
    name.set_class_name("sheet-name-box");
    name.set_attribute("aria-label", "Selected cell").unwrap();
    name.set_attribute("data-state-key", "sheet-selected-cell-v2")
        .unwrap();
    let fx = element(document, "span", "sheet-fx");
    fx.set_text_content(Some("fx"));
    let formula = document.create_element("input").unwrap();
    let formula_input: HtmlInputElement = formula.clone().dyn_into().unwrap();
    formula_input.set_placeholder("Enter a value or =SUM(A1:A10)");
    formula.set_class_name("sheet-formula-input");
    formula.set_attribute("aria-label", "Formula bar").unwrap();
    formula
        .set_attribute("data-state-key", "sheet-formula-editor-v2")
        .unwrap();
    formula_row.append_child(&name).unwrap();
    formula_row.append_child(&fx).unwrap();
    formula_row.append_child(&formula).unwrap();
    root.append_child(&formula_row).unwrap();

    let viewport = element(document, "div", "sheet-grid-viewport");
    let grid = element(document, "div", "sheet-grid");
    viewport.append_child(&grid).unwrap();
    root.append_child(&viewport).unwrap();

    let status = element(document, "div", "sheet-status");
    root.append_child(&status).unwrap();

    render_grid(
        document,
        &root,
        &grid,
        &formula_input,
        &name_input,
        &status,
        &state,
        &active,
    );
    wire_toolbar(
        document,
        &root,
        &grid,
        &formula_input,
        &name_input,
        &status,
        &state,
        &active,
        &add_row,
        &add_col,
        &clear,
    );
    wire_formula_bar(&root, &formula_input, &state, &active, &status);
    wire_paste(
        document,
        &root,
        &grid,
        &formula_input,
        &name_input,
        &status,
        &state,
        &active,
    );
    root
}

#[allow(clippy::too_many_arguments)]
fn render_grid(
    document: &Document,
    root: &Element,
    grid: &Element,
    formula: &HtmlInputElement,
    name: &HtmlInputElement,
    status: &Element,
    state: &Rc<RefCell<SheetState>>,
    active: &Rc<RefCell<String>>,
) {
    grid.set_inner_html("");
    let snapshot = state.borrow().clone();
    grid.set_attribute(
        "style",
        &format!(
            "grid-template-columns: 42px repeat({}, minmax(88px, 1fr));",
            snapshot.cols
        ),
    )
    .unwrap();
    grid.append_child(&element(document, "div", "sheet-corner"))
        .unwrap();
    for col in 0..snapshot.cols {
        let header = element(document, "div", "sheet-column-header");
        header.set_text_content(Some(&col_label(col)));
        grid.append_child(&header).unwrap();
    }
    for row in 0..snapshot.rows {
        let header = element(document, "div", "sheet-row-header");
        header.set_text_content(Some(&(row + 1).to_string()));
        grid.append_child(&header).unwrap();
        for col in 0..snapshot.cols {
            let reference = cell_ref(col, row);
            let cell = document.create_element("input").unwrap();
            let input: HtmlInputElement = cell.clone().dyn_into().unwrap();
            input.set_value(&display_value(&snapshot, &reference));
            cell.set_class_name("sheet-cell");
            cell.set_attribute("data-cell-ref", &reference).unwrap();
            cell.set_attribute("data-state-key", &format!("sheet-cell-v2-{reference}"))
                .unwrap();
            cell.set_attribute("aria-label", &format!("Cell {reference}"))
                .unwrap();
            if snapshot.raw(&reference).starts_with('=') {
                cell.class_list().add_1("formula").unwrap();
            }
            wire_cell(
                root, &input, formula, name, status, state, active, &reference,
            );
            grid.append_child(&cell).unwrap();
        }
    }
    update_selection(root, formula, name, status, state, active);
}

fn wire_cell(
    root: &Element,
    input: &HtmlInputElement,
    formula: &HtmlInputElement,
    name: &HtmlInputElement,
    status: &Element,
    state: &Rc<RefCell<SheetState>>,
    active: &Rc<RefCell<String>>,
    reference: &str,
) {
    let input_focus = input.clone();
    let formula_focus = formula.clone();
    let name_focus = name.clone();
    let status_focus = status.clone();
    let root_focus = root.clone();
    let state_focus = Rc::clone(state);
    let active_focus = Rc::clone(active);
    let reference_focus = reference.to_string();
    let focus = Closure::wrap(Box::new(move || {
        *active_focus.borrow_mut() = reference_focus.clone();
        input_focus.set_value(state_focus.borrow().raw(&reference_focus));
        update_selection(
            &root_focus,
            &formula_focus,
            &name_focus,
            &status_focus,
            &state_focus,
            &active_focus,
        );
    }) as Box<dyn FnMut()>);
    input
        .add_event_listener_with_callback("focus", focus.as_ref().unchecked_ref())
        .unwrap();
    focus.forget();

    let input_live = input.clone();
    let formula_live = formula.clone();
    let input_event = Closure::wrap(Box::new(move || {
        formula_live.set_value(&input_live.value());
    }) as Box<dyn FnMut()>);
    input
        .add_event_listener_with_callback("input", input_event.as_ref().unchecked_ref())
        .unwrap();
    input_event.forget();

    let input_blur = input.clone();
    let root_blur = root.clone();
    let state_blur = Rc::clone(state);
    let reference_blur = reference.to_string();
    let blur = Closure::wrap(Box::new(move || {
        let value = input_blur.value();
        if state_blur.borrow().raw(&reference_blur) != value {
            state_blur.borrow_mut().set(&reference_blur, value);
            persist(&root_blur, &state_blur.borrow(), "edit sheet cell");
        }
        refresh_values(&root_blur, &state_blur.borrow());
    }) as Box<dyn FnMut()>);
    input
        .add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref())
        .unwrap();
    blur.forget();

    let input_key = input.clone();
    let root_key = root.clone();
    let state_key = Rc::clone(state);
    let active_key = Rc::clone(active);
    let reference_key = reference.to_string();
    let keydown = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        if event.key() != "Enter" && event.key() != "Tab" {
            return;
        }
        event.prevent_default();
        state_key
            .borrow_mut()
            .set(&reference_key, input_key.value());
        persist(&root_key, &state_key.borrow(), "edit sheet cell");
        let (col, row) = parse_cell_ref(&reference_key).unwrap_or((0, 0));
        let snapshot = state_key.borrow();
        let (next_col, next_row) = if event.key() == "Tab" {
            if event.shift_key() {
                (col.saturating_sub(1), row)
            } else if col + 1 < snapshot.cols {
                (col + 1, row)
            } else {
                (0, (row + 1).min(snapshot.rows - 1))
            }
        } else if event.shift_key() {
            (col, row.saturating_sub(1))
        } else {
            (col, (row + 1).min(snapshot.rows - 1))
        };
        drop(snapshot);
        let next = cell_ref(next_col, next_row);
        *active_key.borrow_mut() = next.clone();
        refresh_values(&root_key, &state_key.borrow());
        focus_cell(&root_key, &next, &state_key.borrow());
    }) as Box<dyn FnMut(KeyboardEvent)>);
    input
        .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
        .unwrap();
    keydown.forget();
}

#[allow(clippy::too_many_arguments)]
fn wire_toolbar(
    document: &Document,
    root: &Element,
    grid: &Element,
    formula: &HtmlInputElement,
    name: &HtmlInputElement,
    status: &Element,
    state: &Rc<RefCell<SheetState>>,
    active: &Rc<RefCell<String>>,
    add_row: &Element,
    add_col: &Element,
    clear: &Element,
) {
    for (button, add_column) in [(add_row, false), (add_col, true)] {
        let document = document.clone();
        let root = root.clone();
        let grid = grid.clone();
        let formula = formula.clone();
        let name = name.clone();
        let status = status.clone();
        let state = Rc::clone(state);
        let active = Rc::clone(active);
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            event.prevent_default();
            event.stop_propagation();
            let changed = if add_column {
                state.borrow_mut().add_col()
            } else {
                state.borrow_mut().add_row()
            };
            if changed {
                persist(&root, &state.borrow(), "resize sheet");
                render_grid(
                    &document, &root, &grid, &formula, &name, &status, &state, &active,
                );
            }
        }) as Box<dyn FnMut(MouseEvent)>);
        button
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    let root_clear = root.clone();
    let formula_clear = formula.clone();
    let state_clear = Rc::clone(state);
    let active_clear = Rc::clone(active);
    let clear_closure = Closure::wrap(Box::new(move |event: MouseEvent| {
        event.prevent_default();
        event.stop_propagation();
        let selected = active_clear.borrow().clone();
        state_clear.borrow_mut().set(&selected, String::new());
        formula_clear.set_value("");
        let selector = format!(".sheet-cell[data-cell-ref=\"{selected}\"]");
        if let Ok(Some(cell)) = root_clear.query_selector(&selector) {
            if let Ok(input) = cell.dyn_into::<HtmlInputElement>() {
                input.set_value("");
            }
        }
        persist(&root_clear, &state_clear.borrow(), "clear sheet cell");
        refresh_values(&root_clear, &state_clear.borrow());
        focus_cell(&root_clear, &selected, &state_clear.borrow());
    }) as Box<dyn FnMut(MouseEvent)>);
    clear
        .add_event_listener_with_callback("mousedown", clear_closure.as_ref().unchecked_ref())
        .unwrap();
    clear_closure.forget();
}

fn wire_formula_bar(
    root: &Element,
    formula: &HtmlInputElement,
    state: &Rc<RefCell<SheetState>>,
    active: &Rc<RefCell<String>>,
    status: &Element,
) {
    let commit = |formula: HtmlInputElement,
                  root: Element,
                  status: Element,
                  state: Rc<RefCell<SheetState>>,
                  active: Rc<RefCell<String>>| {
        let selected = active.borrow().clone();
        let value = formula.value();
        if state.borrow().raw(&selected) != value {
            state.borrow_mut().set(&selected, value);
            persist(&root, &state.borrow(), "edit sheet formula");
            refresh_values(&root, &state.borrow());
            status.set_text_content(Some(&status_text(&state.borrow(), &selected)));
        }
    };

    let formula_key = formula.clone();
    let root_key = root.clone();
    let status_key = status.clone();
    let state_key = Rc::clone(state);
    let active_key = Rc::clone(active);
    let keydown = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        if event.key() == "Enter" {
            event.prevent_default();
            commit(
                formula_key.clone(),
                root_key.clone(),
                status_key.clone(),
                Rc::clone(&state_key),
                Rc::clone(&active_key),
            );
            focus_cell(&root_key, &active_key.borrow(), &state_key.borrow());
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);
    formula
        .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
        .unwrap();
    keydown.forget();

    let formula_blur = formula.clone();
    let root_blur = root.clone();
    let status_blur = status.clone();
    let state_blur = Rc::clone(state);
    let active_blur = Rc::clone(active);
    let blur = Closure::wrap(Box::new(move || {
        commit(
            formula_blur.clone(),
            root_blur.clone(),
            status_blur.clone(),
            Rc::clone(&state_blur),
            Rc::clone(&active_blur),
        );
    }) as Box<dyn FnMut()>);
    formula
        .add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref())
        .unwrap();
    blur.forget();
}

#[allow(clippy::too_many_arguments)]
fn wire_paste(
    document: &Document,
    root: &Element,
    grid: &Element,
    formula: &HtmlInputElement,
    name: &HtmlInputElement,
    status: &Element,
    state: &Rc<RefCell<SheetState>>,
    active: &Rc<RefCell<String>>,
) {
    let document = document.clone();
    let root_for_event = root.clone();
    let grid = grid.clone();
    let formula = formula.clone();
    let name = name.clone();
    let status = status.clone();
    let state = Rc::clone(state);
    let active = Rc::clone(active);
    let paste = Closure::wrap(Box::new(move |event: ClipboardEvent| {
        let Some(clipboard) = event.clipboard_data() else {
            return;
        };
        let Ok(text) = clipboard.get_data("text/plain") else {
            return;
        };
        if !text.contains('\t') && !text.contains('\n') && !text.contains('\r') {
            return;
        }
        event.prevent_default();
        let selected = active.borrow().clone();
        state.borrow_mut().paste_tsv(&selected, &text);
        persist(&root_for_event, &state.borrow(), "paste sheet cells");
        render_grid(
            &document,
            &root_for_event,
            &grid,
            &formula,
            &name,
            &status,
            &state,
            &active,
        );
        focus_cell(&root_for_event, &selected, &state.borrow());
    }) as Box<dyn FnMut(ClipboardEvent)>);
    root.add_event_listener_with_callback("paste", paste.as_ref().unchecked_ref())
        .unwrap();
    paste.forget();
}
