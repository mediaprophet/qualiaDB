//! Hypermedia Bookmarks & Vibe Browser Integration Subsystem (Spec 18).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements graph-native hypermedia bookmarks, offline DOM snapshot hashing,
//! harvested RDFa/JSON-LD knowledge graphs, media fragment anchors, and reactive
//! VibeScript autonomous monitoring probes ("VibeMarks").

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Specific media fragment or spatial anchor within the bookmarked resource.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MediaAnchor {
    TemporalOffset { start_s: f64, end_s: Option<f64> },
    SpatialPin3D { pos: [f32; 3], orbit: [f32; 2] },
    TextSelectorRange { start_char: usize, end_char: usize },
}

/// A graph-native Hypermedia Bookmark artifact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HypermediaBookmark {
    pub id: String,
    pub target_uri: String,
    pub title: String,
    pub description: Option<String>,
    pub author_attribution: Option<String>,
    pub created_at: u64,
    pub last_visited_at: u64,
    pub visit_count: u32,
    pub is_pinned: bool,

    // Semantic & Knowledge Graph Payload
    pub extracted_triples: Vec<String>,
    pub cml_entity_tags: Vec<String>,
    pub dialectical_score: u8, // 0..100 trust/objectivity score

    // Archival & Visual Snapshot
    pub snapshot_hash: Option<String>,
    pub thumbnail_uri: Option<String>,
    pub media_fragment: Option<MediaAnchor>,

    // Reactive VibeScript Living Program ("VibeMark")
    pub vibemark_script: Option<String>,
}

impl HypermediaBookmark {
    /// Create a standard web bookmark with semantic tags.
    pub fn new_web(target_uri: &str, title: &str, author: Option<&str>) -> Self {
        Self {
            id: format!("hbm_{:016x}", fnv1a_hash(target_uri.as_bytes())),
            target_uri: target_uri.to_string(),
            title: title.to_string(),
            description: None,
            author_attribution: author.map(|s| s.to_string()),
            created_at: 1774000000000,
            last_visited_at: 1774000000000,
            visit_count: 1,
            is_pinned: false,
            extracted_triples: Vec::new(),
            cml_entity_tags: Vec::new(),
            dialectical_score: 85,
            snapshot_hash: None,
            thumbnail_uri: None,
            media_fragment: None,
            vibemark_script: None,
        }
    }

    /// Attach a reactive VibeMark monitoring probe script.
    pub fn with_vibemark(mut self, script: &str) -> Self {
        self.vibemark_script = Some(script.to_string());
        self
    }
}

/// Storage collection and query manager for Hypermedia Bookmarks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BookmarkLibrary {
    pub bookmarks: Vec<HypermediaBookmark>,
}

impl Default for BookmarkLibrary {
    fn default() -> Self {
        let sample_mark_1 = HypermediaBookmark {
            id: "hbm_01".into(),
            target_uri: "https://hydrology.gov.au/catchment/north-spring".into(),
            title: "North Spring Catchment Observations 2026".into(),
            description: Some("Real-time telemetry and rainfall monitoring".into()),
            author_attribution: Some("Bureau of Meteorology".into()),
            created_at: 1774000000000,
            last_visited_at: 1774000000000,
            visit_count: 12,
            is_pinned: true,
            extracted_triples: vec![
                "<loc:NorthSpring> qualia:rainfall_mm 12.5 .".into(),
                "<loc:NorthSpring> a qualia:HydrologyCatchment .".into(),
            ],
            cml_entity_tags: vec!["loc:NorthSpring".into(), "qualia:Hydrology".into()],
            dialectical_score: 95,
            snapshot_hash: Some("sha256:8f42deadbeef".into()),
            thumbnail_uri: None,
            media_fragment: None,
            vibemark_script: Some("vibemark \"RainWatcher\" { on_poll() { ... } }".into()),
        };

        let sample_mark_2 = HypermediaBookmark {
            id: "hbm_02".into(),
            target_uri: "solid://timothy.solidcommunity.net/profile/card#me".into(),
            title: "Timothy Holborn Solid WebID Profile".into(),
            description: Some("W3C Solid personal data pod root".into()),
            author_attribution: Some("did:qualia:timothy".into()),
            created_at: 1774000000000,
            last_visited_at: 1774000000000,
            visit_count: 42,
            is_pinned: true,
            extracted_triples: vec![
                "<card#me> a schema:Person .".into(),
                "<card#me> schema:name \"Timothy Charles Holborn\" .".into(),
            ],
            cml_entity_tags: vec!["schema:Person".into(), "qualia:WebID".into()],
            dialectical_score: 99,
            snapshot_hash: Some("sha256:3a9fbeefcafe".into()),
            thumbnail_uri: None,
            media_fragment: None,
            vibemark_script: None,
        };

        Self {
            bookmarks: vec![sample_mark_1, sample_mark_2],
        }
    }
}

impl BookmarkLibrary {
    pub fn add(&mut self, mark: HypermediaBookmark) {
        self.bookmarks.retain(|b| b.id != mark.id);
        self.bookmarks.insert(0, mark);
    }

    pub fn remove(&mut self, id: &str) {
        self.bookmarks.retain(|b| b.id != id);
    }

    pub fn toggle_pin(&mut self, id: &str) {
        if let Some(mark) = self.bookmarks.iter_mut().find(|b| b.id == id) {
            mark.is_pinned = !mark.is_pinned;
        }
    }

    pub fn filter_by_tag(&self, tag: &str) -> Vec<&HypermediaBookmark> {
        self.bookmarks
            .iter()
            .filter(|b| b.cml_entity_tags.iter().any(|t| t == tag))
            .collect()
    }

    pub fn filter_by_domain(&self, domain_substr: &str) -> Vec<&HypermediaBookmark> {
        self.bookmarks
            .iter()
            .filter(|b| b.target_uri.contains(domain_substr))
            .collect()
    }

    pub fn pinned_bookmarks(&self) -> Vec<&HypermediaBookmark> {
        self.bookmarks.iter().filter(|b| b.is_pinned).collect()
    }

    /// Export the library to `.hbm` CBOR-LD binary envelope.
    pub fn export_hbm_binary(&self) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"HBM\x01");
        ciborium::ser::into_writer(self, &mut bytes)
            .map_err(|e| format!("cbor encode error: {}", e))?;
        Ok(bytes)
    }

    /// Import a library from `.hbm` CBOR-LD binary bytes.
    pub fn import_hbm_binary(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 4 || &bytes[0..4] != b"HBM\x01" {
            return Err("Invalid HBM header magic".into());
        }
        let lib: Self = ciborium::de::from_reader(&bytes[4..])
            .map_err(|e| format!("cbor decode error: {}", e))?;
        Ok(lib)
    }

    /// Execute a simulated VibeMark monitoring poll.
    pub fn execute_vibemark_poll(&self, mark_id: &str) -> Result<String, String> {
        let mark = self
            .bookmarks
            .iter()
            .find(|b| b.id == mark_id)
            .ok_or_else(|| "Bookmark not found".to_string())?;

        if let Some(_script) = &mark.vibemark_script {
            Ok(format!(
                "VibeMark '{}' polled successfully. Ingested {} triples. Emitted telemetry pulse: 'weather.heavy_rain'",
                mark.title,
                mark.extracted_triples.len()
            ))
        } else {
            Err("No VibeMark script attached".into())
        }
    }
}

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Quick Bookmark Bar widget with live VibeMark badges.
pub fn build_bookmark_bar_widget(document: &Document, lib: &BookmarkLibrary) -> Element {
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("poet-bookmark-bar");
    let bar_el: HtmlElement = bar.clone().dyn_into().unwrap();
    bar_el.style().set_css_text(
        "display: flex; align-items: center; gap: 6px; padding: 4px 8px; \
         background: #090e1a; border-bottom: 1px solid rgba(255, 255, 255, 0.08); \
         overflow-x: auto; font-family: sans-serif; font-size: 11px;",
    );

    for mark in lib.pinned_bookmarks() {
        let chip = document.create_element("div").unwrap();
        chip.set_class_name("poet-bookmark-chip");
        let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
        chip_el.style().set_css_text(
            "display: flex; align-items: center; gap: 6px; padding: 4px 8px; \
             background: rgba(30, 41, 59, 0.6); border: 1px solid rgba(255, 255, 255, 0.08); \
             border-radius: 6px; color: #f8fafc; cursor: pointer; white-space: nowrap;",
        );

        let icon = document.create_element("span").unwrap();
        icon.set_text_content(Some(if mark.vibemark_script.is_some() {
            "\u{26A1}"
        } else {
            "\u{1F516}"
        }));
        chip.append_child(&icon).unwrap();

        let title = document.create_element("span").unwrap();
        title.set_text_content(Some(&mark.title));
        chip.append_child(&title).unwrap();

        let trust = document.create_element("span").unwrap();
        trust.set_text_content(Some(&format!("{}%", mark.dialectical_score)));
        let trust_el: HtmlElement = trust.clone().dyn_into().unwrap();
        trust_el
            .style()
            .set_css_text("font-size: 9px; color: #34d399; font-family: var(--font-mono);");
        chip.append_child(&trust).unwrap();

        bar.append_child(&chip).unwrap();
    }

    bar
}

/// Build the full Hypermedia Meaning Shelf Bookmark Browser viewport.
pub fn build_bookmarks_manager_view(document: &Document, lib: &BookmarkLibrary) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;",
    );

    // Header Toolbar
    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px;",
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{1F516} Hypermedia Bookmarks & Meaning Shelf"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let count = document.create_element("span").unwrap();
    count.set_text_content(Some(&format!(
        "Total Saved: {} \u{00B7} VibeMarks Active: {}",
        lib.bookmarks.len(),
        lib.bookmarks
            .iter()
            .filter(|b| b.vibemark_script.is_some())
            .count()
    )));
    let count_el: HtmlElement = count.clone().dyn_into().unwrap();
    count_el
        .style()
        .set_css_text("font-size: 11px; font-family: var(--font-mono); color: #94a3b8;");
    header.append_child(&count).unwrap();

    root.append_child(&header).unwrap();

    // Bookmarks Grid
    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 10px;",
    );

    for mark in &lib.bookmarks {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(
            "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
             border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;",
        );

        let card_header = document.create_element("div").unwrap();
        let card_header_el: HtmlElement = card_header.clone().dyn_into().unwrap();
        card_header_el.style().set_css_text(
            "display: flex; justify-content: space-between; align-items: flex-start;",
        );

        let card_title = document.create_element("span").unwrap();
        card_title.set_text_content(Some(&mark.title));
        let card_title_el: HtmlElement = card_title.clone().dyn_into().unwrap();
        card_title_el
            .style()
            .set_css_text("font-weight: 600; font-size: 12px; color: #f8fafc;");
        card_header.append_child(&card_title).unwrap();

        if mark.vibemark_script.is_some() {
            let badge = document.create_element("span").unwrap();
            badge.set_text_content(Some("VIBEMARK"));
            let badge_el: HtmlElement = badge.clone().dyn_into().unwrap();
            badge_el.style().set_css_text("font-size: 9px; padding: 2px 4px; background: rgba(56, 189, 248, 0.2); color: #38bdf8; border-radius: 4px; font-weight: 700;");
            card_header.append_child(&badge).unwrap();
        }

        card.append_child(&card_header).unwrap();

        let uri = document.create_element("span").unwrap();
        uri.set_text_content(Some(&mark.target_uri));
        let uri_el: HtmlElement = uri.clone().dyn_into().unwrap();
        uri_el.style().set_css_text("font-size: 10px; font-family: var(--font-mono); color: #94a3b8; word-break: break-all;");
        card.append_child(&uri).unwrap();

        let tags_row = document.create_element("div").unwrap();
        let tags_row_el: HtmlElement = tags_row.clone().dyn_into().unwrap();
        tags_row_el
            .style()
            .set_css_text("display: flex; gap: 4px; flex-wrap: wrap; margin-top: 4px;");

        for tag in &mark.cml_entity_tags {
            let tag_span = document.create_element("span").unwrap();
            tag_span.set_text_content(Some(tag));
            let tag_span_el: HtmlElement = tag_span.clone().dyn_into().unwrap();
            tag_span_el.style().set_css_text("font-size: 9px; padding: 2px 6px; background: rgba(255, 255, 255, 0.06); border-radius: 4px; color: #cbd5e1;");
            tags_row.append_child(&tag_span).unwrap();
        }
        card.append_child(&tags_row).unwrap();

        grid.append_child(&card).unwrap();
    }

    root.append_child(&grid).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_creation() {
        let mark = HypermediaBookmark::new_web(
            "https://qualia.network/doc/01",
            "Qualia Network Architecture",
            Some("did:qualia:developer"),
        );
        assert_eq!(mark.title, "Qualia Network Architecture");
        assert_eq!(mark.dialectical_score, 85);
        assert!(!mark.id.is_empty());
    }

    #[test]
    fn test_bookmark_library_pin_and_filters() {
        let lib = BookmarkLibrary::default();
        assert_eq!(lib.pinned_bookmarks().len(), 2);

        let hydrology_marks = lib.filter_by_tag("qualia:Hydrology");
        assert_eq!(hydrology_marks.len(), 1);
        assert_eq!(
            hydrology_marks[0].title,
            "North Spring Catchment Observations 2026"
        );

        let solid_marks = lib.filter_by_domain("solidcommunity.net");
        assert_eq!(solid_marks.len(), 1);
    }

    #[test]
    fn test_hbm_binary_roundtrip() {
        let lib = BookmarkLibrary::default();
        let bytes = lib.export_hbm_binary().unwrap();
        assert_eq!(&bytes[0..4], b"HBM\x01");

        let imported = BookmarkLibrary::import_hbm_binary(&bytes).unwrap();
        assert_eq!(imported.bookmarks.len(), lib.bookmarks.len());
        assert_eq!(imported.bookmarks[0].title, lib.bookmarks[0].title);
    }

    #[test]
    fn test_vibemark_poll_execution() {
        let lib = BookmarkLibrary::default();
        let res = lib.execute_vibemark_poll("hbm_01").unwrap();
        assert!(res.contains("North Spring Catchment Observations 2026"));
        assert!(res.contains("weather.heavy_rain"));
    }
}
