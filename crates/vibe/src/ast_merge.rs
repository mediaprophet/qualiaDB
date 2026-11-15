//! 3-Way AST Structural Merge for VibeScript.
//!
//! Reconciles concurrent edits from the visual Studio canvas (`ours`) and the textual
//! code editor (`theirs`) against a shared ancestor AST (`base`). Non-conflicting additions,
//! modifications, and property edits are merged deterministically.

use crate::ast::*;

/// Result of a 3-way AST merge operation.
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Merged program AST.
    pub program: Program,
    /// Any structural conflicts detected during merge.
    pub conflicts: Vec<MergeConflict>,
}

/// A conflict where two branches modified the same AST node incompatibly.
#[derive(Debug, Clone, PartialEq)]
pub struct MergeConflict {
    pub path: String,
    pub description: String,
    pub our_change: String,
    pub their_change: String,
}

/// Perform a 3-way structural merge on VibeScript programs.
pub fn merge_programs(base: &Program, ours: &Program, theirs: &Program) -> MergeResult {
    let mut conflicts = Vec::new();
    let mut merged_items = Vec::new();

    // Index base, ours, and theirs items by key (Item kind + identifier)
    let base_map = index_items(&base.items);
    let mut our_map = index_items(&ours.items);
    let mut their_map = index_items(&theirs.items);

    // Deterministic key order: preserve appearance order across base -> ours -> theirs.
    let mut all_keys: Vec<String> = Vec::new();
    for item in base
        .items
        .iter()
        .chain(ours.items.iter())
        .chain(theirs.items.iter())
    {
        let key = item_key(item);
        if !all_keys.contains(&key) {
            all_keys.push(key);
        }
    }

    for key in &all_keys {
        let base_item = find_indexed_item(&base_map, key);
        let our_item = take_indexed_item(&mut our_map, key);
        let their_item = take_indexed_item(&mut their_map, key);

        match (base_item, our_item, their_item) {
            // Unchanged in both
            (Some(b), Some(o), Some(t)) if items_equal(b, o) && items_equal(b, t) => {
                merged_items.push((*b).clone());
            }
            // Modified only in ours
            (Some(b), Some(o), Some(t)) if items_equal(b, t) && !items_equal(b, o) => {
                merged_items.push(o.clone());
            }
            // Modified only in theirs
            (Some(b), Some(o), Some(t)) if items_equal(b, o) && !items_equal(b, t) => {
                merged_items.push(t.clone());
            }
            // Modified identically in both
            (Some(_), Some(o), Some(t)) if items_equal(o, t) => {
                merged_items.push(o.clone());
            }
            // Modified incompatibly in both -> Conflict (choose ours, record conflict)
            (Some(_), Some(o), Some(t)) => {
                conflicts.push(MergeConflict {
                    path: key.clone(),
                    description: "Conflicting modifications to item".to_string(),
                    our_change: format!("{o:?}"),
                    their_change: format!("{t:?}"),
                });
                merged_items.push(o.clone());
            }
            // Added only in ours
            (None, Some(o), None) => {
                merged_items.push(o.clone());
            }
            // Added only in theirs
            (None, None, Some(t)) => {
                merged_items.push(t.clone());
            }
            // Added identically in both
            (None, Some(o), Some(t)) if items_equal(o, t) => {
                merged_items.push(o.clone());
            }
            // Added differently in both -> Conflict
            (None, Some(o), Some(t)) => {
                conflicts.push(MergeConflict {
                    path: key.clone(),
                    description: "Conflicting additions with same identifier".to_string(),
                    our_change: format!("{o:?}"),
                    their_change: format!("{t:?}"),
                });
                merged_items.push(o.clone());
            }
            // Deleted in ours, unchanged in theirs -> Delete
            (Some(b), None, Some(t)) if items_equal(b, t) => {}
            // Deleted in theirs, unchanged in ours -> Delete
            (Some(b), Some(o), None) if items_equal(b, o) => {}
            // Deleted in both -> Delete
            (Some(_), None, None) => {}
            // Deleted in one, modified in the other -> Conflict
            (Some(_), Some(o), None) => {
                conflicts.push(MergeConflict {
                    path: key.clone(),
                    description: "Item deleted in theirs but modified in ours".to_string(),
                    our_change: format!("{o:?}"),
                    their_change: "deleted".to_string(),
                });
                merged_items.push(o.clone());
            }
            (Some(_), None, Some(t)) => {
                conflicts.push(MergeConflict {
                    path: key.clone(),
                    description: "Item deleted in ours but modified in theirs".to_string(),
                    our_change: "deleted".to_string(),
                    their_change: format!("{t:?}"),
                });
                merged_items.push(t.clone());
            }
            (None, None, None) => {}
        }
    }

    // Merge imports without duplication
    let mut merged_imports = ours.imports.clone();
    for imp in &theirs.imports {
        if !merged_imports.iter().any(|i| i.path == imp.path) {
            merged_imports.push(imp.clone());
        }
    }

    // Merge requires without duplication
    let mut merged_requires = ours.requires.clone();
    for req in &theirs.requires {
        if !merged_requires.iter().any(|r| r.id == req.id) {
            merged_requires.push(req.clone());
        }
    }

    let program = Program {
        span: ours.span,
        module: ours.module.clone().or_else(|| theirs.module.clone()),
        prefixes: ours.prefixes.clone(),
        locales: ours.locales.clone(),
        imports: merged_imports,
        requires: merged_requires,
        items: merged_items,
    };

    MergeResult { program, conflicts }
}

fn item_key(item: &Item) -> String {
    match item {
        Item::Const(cd) => format!("const:{}", cd.name),
        Item::Function(fd) => format!("fn:{}", fd.name),
        Item::Hook(hd) => format!("on:{}", hd.path.join(".")),
        Item::Enum(ed) => format!("enum:{}", ed.name),
        Item::Law(ld) => format!("law:{}", ld.name),
        Item::Material(md) => format!("material:{}", md.name),
        Item::Field(fld) => format!("field:{}", fld.name),
        Item::Cell(cd) => format!("cell:{}", cd.name),
        Item::Present(pd) => format!("present:{}", pd.name),
        Item::Bind(b) => format!(
            "bind:{}",
            strip_debug_spans(&format!("{:?}<->{:?}", b.left, b.right))
        ),
        Item::Statement(stmt) => format!("stmt:{}", strip_debug_spans(&format!("{stmt:?}"))),
    }
}

fn index_items(items: &[Item]) -> Vec<(String, &Item)> {
    let mut indexed = Vec::with_capacity(items.len());
    for item in items {
        let key = item_key(item);
        if let Some(slot) = indexed.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = item;
        } else {
            indexed.push((key, item));
        }
    }
    indexed
}

fn find_indexed_item<'a>(indexed: &[(String, &'a Item)], key: &str) -> Option<&'a Item> {
    indexed.iter().find(|(k, _)| k == key).map(|(_, item)| *item)
}

fn take_indexed_item<'a>(indexed: &mut Vec<(String, &'a Item)>, key: &str) -> Option<&'a Item> {
    indexed
        .iter()
        .position(|(k, _)| k == key)
        .map(|pos| indexed.swap_remove(pos).1)
}

fn strip_debug_spans(debug: &str) -> String {
    let bytes = debug.as_bytes();
    let mut out = String::with_capacity(debug.len());
    let mut i = 0;
    let mut in_str = false;
    let mut escape = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            out.push(b as char);
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = false;
                if i + 1 < bytes.len() {
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
            } else if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if b == b'"' {
            in_str = true;
            out.push('"');
            i += 1;
            continue;
        }
        const PREFIX: &[u8] = b"Span { start:";
        if bytes[i..].starts_with(PREFIX) {
            if let Some(rel_end) = debug[i..].find('}') {
                out.push_str("Span");
                i += rel_end + 1;
                continue;
            }
        }
        out.push(b as char);
        i += 1;
    }
    out
}

fn items_equal(a: &Item, b: &Item) -> bool {
    // Structural span-independent equality check.
    strip_debug_spans(&format!("{a:?}")) == strip_debug_spans(&format!("{b:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_program;

    #[test]
    fn merge_disjoint_additions() {
        let base = parse_program("const A: i64 = 1;\n").expect("base");
        let ours = parse_program("const A: i64 = 1;\nconst B: i64 = 2;\n").expect("ours");
        let theirs = parse_program("const A: i64 = 1;\nconst C: i64 = 3;\n").expect("theirs");

        let res = merge_programs(&base, &ours, &theirs);
        assert!(res.conflicts.is_empty());
        assert_eq!(res.program.items.len(), 3);
    }

    #[test]
    fn merge_bind_decl_items() {
        let base = parse_program("const A: i64 = 1;\n").expect("base");
        let mut ours = base.clone();
        let bind_item = Item::Bind(BindDecl {
            span: crate::span::Span::point(0),
            left: Expr {
                span: crate::span::Span::point(0),
                kind: ExprKind::Ident("left_slot".into()),
            },
            right: Expr {
                span: crate::span::Span::point(0),
                kind: ExprKind::Ident("right_slot".into()),
            },
            clamp: None,
            resolve: BindResolve::Latest,
        });
        ours.items.push(bind_item);
        let mut theirs = base.clone();
        theirs.items.push(
            parse_program("const C: i64 = 3;\n")
                .expect("theirs")
                .items
                .remove(0),
        );

        let res = merge_programs(&base, &ours, &theirs);
        assert!(res.conflicts.is_empty());
        assert_eq!(res.program.items.len(), 3);
        assert!(res.program.items.iter().any(|item| matches!(item, Item::Bind(_))));
    }
}
