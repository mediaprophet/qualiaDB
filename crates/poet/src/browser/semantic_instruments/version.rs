//! Version comparison panel (SI-09).
//!
//! Two labelled demo releases in the same slug family. Not OWL, not Host.*.
//! Entry points are `assess` / `recognise` only.

use web_sys::{Document, Element};

const REGION_LABEL: &str = "instrument version comparison";

/// One labelled demo release. `entry` is assess or recognise only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseView {
    pub id: &'static str,
    pub name: &'static str,
    pub entry: &'static str,
    pub digest: &'static str,
}

/// Unit conversion demo, patch 1.0.0.
pub const UNIT_CONVERT_V1: ReleaseView = ReleaseView {
    id: "unit-convert@1.0.0",
    name: "Unit conversion (demo)",
    entry: "assess",
    digest: "demo-digest-unit-convert-1.0.0",
};

/// Same slug family, different digest / patch version.
pub const UNIT_CONVERT_V2: ReleaseView = ReleaseView {
    id: "unit-convert@1.0.1",
    name: "Unit conversion (demo)",
    entry: "assess",
    digest: "demo-digest-unit-convert-1.0.1",
};

fn refuse_host_id(id: &str) -> bool {
    id.contains("Host.")
}

/// Field triples `(field, left, right)` that differ. Host.* ids refuse (empty).
pub fn diff_fields(
    a: &ReleaseView,
    b: &ReleaseView,
) -> Vec<(&'static str, &'static str, &'static str)> {
    if refuse_host_id(a.id) || refuse_host_id(b.id) {
        return Vec::new();
    }
    let mut out = Vec::new();
    if a.id != b.id {
        out.push(("id", a.id, b.id));
    }
    if a.name != b.name {
        out.push(("name", a.name, b.name));
    }
    if a.entry != b.entry {
        out.push(("entry", a.entry, b.entry));
    }
    if a.digest != b.digest {
        out.push(("digest", a.digest, b.digest));
    }
    out
}

/// Version comparison region. Seeded demo pair; Host.* ids refuse empty.
pub fn build_version_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-version");
    root.set_attribute("data-version-panel", "1").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", REGION_LABEL).ok();

    if refuse_host_id(UNIT_CONVERT_V1.id) || refuse_host_id(UNIT_CONVERT_V2.id) {
        return root;
    }

    let title = document.create_element("div").unwrap();
    title.set_class_name("instrument-version-title");
    title.set_text_content(Some("Version comparison"));
    root.append_child(&title).unwrap();

    let ids = document.create_element("div").unwrap();
    ids.set_class_name("instrument-version-ids");
    ids.set_attribute("role", "list").ok();
    ids.set_attribute("aria-label", "compared release ids").ok();
    for rel in [UNIT_CONVERT_V1, UNIT_CONVERT_V2] {
        let item = document.create_element("div").unwrap();
        item.set_attribute("role", "listitem").ok();
        item.set_attribute("data-release-id", rel.id).ok();
        item.set_text_content(Some(rel.id));
        ids.append_child(&item).unwrap();
    }
    root.append_child(&ids).unwrap();

    let diffs = document.create_element("div").unwrap();
    diffs.set_class_name("instrument-version-diffs");
    diffs.set_attribute("role", "list").ok();
    diffs.set_attribute("aria-label", "release field diffs").ok();
    for (field, left, right) in diff_fields(&UNIT_CONVERT_V1, &UNIT_CONVERT_V2) {
        let item = document.create_element("div").unwrap();
        item.set_attribute("role", "listitem").ok();
        item.set_attribute("data-diff-field", field).ok();
        item.set_text_content(Some(&format!("{field}: {left} → {right}")));
        diffs.append_child(&item).unwrap();
    }
    root.append_child(&diffs).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_vs_v2_differs_on_digest_and_version_id() {
        let diffs = diff_fields(&UNIT_CONVERT_V1, &UNIT_CONVERT_V2);
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0], ("id", UNIT_CONVERT_V1.id, UNIT_CONVERT_V2.id));
        assert_eq!(
            diffs[1],
            ("digest", UNIT_CONVERT_V1.digest, UNIT_CONVERT_V2.digest)
        );
        assert_ne!(UNIT_CONVERT_V1.id, UNIT_CONVERT_V2.id);
        assert_ne!(UNIT_CONVERT_V1.digest, UNIT_CONVERT_V2.digest);
        assert_eq!(UNIT_CONVERT_V1.name, UNIT_CONVERT_V2.name);
        assert_eq!(UNIT_CONVERT_V1.entry, UNIT_CONVERT_V2.entry);
        assert!(UNIT_CONVERT_V1.id.starts_with("unit-convert@"));
        assert!(UNIT_CONVERT_V2.id.starts_with("unit-convert@"));
    }

    #[test]
    fn same_release_diffs_empty() {
        assert!(diff_fields(&UNIT_CONVERT_V1, &UNIT_CONVERT_V1).is_empty());
        assert!(diff_fields(&UNIT_CONVERT_V2, &UNIT_CONVERT_V2).is_empty());
    }

    #[test]
    fn host_id_refused() {
        let host = ReleaseView {
            id: "Host.ClinicalRisk.framingham",
            name: "refused",
            entry: "assess",
            digest: "none",
        };
        assert!(diff_fields(&UNIT_CONVERT_V1, &host).is_empty());
        assert!(diff_fields(&host, &UNIT_CONVERT_V2).is_empty());
        assert!(diff_fields(&host, &host).is_empty());
        assert!(refuse_host_id(host.id));
        assert!(!refuse_host_id(UNIT_CONVERT_V1.id));
    }

    #[test]
    fn entries_have_no_host() {
        for rel in [UNIT_CONVERT_V1, UNIT_CONVERT_V2] {
            assert!(rel.entry == "assess" || rel.entry == "recognise");
            assert!(!rel.id.contains("Host."));
            assert!(!rel.name.contains("Host."));
            assert!(!rel.entry.contains("Host."));
            assert!(!rel.digest.contains("Host."));
            assert!(!rel.name.contains("OWL"));
            assert!(!rel.name.contains("owl:"));
        }
        assert!(!REGION_LABEL.contains("Host."));
    }
}
