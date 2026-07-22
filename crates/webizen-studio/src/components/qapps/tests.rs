//! Consistency tests for the QApp catalog.

// â”€â”€ Consistency tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    //! Source-level consistency checks. These parse the catalogue and dispatcher
    //! source via `include_str!` (no component instantiation), so they stay cheap
    //! and catch id drift like the historical `physics-sim` / `physics-simulator`
    //! mismatch automatically.

    const QAPPS_SRC: &str = include_str!("catalog/mod.rs");
    const DISPATCHER_SRC: &str = include_str!("../../studio_canvas.rs");

    /// First double-quoted substring in `s`, if any.
    fn first_quoted(s: &str) -> Option<&str> {
        let start = s.find('"')? + 1;
        let rest = &s[start..];
        let end = rest.find('"')?;
        Some(&rest[..end])
    }

    /// All `id: "..."` values whose `QApp { .. }` body is `cat: Cat::Academic`.
    fn academic_ids(src: &str) -> Vec<&str> {
        let mut ids = Vec::new();
        let mut rest = src;
        while let Some(open) = rest.find("QApp {") {
            let after = &rest[open + "QApp {".len()..];
            let close = after.find('}').unwrap_or(after.len());
            let body = &after[..close];
            if body.contains("cat: Cat::Academic") {
                if let Some(idx) = body.find("id:") {
                    if let Some(id) = first_quoted(&body[idx..]) {
                        ids.push(id);
                    }
                }
            }
            rest = &after[close..];
        }
        ids
    }

    /// All match-arm tags (`"tag" => ...`) handled by the dispatcher.
    ///
    /// Matches the leading `"tag" =>` form regardless of what follows, so it is
    /// robust to rustfmt wrapping a long arm onto multiple lines (`"tag" => {`
    /// with the `rsx!` body on the next line).
    fn dispatcher_tags(src: &str) -> std::collections::HashSet<&str> {
        src.lines()
            .filter_map(|line| {
                let t = line.trim_start();
                if !t.starts_with('"') {
                    return None;
                }
                let tag = first_quoted(t)?;
                let rest = t.get(tag.len() + 2..)?.trim_start();
                rest.starts_with("=>").then_some(tag)
            })
            .collect()
    }

    #[test]
    fn every_academic_qapp_has_a_dispatcher_arm() {
        let ids = academic_ids(QAPPS_SRC);
        let tags = dispatcher_tags(DISPATCHER_SRC);
        assert!(!ids.is_empty(), "no Academic QApps parsed â€” parser drift?");

        let missing: Vec<&str> = ids
            .iter()
            .copied()
            .filter(|id| !tags.contains(id))
            .collect();
        assert!(
            missing.is_empty(),
            "Academic QApps with no dispatcher arm: {missing:?}"
        );
    }

    #[test]
    fn academic_catalogue_ids_are_unique() {
        let ids = academic_ids(QAPPS_SRC);
        let mut seen = std::collections::HashSet::new();
        let dupes: Vec<&str> = ids.iter().copied().filter(|id| !seen.insert(*id)).collect();
        assert!(
            dupes.is_empty(),
            "duplicate Academic catalogue ids: {dupes:?}"
        );
    }
}
