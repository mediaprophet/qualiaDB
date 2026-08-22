//! Advanced Multi-Modal Clipboard & Provenance Subsystem (Spec 16).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Provides a 64-slot history ring buffer, multi-flavor payload encapsulation,
//! "Paste As..." contextual polymorphic conversion, cryptographic author provenance,
//! and the multi-item "Collect & Paste" staging tray.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

pub const MAX_RING_BUFFER_CAPACITY: usize = 64;

/// Supported clipboard mime/modality flavors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClipFlavor {
    PlainText,
    HtmlWithRdfa,
    CmlSemanticSpan,
    RdfNQuins,
    VibeScriptAst,
    SpreadsheetGrid,
    TenDMesh,
    P64AudioTensor,
    ContainerDefinition,
}

/// Category of the copied item for UI filtering and icon display.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClipCategory {
    Text,
    Entity,
    Code,
    Media,
    Table,
    Container,
}

impl ClipCategory {
    pub fn glyph(&self) -> &'static str {
        match self {
            Self::Text => "\u{1F4C4}",      // 📄
            Self::Entity => "\u{1F3F7}\u{FE0F}", // 🏷️
            Self::Code => "\u{26A1}",        // ⚡
            Self::Media => "\u{1F9CA}",       // 🧊
            Self::Table => "\u{1F4CA}",       // 📊
            Self::Container => "\u{1F4E6}",   // 📦
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Text => "Text Span",
            Self::Entity => "CML Entity",
            Self::Code => "Vibe Code",
            Self::Media => "3D / Audio Asset",
            Self::Table => "Data Grid",
            Self::Container => "Container State",
        }
    }
}

/// Preview metadata for clipboard items.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipPreview {
    pub title: String,
    pub snippet: String,
    pub category: ClipCategory,
    pub item_count: usize,
}

/// An item in the Multi-Modal Clipboard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub clip_id: u64,
    pub timestamp_ms: u64,
    pub author_did: String,
    pub provenance_uri: Option<String>,
    pub sensitivity: u8, // 0 = Public, 1 = Restricted, 2 = Classified (Sanctuary)
    pub is_pinned: bool,
    pub flavors: HashMap<ClipFlavor, Vec<u8>>,
    pub preview: ClipPreview,
}

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

impl ClipboardItem {
    /// Create a new plain text clipboard item.
    pub fn new_text(text: &str, author_did: &str) -> Self {
        let mut flavors = HashMap::new();
        flavors.insert(ClipFlavor::PlainText, text.as_bytes().to_vec());
        Self {
            clip_id: fnv1a_hash(text.as_bytes()),
            timestamp_ms: 1774000000000,
            author_did: author_did.to_string(),
            provenance_uri: None,
            sensitivity: 0,
            is_pinned: false,
            flavors,
            preview: ClipPreview {
                title: if text.len() > 30 { format!("{}...", &text[..30]) } else { text.to_string() },
                snippet: text.to_string(),
                category: ClipCategory::Text,
                item_count: 1,
            },
        }
    }

    /// Create a rich CML entity item.
    pub fn new_cml_entity(
        entity_name: &str,
        entity_category: &str,
        author_did: &str,
        source_doc_uri: &str,
    ) -> Self {
        let mut flavors = HashMap::new();
        let plain = format!("{}: {}", entity_category, entity_name);
        let cml_json = format!(r#"{{"entity":"{}","category":"{}"}}"#, entity_name, entity_category);
        let rdf = format!(r#"<did:q42:entity:{}> a qualia:{} ."#, entity_name, entity_category);

        flavors.insert(ClipFlavor::PlainText, plain.as_bytes().to_vec());
        flavors.insert(ClipFlavor::CmlSemanticSpan, cml_json.into_bytes());
        flavors.insert(ClipFlavor::RdfNQuins, rdf.into_bytes());

        Self {
            clip_id: fnv1a_hash(entity_name.as_bytes()),
            timestamp_ms: 1774000000000,
            author_did: author_did.to_string(),
            provenance_uri: Some(source_doc_uri.to_string()),
            sensitivity: 0,
            is_pinned: false,
            flavors,
            preview: ClipPreview {
                title: format!("🏷️ {}", entity_name),
                snippet: format!("Category: {} · Provenance: {}", entity_category, source_doc_uri),
                category: ClipCategory::Entity,
                item_count: 1,
            },
        }
    }

    /// Add a flavor representation to the item.
    pub fn add_flavor(&mut self, flavor: ClipFlavor, bytes: Vec<u8>) {
        self.flavors.insert(flavor, bytes);
    }
}

/// 64-Slot Bounded Clipboard Ring Buffer.
#[derive(Clone, Debug, Default)]
pub struct ClipboardRingBuffer {
    items: Vec<ClipboardItem>,
}

impl ClipboardRingBuffer {
    pub fn new() -> Self {
        Self {
            items: Vec::with_capacity(MAX_RING_BUFFER_CAPACITY),
        }
    }

    /// Push an item onto the front of the clipboard.
    /// If capacity is exceeded, evicts the oldest unpinned item.
    pub fn push(&mut self, item: ClipboardItem) {
        // Remove duplicate if same clip_id already exists
        self.items.retain(|existing| existing.clip_id != item.clip_id);

        if self.items.len() >= MAX_RING_BUFFER_CAPACITY {
            // Find oldest unpinned item to evict (from back)
            if let Some(pos) = self.items.iter().rposition(|i| !i.is_pinned) {
                self.items.remove(pos);
            } else {
                // If all are pinned, pop the very oldest
                self.items.pop();
            }
        }
        self.items.insert(0, item);
    }

    /// List all clipboard items.
    pub fn items(&self) -> &[ClipboardItem] {
        &self.items
    }

    /// Toggle pinned state for an item.
    pub fn toggle_pin(&mut self, clip_id: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.clip_id == clip_id) {
            item.is_pinned = !item.is_pinned;
        }
    }

    /// Remove a specific item.
    pub fn remove(&mut self, clip_id: u64) {
        self.items.retain(|i| i.clip_id != clip_id);
    }

    /// Filter items by text query and category.
    pub fn query(&self, query: &str, category: Option<ClipCategory>) -> Vec<&ClipboardItem> {
        let q = query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                if let Some(cat) = category {
                    if item.preview.category != cat {
                        return false;
                    }
                }
                if q.is_empty() {
                    true
                } else {
                    item.preview.title.to_lowercase().contains(&q)
                        || item.preview.snippet.to_lowercase().contains(&q)
                        || item.author_did.to_lowercase().contains(&q)
                }
            })
            .collect()
    }
}

/// "Paste As..." negotiation options for target containers.
#[derive(Clone, Debug, PartialEq)]
pub enum PasteAsOption {
    PlainText,
    RichCmlEntity,
    VerifiableCitation,
    RdfTriples,
    ReactiveVibeCell,
    MarkdownTable,
}

impl PasteAsOption {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PlainText => "Plain Text (Strip formatting)",
            Self::RichCmlEntity => "Interactive CML Entity (<q-entity>)",
            Self::VerifiableCitation => "Verifiable Citation (<q-citation> with DID)",
            Self::RdfTriples => "RDF Triples (Insert into Local Graph)",
            Self::ReactiveVibeCell => "Reactive Vibe Cell (<q-cell>)",
            Self::MarkdownTable => "Markdown GFM Table",
        }
    }
}

/// Negotiate available paste options given a clipboard item.
pub fn available_paste_options(item: &ClipboardItem) -> Vec<PasteAsOption> {
    let mut options = vec![PasteAsOption::PlainText];

    if item.flavors.contains_key(&ClipFlavor::CmlSemanticSpan) {
        options.push(PasteAsOption::RichCmlEntity);
    }
    if item.provenance_uri.is_some() {
        options.push(PasteAsOption::VerifiableCitation);
    }
    if item.flavors.contains_key(&ClipFlavor::RdfNQuins) {
        options.push(PasteAsOption::RdfTriples);
    }
    if item.flavors.contains_key(&ClipFlavor::VibeScriptAst) {
        options.push(PasteAsOption::ReactiveVibeCell);
    }
    if item.flavors.contains_key(&ClipFlavor::SpreadsheetGrid) {
        options.push(PasteAsOption::MarkdownTable);
    }

    options
}

/// Multi-item staging tray ("Collect & Paste").
#[derive(Clone, Debug, Default)]
pub struct ClipCollectTray {
    staged: Vec<ClipboardItem>,
}

impl ClipCollectTray {
    pub fn new() -> Self {
        Self { staged: Vec::new() }
    }

    pub fn add(&mut self, item: ClipboardItem) {
        self.staged.push(item);
    }

    pub fn items(&self) -> &[ClipboardItem] {
        &self.staged
    }

    pub fn clear(&mut self) {
        self.staged.clear();
    }

    /// Concatenate all staged plain text items with newline separation.
    pub fn paste_all_text(&self) -> String {
        self.staged
            .iter()
            .filter_map(|item| {
                item.flavors
                    .get(&ClipFlavor::PlainText)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Quick Clipboard Ring HUD Modal (`Ctrl+Shift+V`).
pub fn build_clipboard_hud_modal(document: &Document, ring: &ClipboardRingBuffer) -> Element {
    let modal = document.create_element("div").unwrap();
    modal.set_class_name("poet-clipboard-hud-modal");
    let modal_el: HtmlElement = modal.clone().dyn_into().unwrap();
    modal_el.style().set_css_text(
        "position: fixed; top: 15%; left: 50%; transform: translateX(-50%); \
         width: 560px; max-height: 480px; background: rgba(15, 23, 42, 0.95); \
         backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.15); \
         border-radius: 10px; box-shadow: 0 20px 40px rgba(0, 0, 0, 0.7); \
         display: flex; flex-direction: column; z-index: 9999; color: #f8fafc; font-family: sans-serif;"
    );

    // Search bar header
    let header = document.create_element("div").unwrap();
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "display: flex; gap: 8px; padding: 12px; border-bottom: 1px solid rgba(255, 255, 255, 0.08);"
    );

    let search_input = document.create_element("input").unwrap();
    search_input.set_attribute("type", "text").unwrap();
    search_input.set_attribute("placeholder", "Search clipboard history... (Ctrl+Shift+V)").unwrap();
    let search_input_el: HtmlElement = search_input.clone().dyn_into().unwrap();
    search_input_el.style().set_css_text(
        "flex: 1; background: rgba(30, 41, 59, 0.7); border: 1px solid rgba(255, 255, 255, 0.12); \
         border-radius: 6px; padding: 8px 12px; color: #fff; font-size: 13px; outline: none;"
    );
    header.append_child(&search_input).unwrap();
    modal.append_child(&header).unwrap();

    // List container
    let list = document.create_element("div").unwrap();
    let list_el: HtmlElement = list.clone().dyn_into().unwrap();
    list_el.style().set_css_text("display: flex; flex-direction: column; overflow-y: auto; padding: 6px; gap: 4px;");

    for (idx, item) in ring.items().iter().enumerate() {
        let row = document.create_element("div").unwrap();
        row.set_class_name("poet-clip-row");
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el.style().set_css_text(
            "display: flex; align-items: center; justify-content: space-between; \
             padding: 8px 10px; background: rgba(30, 41, 59, 0.4); border-radius: 6px; \
             cursor: pointer; transition: background 0.15s ease;"
        );

        let left = document.create_element("div").unwrap();
        let left_el: HtmlElement = left.clone().dyn_into().unwrap();
        left_el.style().set_css_text("display: flex; align-items: center; gap: 8px; overflow: hidden;");

        let glyph = document.create_element("span").unwrap();
        glyph.set_text_content(Some(item.preview.category.glyph()));
        left.append_child(&glyph).unwrap();

        let text_box = document.create_element("div").unwrap();
        let text_box_el: HtmlElement = text_box.clone().dyn_into().unwrap();
        text_box_el.style().set_css_text("display: flex; flex-direction: column; overflow: hidden;");

        let title = document.create_element("span").unwrap();
        title.set_text_content(Some(&item.preview.title));
        let title_el: HtmlElement = title.clone().dyn_into().unwrap();
        title_el.style().set_css_text("font-size: 12px; font-weight: 500; text-overflow: ellipsis; white-space: nowrap;");
        text_box.append_child(&title).unwrap();

        let meta = document.create_element("span").unwrap();
        meta.set_text_content(Some(&format!("#{}: {}", idx + 1, item.preview.snippet)));
        let meta_el: HtmlElement = meta.clone().dyn_into().unwrap();
        meta_el.style().set_css_text("font-size: 10px; color: #94a3b8; text-overflow: ellipsis; white-space: nowrap;");
        text_box.append_child(&meta).unwrap();

        left.append_child(&text_box).unwrap();
        row.append_child(&left).unwrap();

        // Pin badge
        if item.is_pinned {
            let pin = document.create_element("span").unwrap();
            pin.set_text_content(Some("\u{1F4CC}"));
            pin.set_attribute("title", "Pinned Item").unwrap();
            row.append_child(&pin).unwrap();
        }

        list.append_child(&row).unwrap();
    }

    modal.append_child(&list).unwrap();
    modal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_ring_capacity_and_eviction() {
        let mut ring = ClipboardRingBuffer::new();
        for i in 0..70 {
            let item = ClipboardItem::new_text(&format!("Item {}", i), "did:q42:author");
            ring.push(item);
        }
        assert_eq!(ring.items().len(), MAX_RING_BUFFER_CAPACITY);
        // Latest item is at index 0
        assert_eq!(ring.items()[0].preview.title, "Item 69");
    }

    #[test]
    fn test_clipboard_pinning_prevents_eviction() {
        let mut ring = ClipboardRingBuffer::new();
        let mut pinned_item = ClipboardItem::new_text("Important Pinned", "did:q42:author");
        pinned_item.is_pinned = true;
        let pinned_id = pinned_item.clip_id;
        ring.push(pinned_item);

        for i in 0..70 {
            let item = ClipboardItem::new_text(&format!("Item {}", i), "did:q42:author");
            ring.push(item);
        }

        assert_eq!(ring.items().len(), MAX_RING_BUFFER_CAPACITY);
        // The pinned item should still be in the ring!
        assert!(ring.items().iter().any(|i| i.clip_id == pinned_id));
    }

    #[test]
    fn test_paste_as_negotiation() {
        let cml_item = ClipboardItem::new_cml_entity(
            "CatchmentArea",
            "Hydrology",
            "did:q42:timothy",
            "qualia:doc:water_01",
        );
        let options = available_paste_options(&cml_item);
        assert!(options.contains(&PasteAsOption::PlainText));
        assert!(options.contains(&PasteAsOption::RichCmlEntity));
        assert!(options.contains(&PasteAsOption::VerifiableCitation));
        assert!(options.contains(&PasteAsOption::RdfTriples));
    }

    #[test]
    fn test_clip_collect_tray() {
        let mut tray = ClipCollectTray::new();
        tray.add(ClipboardItem::new_text("Paragraph One", "did:q42:author"));
        tray.add(ClipboardItem::new_text("Paragraph Two", "did:q42:author"));

        assert_eq!(tray.items().len(), 2);
        let combined = tray.paste_all_text();
        assert_eq!(combined, "Paragraph One\n\nParagraph Two");
    }
}
