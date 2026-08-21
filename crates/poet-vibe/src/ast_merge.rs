//! 3-Way AST Structural Merge for VibeScript.
//!
//! Reconciles concurrent edits from the visual Studio canvas (`ours`) and the textual
//! code editor (`theirs`) against a shared ancestor AST (`base`). Non-conflicting additions,
//! modifications, and property edits are merged deterministically.

use crate::ast::*;
use std::collections::HashMap;

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

    // Track all seen item keys
    let mut all_keys: Vec<String> = Vec::new();
    for k in base_map
        .keys()
        .chain(our_map.keys())
        .chain(their_map.keys())
    {
        if !all_keys.contains(k) {
            all_keys.push(k.clone());
        }
    }

    for key in &all_keys {
        let base_item = base_map.get(key);
        let our_item = our_map.remove(key);
        let their_item = their_map.remove(key);

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
        imports: merged_imports,
        requires: merged_requires,
        items: merged_items,
    };

    MergeResult { program, conflicts }
}

fn index_items(items: &[Item]) -> HashMap<String, &Item> {
    let mut map = HashMap::new();
    for item in items {
        let key = match item {
            Item::Const(cd) => format!("const:{}", cd.name),
            Item::Function(fd) => format!("fn:{}", fd.name),
            Item::Hook(hd) => format!("on:{}", hd.path.join(".")),
            Item::Enum(ed) => format!("enum:{}", ed.name),
            Item::Law(ld) => format!("law:{}", ld.name),
            Item::Material(md) => format!("material:{}", md.name),
            Item::Field(fld) => format!("field:{}", fld.name),
            Item::Statement(stmt) => format!("stmt:{stmt:?}"),
        };
        map.insert(key, item);
    }
    map
}

fn items_equal(a: &Item, b: &Item) -> bool {
    // Structural equality check
    format!("{a:?}") == format!("{b:?}")
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
}
