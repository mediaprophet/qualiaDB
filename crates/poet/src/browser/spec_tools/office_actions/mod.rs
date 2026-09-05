//! Human-authored structural document actions.
//!
//! These actions intentionally operate only inside the selected document
//! container. Values gathered from prompts are installed with DOM APIs, never
//! interpolated into HTML.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Node, Range, Selection};

const BOOKMARK_PREFIX: &str = "poet-bookmark-";

/// Runs an Office action when this module owns `tool_id`.
///
/// `None` means another local-effect provider may handle it. `Err` means no
/// mutation was committed, including a person cancelling a prompt.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    Some(match tool_id {
        "office:insert-hyperlink" => insert_hyperlink(document, container),
        "office:insert-bookmark" => insert_bookmark(document, container),
        "office:insert-footnote" => insert_footnote(document, container),
        "office:insert-citation" => insert_citation(document, container),
        "office:insert-toc" => insert_toc(document, container),
        _ => return None,
    })
}

fn insert_hyperlink(document: &Document, container: &Element) -> Result<(), String> {
    let range = selected_range(document, container, true)?;
    let destination = prompt("Link destination (https://, http://, mailto:, tel:, or a document path):")?;
    if !safe_url(&destination) {
        return Err("Use an http(s), mailto, tel, or document-relative link address.".into());
    }
    restore_selection(&range)?;
    let link = document.create_element("a").map_err(js_error)?;
    link.set_attribute("href", destination.trim()).map_err(js_error)?;
    link.set_attribute("data-poet-hyperlink", "true").map_err(js_error)?;
    let contents = range.extract_contents().map_err(js_error)?;
    link.append_child(&contents).map_err(js_error)?;
    range.insert_node(&link).map_err(js_error)?;
    Ok(())
}

fn insert_bookmark(document: &Document, container: &Element) -> Result<(), String> {
    let range = selected_range(document, container, false)?;
    let name = prompt("Bookmark name (letters, numbers, hyphens, or underscores):")?;
    let name = valid_bookmark_name(&name).ok_or_else(|| {
        "Bookmark names use 1–64 letters, numbers, hyphens, or underscores.".to_owned()
    })?;
    let id = format!("{BOOKMARK_PREFIX}{name}");
    if document.get_element_by_id(&id).is_some() {
        return Err("That bookmark name is already in use in this document.".into());
    }
    restore_selection(&range)?;
    let marker = document.create_element("span").map_err(js_error)?;
    marker.set_attribute("id", &id).map_err(js_error)?;
    marker.set_attribute("data-poet-bookmark", name).map_err(js_error)?;
    if range.collapsed() {
        // A zero-width marker gives a stable link target without visible copy.
        marker.set_text_content(Some("\u{200b}"));
        range.insert_node(&marker).map_err(js_error)?;
    } else {
        let contents = range.extract_contents().map_err(js_error)?;
        marker.append_child(&contents).map_err(js_error)?;
        range.insert_node(&marker).map_err(js_error)?;
    }
    Ok(())
}

fn insert_footnote(document: &Document, container: &Element) -> Result<(), String> {
    let range = selected_range(document, container, true)?;
    let text = prompt("Footnote text:")?;
    if text.trim().is_empty() {
        return Err("Footnote text cannot be empty.".into());
    }
    let notes = footnote_list(document, container)?;
    let number = notes.child_element_count() + 1;
    restore_selection(&range)?;
    let marker_range = range.clone_range();
    marker_range.collapse_with_to_start(false);
    let marker = document.create_element("sup").map_err(js_error)?;
    marker.set_attribute("data-poet-footnote-ref", &number.to_string()).map_err(js_error)?;
    marker.set_text_content(Some(&number.to_string()));
    marker_range.insert_node(&marker).map_err(js_error)?;
    let item = document.create_element("li").map_err(js_error)?;
    item.set_attribute("data-poet-footnote", &number.to_string()).map_err(js_error)?;
    item.set_text_content(Some(text.trim()));
    notes.append_child(&item).map_err(js_error)?;
    Ok(())
}

fn insert_citation(document: &Document, container: &Element) -> Result<(), String> {
    let range = selected_range(document, container, true)?;
    let source = prompt("Source address for this citation:")?;
    if !safe_url(&source) {
        return Err("Use an http(s), mailto, tel, or document-relative source address.".into());
    }
    restore_selection(&range)?;
    let citation = document.create_element("cite").map_err(js_error)?;
    citation.set_attribute("data-poet-citation-source", source.trim()).map_err(js_error)?;
    citation.set_attribute("data-poet-citation", "human-supplied").map_err(js_error)?;
    let contents = range.extract_contents().map_err(js_error)?;
    citation.append_child(&contents).map_err(js_error)?;
    range.insert_node(&citation).map_err(js_error)?;
    Ok(())
}

fn insert_toc(document: &Document, container: &Element) -> Result<(), String> {
    let range = selected_range(document, container, false)?;
    let depth = prompt("Include headings through level (1–6):")?;
    let depth = depth.trim().parse::<u8>().ok().filter(|level| (1..=6).contains(level))
        .ok_or_else(|| "Choose a heading depth from 1 to 6.".to_owned())?;
    let headings = headings_at_depth(container, depth)?;
    if headings.is_empty() {
        return Err("Add a heading before building a table of contents; document unchanged.".into());
    }
    restore_selection(&range)?;
    let nav = document.create_element("nav").map_err(js_error)?;
    nav.set_attribute("data-poet-toc", "true").map_err(js_error)?;
    nav.set_attribute("aria-label", "Table of contents").map_err(js_error)?;
    let list = document.create_element("ol").map_err(js_error)?;
    for (index, heading) in headings.iter().enumerate() {
        let id = heading.get_attribute("id").filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| format!("poet-heading-{}", index + 1));
        heading.set_attribute("id", &id).map_err(js_error)?;
        let item = document.create_element("li").map_err(js_error)?;
        let link = document.create_element("a").map_err(js_error)?;
        link.set_attribute("href", &format!("#{id}")).map_err(js_error)?;
        link.set_text_content(heading.text_content().as_deref());
        item.append_child(&link).map_err(js_error)?;
        list.append_child(&item).map_err(js_error)?;
    }
    nav.append_child(&list).map_err(js_error)?;
    range.insert_node(&nav).map_err(js_error)?;
    Ok(())
}

fn selected_range(_document: &Document, container: &Element, non_empty: bool) -> Result<Range, String> {
    let selection = selection()?;
    if selection.range_count() == 0 {
        return Err("Select a place in this document first.".into());
    }
    let range = selection.get_range_at(0).map_err(js_error)?;
    if !range_belongs_to(&range, container) {
        return Err("Select writing inside the active document first.".into());
    }
    if non_empty && range.collapsed() {
        return Err("Select writing in the active document first.".into());
    }
    Ok(range.clone_range())
}

fn selection() -> Result<Selection, String> {
    web_sys::window().ok_or_else(|| "The browser window is unavailable.".to_owned())?
        .get_selection().map_err(js_error)?
        .ok_or_else(|| "Select a place in this document first.".to_owned())
}

fn restore_selection(range: &Range) -> Result<(), String> {
    let selection = selection()?;
    selection.remove_all_ranges().map_err(js_error)?;
    selection.add_range(range).map_err(js_error)
}

fn range_belongs_to(range: &Range, container: &Element) -> bool {
    let Ok(ancestor) = range.common_ancestor_container() else {
        return false;
    };
    let container_node: &Node = container.as_ref();
    container_node.is_same_node(Some(&ancestor)) || container_node.contains(Some(&ancestor))
}

fn prompt(message: &str) -> Result<String, String> {
    web_sys::window().ok_or_else(|| "The browser window is unavailable.".to_owned())?
        .prompt_with_message(message).map_err(js_error)?
        .ok_or_else(|| "Cancelled; document unchanged.".to_owned())
}

fn footnote_list(document: &Document, container: &Element) -> Result<Element, String> {
    if let Some(existing) = container.query_selector("ol[data-poet-footnotes]").map_err(js_error)? {
        return Ok(existing);
    }
    let list = document.create_element("ol").map_err(js_error)?;
    list.set_attribute("data-poet-footnotes", "true").map_err(js_error)?;
    container.append_child(&list).map_err(js_error)?;
    Ok(list)
}

fn headings_at_depth(container: &Element, depth: u8) -> Result<Vec<Element>, String> {
    let selectors = (1..=depth).map(|level| format!("h{level}")).collect::<Vec<_>>().join(",");
    let nodes = container.query_selector_all(&selectors).map_err(js_error)?;
    let mut headings = Vec::with_capacity(nodes.length() as usize);
    for index in 0..nodes.length() {
        if let Some(node) = nodes.item(index) {
            if let Ok(element) = node.dyn_into::<Element>() {
                headings.push(element);
            }
        }
    }
    Ok(headings)
}

fn safe_url(value: &str) -> bool {
    if value.is_empty() || value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    ["https://", "http://", "mailto:", "tel:", "#", "/", "./", "../"]
        .iter().any(|prefix| lower.starts_with(prefix))
}

fn valid_bookmark_name(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty() && value.len() <= 64
        && value.chars().all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')))
        .then_some(value)
}

fn js_error(error: wasm_bindgen::JsValue) -> String {
    error.as_string().unwrap_or_else(|| "The browser could not update this document.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_safe_link_schemes_or_document_paths_are_allowed() {
        assert!(safe_url("https://example.test/a"));
        assert!(safe_url("mailto:author@example.test"));
        assert!(safe_url("#chapter-one"));
        assert!(safe_url("../appendix"));
        assert!(!safe_url("javascript:alert(1)"));
        assert!(!safe_url("data:text/html,unsafe"));
        assert!(!safe_url(" https://example.test\n"));
    }

    #[test]
    fn bookmark_names_cannot_escape_an_id_attribute() {
        assert_eq!(valid_bookmark_name("chapter_1"), Some("chapter_1"));
        assert_eq!(valid_bookmark_name("chapter one"), None);
        assert_eq!(valid_bookmark_name("<script>"), None);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn selected_range_must_belong_to_its_container() {
        let document = web_sys::window().unwrap().document().unwrap();
        let owner = document.create_element("div").unwrap();
        let outsider = document.create_element("div").unwrap();
        let text = document.create_text_node("owned text");
        owner.append_child(&text).unwrap();
        let range = document.create_range().unwrap();
        range.select_node_contents(&text).unwrap();
        assert!(range_belongs_to(&range, &owner));
        assert!(!range_belongs_to(&range, &outsider));
    }
}
