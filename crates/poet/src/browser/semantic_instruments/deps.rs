//! Dependency election panel (SI-09).
//!
//! Demo pack dependencies. Unresolved required deps stay held / not yet.
//! Host.* names are refused and skipped. No clinical kernel.

use web_sys::{Document, Element};

const REGION_LABEL: &str = "instrument dependency election";
const HELD_UNRESOLVED: &str = "held / not yet — unresolved required dependency";
const CLOSED: &str = "dependencies closed";

/// One elected dependency for a demo pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepReq {
    pub name: &'static str,
    pub required: bool,
    pub resolved: bool,
}

/// Optional visual `.10d` artwork — resolved, not required.
pub const OPTIONAL_VISUAL_10D: DepReq = DepReq {
    name: "visual .10d",
    required: false,
    resolved: true,
};

/// Required definition N3 — resolved.
pub const REQUIRED_DEFINITION_N3: DepReq = DepReq {
    name: "definition N3",
    required: true,
    resolved: true,
};

/// Required executable — unresolved in the open fixture.
pub const REQUIRED_EXECUTABLE_UNRESOLVED: DepReq = DepReq {
    name: "executable form",
    required: true,
    resolved: false,
};

/// Required executable — resolved in the closed fixture.
pub const REQUIRED_EXECUTABLE_RESOLVED: DepReq = DepReq {
    name: "executable form",
    required: true,
    resolved: true,
};

/// Open demo pack: required executable still unresolved.
pub const DEMO_PACK_OPEN: &[DepReq] = &[
    OPTIONAL_VISUAL_10D,
    REQUIRED_DEFINITION_N3,
    REQUIRED_EXECUTABLE_UNRESOLVED,
];

/// Closed demo pack: every required dependency resolved.
pub const DEMO_PACK_CLOSED: &[DepReq] = &[
    OPTIONAL_VISUAL_10D,
    REQUIRED_DEFINITION_N3,
    REQUIRED_EXECUTABLE_RESOLVED,
];

/// True when a dependency name contains `Host.` and must be skipped.
pub fn refuse_host_dep(name: &str) -> bool {
    name.contains("Host.")
}

/// Held while any non-Host required dependency is unresolved.
pub fn election_status(deps: &[DepReq]) -> &'static str {
    if deps
        .iter()
        .any(|d| !refuse_host_dep(d.name) && d.required && !d.resolved)
    {
        HELD_UNRESOLVED
    } else {
        CLOSED
    }
}

fn bool_attr(ok: bool) -> &'static str {
    if ok {
        "true"
    } else {
        "false"
    }
}

/// Dependency election region. Host.* names are skipped.
pub fn build_deps_view(document: &Document) -> Element {
    let deps = DEMO_PACK_OPEN;
    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-deps");
    root.set_attribute("data-deps-panel", "1").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", REGION_LABEL).ok();

    let title = document.create_element("div").unwrap();
    title.set_class_name("instrument-deps-title");
    title.set_text_content(Some("Dependency election"));
    root.append_child(&title).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_class_name("instrument-deps-list");
    list.set_attribute("role", "list").ok();
    list.set_attribute("aria-label", REGION_LABEL).ok();
    for dep in deps.iter().filter(|d| !refuse_host_dep(d.name)) {
        let item = document.create_element("div").unwrap();
        item.set_class_name("instrument-deps-row");
        item.set_attribute("role", "listitem").ok();
        item.set_attribute("data-dep-name", dep.name).ok();
        item.set_attribute("data-required", bool_attr(dep.required))
            .ok();
        item.set_attribute("data-resolved", bool_attr(dep.resolved))
            .ok();
        item.set_text_content(Some(dep.name));
        list.append_child(&item).unwrap();
    }
    root.append_child(&list).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_class_name("instrument-deps-status");
    status.set_attribute("data-deps-status", "1").ok();
    status.set_attribute("role", "status").ok();
    status.set_text_content(Some(election_status(deps)));
    root.append_child(&status).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresolved_required_is_held() {
        assert_eq!(election_status(DEMO_PACK_OPEN), HELD_UNRESOLVED);
        assert!(!REQUIRED_EXECUTABLE_UNRESOLVED.resolved);
        assert!(REQUIRED_EXECUTABLE_UNRESOLVED.required);
        assert!(!OPTIONAL_VISUAL_10D.required);
        assert!(OPTIONAL_VISUAL_10D.resolved);
        assert!(REQUIRED_DEFINITION_N3.required);
        assert!(REQUIRED_DEFINITION_N3.resolved);
        assert!(HELD_UNRESOLVED.starts_with("held / not yet"));
    }

    #[test]
    fn all_required_resolved_is_closed() {
        assert_eq!(election_status(DEMO_PACK_CLOSED), CLOSED);
        assert!(DEMO_PACK_CLOSED
            .iter()
            .filter(|d| d.required)
            .all(|d| d.resolved));
        assert_eq!(election_status(&[]), CLOSED);
    }

    #[test]
    fn host_dep_skipped_and_refused() {
        let host = DepReq {
            name: "Host.ClinicalRisk.framingham",
            required: true,
            resolved: false,
        };
        assert!(refuse_host_dep(host.name));
        assert!(!refuse_host_dep(REQUIRED_DEFINITION_N3.name));
        assert!(!refuse_host_dep(OPTIONAL_VISUAL_10D.name));
        let mixed = [
            OPTIONAL_VISUAL_10D,
            REQUIRED_DEFINITION_N3,
            REQUIRED_EXECUTABLE_RESOLVED,
            host,
        ];
        assert_eq!(election_status(&mixed), CLOSED);
        let visible: Vec<&str> = mixed
            .iter()
            .filter(|d| !refuse_host_dep(d.name))
            .map(|d| d.name)
            .collect();
        assert!(!visible.iter().any(|n| n.contains("Host.")));
        assert_eq!(visible.len(), 3);
        for dep in DEMO_PACK_OPEN.iter().chain(DEMO_PACK_CLOSED.iter()) {
            assert!(!dep.name.contains("Host."));
        }
        assert!(!REGION_LABEL.contains("Host."));
        assert!(!HELD_UNRESOLVED.contains("Host."));
        assert!(!CLOSED.contains("Host."));
    }
}
