//! Reversible staging and fail-closed receipt commit pipeline (`StagedGovernedCallRunner`, `VS03-10`).

use super::policy_gate::WebizenSentinelGate;
use crate::NQuin;
use vibe::extension_registry::{
    dispatch_prepared_call, CallExecutionContext, DispatchOutcome, ExtensionRegistry,
};
use vibescript_abi::{
    BorrowedSliceView, CallerOutputSlice, PreparedCallHandle, VibeTrap,
};

/// Executes a prepared extension call together with staged `NQuin` graph mutations,
/// committing the staged mutations into `published_graph[..*published_len]` **if and only if**:
/// 1. Sentinel admission succeeds,
/// 2. Provider execution succeeds within tick/byte envelopes,
/// 3. Post-execution validation succeeds (no mid-call policy revocation),
/// 4. Cryptographic/audit receipt `NQuin` persistence succeeds, and
/// 5. Every staged `NQuin` passes XOR parity validation and fits in `published_graph`.
///
/// If any stage fails, `published_graph` and `*published_len` remain completely unchanged.
pub fn execute_governed_staged_call(
    registry: &ExtensionRegistry<'_>,
    handle: &PreparedCallHandle,
    ctx: &CallExecutionContext,
    gate: &mut WebizenSentinelGate<'_>,
    inputs: &[BorrowedSliceView<'_>],
    output: &mut CallerOutputSlice<'_>,
    staged_mutations: &[NQuin],
    published_graph: &mut [NQuin],
    published_len: &mut usize,
) -> Result<DispatchOutcome, VibeTrap> {
    // Pre-verify that staged mutations have valid parity and fit into `published_graph`
    // before running or committing anything.
    if *published_len + staged_mutations.len() > published_graph.len() {
        return Err(VibeTrap::StagingAbortRequired);
    }
    let mut i = 0;
    while i < staged_mutations.len() {
        let q = &staged_mutations[i];
        let expected_parity = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
        if q.parity != expected_parity {
            return Err(VibeTrap::InvalidQuinParity);
        }
        i += 1;
    }

    // Run the 6-stage lifecycle (`Admit -> Execute -> Validate -> Commit Receipt`).
    // If receipt persistence fails (`VibeTrap::ReceiptPersistenceFailed`), `?` aborts
    // before `published_graph` is touched.
    let outcome = dispatch_prepared_call(
        registry,
        handle,
        ctx,
        Some(gate),
        inputs,
        output,
    )?;

    // Publish staged mutations atomically now that the receipt is durably committed.
    let base = *published_len;
    let mut j = 0;
    while j < staged_mutations.len() {
        published_graph[base + j] = staged_mutations[j];
        j += 1;
    }
    *published_len = base + staged_mutations.len();

    Ok(outcome)
}
