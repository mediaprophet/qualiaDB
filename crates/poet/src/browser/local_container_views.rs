//! Local-first POET containers whose complete interaction model fits in the browser.

use std::{cell::Cell, rc::Rc};

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlTextAreaElement};

pub fn build_latex_view(document: &Document) -> Element {
    let wrapper = column(document);
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let editor = document.create_element("textarea").unwrap();
    editor.set_class_name("vibe-editor");
    editor.set_attribute("data-state-key", "latex-source").ok();
    let editor_input: HtmlTextAreaElement = editor.clone().dyn_into().unwrap();
    editor_input.set_value(
        "\\documentclass{article}\n\\begin{document}\n\\section{Untitled}\n\n\\end{document}",
    );
    for (label, snippet) in [
        ("\\frac", "\\frac{a}{b}"),
        ("\\sum", "\\sum_{i=0}^{n}"),
        ("\\int", "\\int_{a}^{b}"),
        ("\\sqrt", "\\sqrt{x}"),
        ("\\alpha", "\\alpha"),
        ("\\nabla", "\\nabla"),
    ] {
        let button = document.create_element("button").unwrap();
        button.set_class_name("vibe-run-btn");
        button.set_text_content(Some(label));
        let editor_clone = editor_input.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            insert_text(&editor_clone, snippet);
        }) as Box<dyn FnMut(_)>);
        button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        bar.append_child(&button).unwrap();
    }
    let cas = document.create_element("button").unwrap();
    cas.set_class_name("vibe-run-btn");
    cas.set_text_content(Some("CAS"));
    disable(
        &cas,
        "Unavailable until the SymbolicAlgebra invocation contract is registered.",
    );
    bar.append_child(&cas).unwrap();
    wrapper.append_child(&bar).unwrap();
    wrapper.append_child(&editor).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_class_name("vibe-output");
    status.set_attribute("role", "status").ok();
    update_latex_status(&editor_input, &status);
    let editor_clone = editor_input.clone();
    let status_clone = status.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        update_latex_status(&editor_clone, &status_clone);
    }) as Box<dyn FnMut(_)>);
    editor
        .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    wrapper.append_child(&status).unwrap();
    wrapper
}

fn insert_text(editor: &HtmlTextAreaElement, snippet: &str) {
    let value = editor.value();
    let start = editor
        .selection_start()
        .ok()
        .flatten()
        .unwrap_or(value.len() as u32) as usize;
    let end = editor
        .selection_end()
        .ok()
        .flatten()
        .unwrap_or(start as u32) as usize;
    let start = start.min(value.len());
    let end = end.min(value.len()).max(start);
    let next = format!("{}{}{}", &value[..start], snippet, &value[end..]);
    editor.set_value(&next);
    let cursor = (start + snippet.len()) as u32;
    editor.set_selection_range(cursor, cursor).ok();
    editor.focus().ok();
}

fn update_latex_status(editor: &HtmlTextAreaElement, status: &Element) {
    status.set_text_content(Some(&format!(
        "Local LaTeX source editor · {} characters · rendering requires a registered TeX engine",
        editor.value().chars().count()
    )));
}

pub fn build_slide_view(document: &Document) -> Element {
    let wrapper = column(document);
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let slide_area = document.create_element("div").unwrap();
    slide_area.set_class_name("poet-local-slide");
    slide_area.set_attribute("contenteditable", "true").ok();
    slide_area
        .set_attribute("data-state-key", "slide-content")
        .ok();
    slide_area.set_attribute("role", "textbox").ok();
    slide_area
        .set_attribute("aria-label", "Editable slide content")
        .ok();
    slide_area
        .set_inner_html("<h2>Untitled slide</h2><p>Click here and edit this local slide.</p>");
    let status = document.create_element("div").unwrap();
    status.set_class_name("vibe-output");
    status.set_attribute("role", "status").ok();
    status.set_text_content(Some("Slide 1 of 1 · title layout · no transition"));

    let slide_count = Rc::new(Cell::new(1usize));
    let layout_index = Rc::new(Cell::new(0usize));
    let transition_index = Rc::new(Cell::new(0usize));
    for label in ["+ Slide", "Layout", "Transition", "Present"] {
        let button = document.create_element("button").unwrap();
        button.set_class_name("vibe-run-btn");
        button.set_text_content(Some(label));
        let area = slide_area.clone();
        let status_clone = status.clone();
        let count = slide_count.clone();
        let layout = layout_index.clone();
        let transition = transition_index.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            match label {
                "+ Slide" => {
                    count.set(count.get() + 1);
                    area.set_inner_html(&format!(
                        "<h2>Untitled slide {}</h2><p>Edit this local slide.</p>",
                        count.get()
                    ));
                }
                "Layout" => layout.set((layout.get() + 1) % 3),
                "Transition" => transition.set((transition.get() + 1) % 3),
                "Present" => {
                    let _ = area.request_fullscreen();
                }
                _ => {}
            }
            let layouts = ["title", "statement", "two-column"];
            let transitions = ["none", "fade", "slide"];
            area.set_attribute("data-layout", layouts[layout.get()])
                .ok();
            area.set_attribute("data-transition", transitions[transition.get()])
                .ok();
            status_clone.set_text_content(Some(&format!(
                "Slide {} of {} · {} layout · {} transition",
                count.get(),
                count.get(),
                layouts[layout.get()],
                transitions[transition.get()]
            )));
        }) as Box<dyn FnMut(_)>);
        button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        bar.append_child(&button).unwrap();
    }
    wrapper.append_child(&bar).unwrap();
    wrapper.append_child(&slide_area).unwrap();
    wrapper.append_child(&status).unwrap();
    wrapper
}

fn column(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let element: HtmlElement = wrapper.clone().dyn_into().unwrap();
    element
        .style()
        .set_css_text("display:flex;flex-direction:column;flex:1;gap:6px;min-height:0");
    wrapper
}

fn disable(element: &Element, reason: &str) {
    element.set_attribute("disabled", "").ok();
    element.set_attribute("aria-disabled", "true").ok();
    element.set_attribute("title", reason).ok();
}
