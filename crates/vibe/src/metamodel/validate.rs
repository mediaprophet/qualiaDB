//! Coverage checks: metamodel registry vs AST / effects / types / catalog IDs.

use crate::catalog::ALL_INVOKE_IDS;

use super::node_kinds::{
    EFFECT_KINDS, EXPR_KINDS, ITEM_KINDS, MODAL_KINDS, PATTERN_KINDS, STMT_KINDS, TYPE_KINDS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageStatus {
    Complete,
    Incomplete { missing: Vec<&'static str> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    pub items: CoverageStatus,
    pub stmts: CoverageStatus,
    pub patterns: CoverageStatus,
    pub exprs: CoverageStatus,
    pub modals: CoverageStatus,
    pub effects: CoverageStatus,
    pub types: CoverageStatus,
    pub catalog_invoke_count: usize,
}

fn expect_count(label: &'static str, have: usize, need: usize) -> CoverageStatus {
    if have >= need {
        CoverageStatus::Complete
    } else {
        CoverageStatus::Incomplete {
            missing: vec![label],
        }
    }
}

/// Report metamodel coverage against known AST category sizes.
///
/// Counts are pinned to the vibe-0.1 enums at the time of P1A; if a variant is
/// added, extend `node_kinds.rs` in the same change.
pub fn coverage_report() -> CoverageReport {
    CoverageReport {
        items: expect_count("Item", ITEM_KINDS.len(), 11),
        stmts: expect_count("Stmt", STMT_KINDS.len(), 13),
        patterns: expect_count("Pattern", PATTERN_KINDS.len(), 11),
        exprs: expect_count("ExprKind", EXPR_KINDS.len(), 23),
        modals: expect_count("ModalKind", MODAL_KINDS.len(), 11),
        effects: expect_count("EffectClass", EFFECT_KINDS.len(), 5),
        types: expect_count("Type", TYPE_KINDS.len(), 41),
        catalog_invoke_count: ALL_INVOKE_IDS.len(),
    }
}

impl CoverageReport {
    pub fn all_complete(&self) -> bool {
        matches!(self.items, CoverageStatus::Complete)
            && matches!(self.stmts, CoverageStatus::Complete)
            && matches!(self.patterns, CoverageStatus::Complete)
            && matches!(self.exprs, CoverageStatus::Complete)
            && matches!(self.modals, CoverageStatus::Complete)
            && matches!(self.effects, CoverageStatus::Complete)
            && matches!(self.types, CoverageStatus::Complete)
            && self.catalog_invoke_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cbor_ast;

    #[test]
    fn metamodel_covers_ast_categories() {
        let r = coverage_report();
        assert!(r.all_complete(), "{r:?}");
    }

    #[test]
    fn tag_4200_fixture_bytes_unchanged_smoke() {
        // Existing codec round-trip path must remain callable; detailed fixtures
        // live under cbor_ast module tests — this only proves linkage.
        let _ = cbor_ast::TAG_VIBE_AST;
        assert_eq!(cbor_ast::TAG_VIBE_AST, 4200);
    }
}
