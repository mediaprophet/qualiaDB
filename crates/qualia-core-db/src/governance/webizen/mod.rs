//! Webizen VM — the Rights-Ontology governance gate over LLM/graph operations.
//!
//! Library-ized from the former `webizen.rs` (pure code motion, no behaviour change):
//! * [`arena`]     — the 42MB zero-allocation SLG tabling arena + N3 rule firing.
//! * [`opcode`]    — the `SlgOpcode` WAM instruction set.
//! * [`vm`]        — `VmFrame`, the VM helpers, and `execute_vm_frame`.
//! * [`agreement`] — agreement DIDs + the personhood-category-error guard.
//!
//! The full public surface is re-exported here, so every external path
//! (`crate::governance::webizen::<Item>`) resolves exactly as before.

use crate::domains::financial::tax_schema::TaxRuleSchema;
use crate::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, harvest_defeater_fingerprints,
    norm_has_active_defeater, DeonticStatus, DeonticVerdict, DEFEATER_BIT, MAX_DEFEATER_SLOTS,
    OP_PERMIT,
};
use crate::modalities::spatio_temporal;
use crate::modalities::temporal_ltl::{self, LtlFormula};
use crate::modalities::{
    abductive, argumentation, asp, ctl, defeasible, dialectical, dl, epistemic, fuzzy, linear,
    manifold, modal, paraconsistent, probabilistic,
};
use crate::NQuin;

macro_rules! vm_log {
    ($($arg:tt)*) => {
        if cfg!(feature = "vm_tracing") {
            println!($($arg)*);
        }
    };
}

// 42MB = 44,040,192 bytes
const SLG_ARENA_SIZE: usize = 42 * 1024 * 1024;
const QUIN_SIZE: usize = 48;
const MAX_SLOTS: usize = SLG_ARENA_SIZE / QUIN_SIZE; // 917,504 slots

use crate::modalities::logic::n3_compiler::{
    compile_rule_to_zero_heap, CompiledRule, CompiledTerm, CompiledTriple,
};
use crate::modalities::logic::n3_parser::Rule;

/// The 42MB Static Tabling Arena for SLG Resolution
/// Implemented as a Zero-Allocation Static Ring-Buffer Arena
const RECENT_SLOT_RING: usize = 512;

// ── Guard-rule grounding (forward chaining) bounds ──────────────────────────────
/// Max distinct variables bound per guard rule (premise + conclusion).
const MAX_RULE_VARS: usize = 16;
/// Max conclusion triples staged across one `fire_guard_rules` pass.
const MAX_GUARD_CONCLUSIONS: usize = 256;
/// Recursion-depth ceiling for the premise join (premise triple count).
const MAX_PREMISE_DEPTH: usize = 16;
/// Max forward-chaining rounds (fixpoint cap) for `fire_guard_rules`.
const MAX_FIXPOINT_ROUNDS: usize = 16;

mod agreement;
mod arena;
mod clinical_native;
mod opcode;
mod vm;

#[cfg(test)]
mod tests;

pub use agreement::*;
pub use arena::*;
pub use opcode::*;
pub use vm::*;
