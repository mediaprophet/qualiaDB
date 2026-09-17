//! Webizen library dialect for semantic instruments (SI-10).
//!
//! Held-gate copy, Demo/Reference chips, collect ≠ activate. No Host widen.
//! Shared by Desktop Dioxus and WASM Poet. No Dioxus tree here.

/// Soft why-text for missing / unknown pack. Never "broken".
pub const HELD_WHY: &str = "held / not yet — open a pack or seed demos";

pub const INSPECT_BEFORE_FETCH: &str = "inspect before network fetch";

pub const COLLECT_NOT_ACTIVATE: &str = "collected is not active";

pub const DEMO_CHIP: &str = "Demo";
pub const REFERENCE_CHIP: &str = "Reference";

/// WASM / lite hosts cannot mmap native Q42 v3 volumes. Inspect N3/HCF; hold the volume.
pub const WASM_Q42_HELD: &str =
    "held / not yet — native Q42 v3 volumes are not loadable on this WASM profile; N3/HCF remain inspectable";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryAction {
    Inspect,
    Collect,
    Activate,
    Run,
    Cancel,
    ViewReceipt,
}

pub fn action_label(action: LibraryAction) -> &'static str {
    match action {
        LibraryAction::Inspect => "inspect",
        LibraryAction::Collect => "collect",
        LibraryAction::Activate => "activate",
        LibraryAction::Run => "run",
        LibraryAction::Cancel => "cancel",
        LibraryAction::ViewReceipt => "view receipt",
    }
}

/// Activate/run need a closed lock. Inspect, collect, cancel, receipts do not.
pub fn action_requires_closed(action: LibraryAction) -> bool {
    matches!(action, LibraryAction::Activate | LibraryAction::Run)
}

/// Dispatch is the instrument entry point. `Host.*` is refused, not invented.
pub fn host_id_refused(entry: &str) -> bool {
    entry.contains("Host.")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryCard {
    pub release_id: String,
    pub chip: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryOutcome {
    Held { why: String },
    Ready(LibraryCard),
}

pub fn catalog_chips() -> &'static [&'static str] {
    &[DEMO_CHIP, REFERENCE_CHIP]
}

pub fn sanitize_held_why(raw: &str) -> String {
    let folded = raw.to_ascii_lowercase();
    if folded.contains("broken")
        || raw.trim().is_empty()
        || folded.contains("unavailable")
    {
        return HELD_WHY.to_string();
    }
    if folded.contains("held") || folded.contains("open a pack") || folded.contains("seed demo")
    {
        raw.trim().to_string()
    } else {
        HELD_WHY.to_string()
    }
}

/// Map invoke result. Missing pack → held. Never "broken".
pub fn interpret_invoke(ok: bool, blob: &str, err: Option<&str>) -> LibraryOutcome {
    if ok {
        if let Some(card) = parse_library_card(blob) {
            return LibraryOutcome::Ready(card);
        }
    }
    let diagnostic = err.unwrap_or(blob);
    let folded = diagnostic.to_ascii_lowercase();
    let why = if folded.contains("e300")
        || folded.contains("not found")
        || folded.contains("missing")
        || folded.contains("held / not yet")
        || folded.contains("open a pack")
    {
        sanitize_held_why(diagnostic)
    } else {
        HELD_WHY.to_string()
    };
    LibraryOutcome::Held { why }
}

fn parse_library_card(src: &str) -> Option<LibraryCard> {
    let release_id = extract_quoted_field(src, "release_id")
        .or_else(|| extract_quoted_field(src, "releaseId"))
        .or_else(|| extract_quoted_field(src, "pack_id"))?;
    if release_id.is_empty() {
        return None;
    }
    let chip = if src.to_ascii_lowercase().contains("reference") {
        REFERENCE_CHIP
    } else {
        DEMO_CHIP
    };
    Some(LibraryCard { release_id, chip })
}

fn extract_quoted_field(src: &str, key: &str) -> Option<String> {
    let patterns = [
        format!("{key}: \""),
        format!("{key}:\""),
        format!("\"{key}\": \""),
        format!("\"{key}\":\""),
    ];
    for pat in patterns {
        if let Some(start) = src.find(&pat) {
            let rest = &src[start + pat.len()..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

pub fn copy_avoids_broken(text: &str) -> bool {
    !text.to_ascii_lowercase().contains("broken")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_copy_never_says_broken() {
        assert_eq!(HELD_WHY, "held / not yet — open a pack or seed demos");
        assert_eq!(sanitize_held_why("volume broken"), HELD_WHY);
        assert_eq!(sanitize_held_why(""), HELD_WHY);
        assert_eq!(sanitize_held_why("Unavailable: start daemon"), HELD_WHY);
        assert!(copy_avoids_broken(HELD_WHY));
        assert!(copy_avoids_broken(INSPECT_BEFORE_FETCH));
        assert!(copy_avoids_broken(COLLECT_NOT_ACTIVATE));
        match interpret_invoke(false, "", Some("E300@0..0: pack not found")) {
            LibraryOutcome::Held { why } => {
                assert_eq!(why, HELD_WHY);
                assert!(copy_avoids_broken(&why));
            }
            other => panic!("{other:?}"),
        }
        match interpret_invoke(false, "broken pack", Some("missing pack")) {
            LibraryOutcome::Held { why } => assert_eq!(why, HELD_WHY),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn collect_is_not_activate() {
        assert_eq!(COLLECT_NOT_ACTIVATE, "collected is not active");
        assert_ne!(
            action_label(LibraryAction::Collect),
            action_label(LibraryAction::Activate)
        );
        assert!(!action_requires_closed(LibraryAction::Collect));
        assert!(!action_requires_closed(LibraryAction::Inspect));
        assert!(!action_requires_closed(LibraryAction::Cancel));
        assert!(!action_requires_closed(LibraryAction::ViewReceipt));
        assert!(action_requires_closed(LibraryAction::Activate));
        assert!(action_requires_closed(LibraryAction::Run));
        assert_eq!(action_label(LibraryAction::Collect), "collect");
        assert_eq!(action_label(LibraryAction::Activate), "activate");
        assert_eq!(action_label(LibraryAction::ViewReceipt), "view receipt");
    }

    #[test]
    fn host_dot_ids_are_refused() {
        assert!(host_id_refused("Host.assess"));
        assert!(host_id_refused("Capability.Host.run"));
        assert!(!host_id_refused("assess"));
        assert!(!host_id_refused("recognise"));
        assert!(!host_id_refused("GraphDatabase.lexicon_manifest"));
    }

    #[test]
    fn wasm_q42_is_held_degradation() {
        assert!(WASM_Q42_HELD.starts_with("held / not yet"));
        assert!(WASM_Q42_HELD.contains("Q42"));
        assert!(WASM_Q42_HELD.contains("WASM"));
        assert!(WASM_Q42_HELD.contains("N3/HCF"));
        assert!(copy_avoids_broken(WASM_Q42_HELD));
        assert!(!WASM_Q42_HELD.to_ascii_lowercase().contains("broken"));
    }

    #[test]
    fn inspect_before_fetch() {
        assert_eq!(INSPECT_BEFORE_FETCH, "inspect before network fetch");
        assert_eq!(action_label(LibraryAction::Inspect), "inspect");
        assert!(!action_requires_closed(LibraryAction::Inspect));
        assert_eq!(catalog_chips(), &[DEMO_CHIP, REFERENCE_CHIP]);
        assert_eq!(DEMO_CHIP, "Demo");
        assert_eq!(REFERENCE_CHIP, "Reference");
        let src = r#"{release_id: "si:demo:unit-convert", category: "Demo"}"#;
        match interpret_invoke(true, src, None) {
            LibraryOutcome::Ready(card) => {
                assert_eq!(card.release_id, "si:demo:unit-convert");
                assert_eq!(card.chip, DEMO_CHIP);
            }
            other => panic!("{other:?}"),
        }
    }
}
