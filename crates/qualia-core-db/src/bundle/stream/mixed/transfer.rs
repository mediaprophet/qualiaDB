//! Production join: MixedHmc inventory → MuxTransfer + BoundEntry.
//! Family is an assignment tag. Admit before tagging Qac/Emf/P64.
use super::{mixed_plan, MixedError, MixedHmc, MixedPlan};
use crate::audio::qac::QacStorageOptions;
use crate::bundle::stream::descriptor::{EntryRecord, HASH_SCRATCH_BYTES};
use crate::bundle::stream::formats::{
    emf_present_id, p64_present_id, qac_present_id, AuthenticatedRange, EmfStreamView,
    P64StreamView, QacStreamView,
};
use crate::bundle::stream::index::{
    admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits,
};
use crate::bundle::stream::plan::{
    PrincipalSelection, SelectionIntent, SelectionRequest, MAX_PLAN_ENTRIES,
};
use crate::bundle::stream::policy::{BoundClosure, LocalWholeSourceGrant};
use crate::bundle::stream::proof::{
    verify_leaf_span, OutboardNode, SpanLimits, TreeBuilder, LEAF_BYTES,
};
use crate::bundle::stream::publish::PublicationClass;
use crate::bundle::stream::selection::SelectedPlainEntry;
use crate::crypto::network::authority::PinAuthority;
use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardLimits};
use crate::net::peer::replication::qbdl::service::{
    transfer_scheduled, BoundEntry, Error as MuxError, MuxReport, MuxTransfer, PlainClient,
    PlainProvider, ServiceIo,
};
use crate::net::qdnf::types::StrongDigest;
use crate::q42::field_envelope::Limits as EmfLimits;
const POLICY_GENERATION: u64 = 1;
fn map_mux(e: MuxError) -> MixedError {
    match e {
        MuxError::Limit | MuxError::Capacity | MuxError::Overflow => MixedError::Limit,
        MuxError::Unsupported | MuxError::Admission => MixedError::Unsupported,
        MuxError::Identity | MuxError::Malformed => MixedError::Identity,
        _ => MixedError::Identity,
    }
}
/// Scene intents for the listed assignment ids, in HMC order.
pub fn mixed_intents(hmc: &MixedHmc, ids: &[u32]) -> Result<Vec<SelectionIntent>, MixedError> {
    if ids.is_empty() || ids.len() > MAX_PLAN_ENTRIES {
        return Err(MixedError::Limit);
    }
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let i = hmc.index_of(*id).ok_or(MixedError::Identity)?;
        out.push(
            SelectionIntent::scene(*id, hmc.family(i), None, None)
                .map_err(|_| MixedError::Unsupported)?,
        );
    }
    Ok(out)
}
/// One principal over the listed assignment ids.
pub fn mixed_selection<'a>(
    intents: &'a [SelectionIntent],
) -> Result<PrincipalSelection<'a>, MixedError> {
    let request = SelectionRequest::scene(intents)?;
    Ok(PrincipalSelection::new(1, request)?)
}
/// Select every published assignment, ordinal order.
pub fn select_mixed<'a>(
    hmc: &'a MixedHmc,
    bound: &BoundClosure<'_>,
) -> Result<Vec<SelectedPlainEntry<'a>>, MixedError> {
    let header = admit_header(
        &hmc.source()[..64],
        hmc.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .map_err(|_| MixedError::Identity)?;
    let mut index_scratch = vec![EntryScratch::EMPTY; hmc.len()];
    let start = usize::try_from(header.index_offset()).map_err(|_| MixedError::Limit)?;
    let index = admit_index(
        &header,
        &hmc.source()[start..],
        IndexReadLimits::default(),
        &mut index_scratch,
    )
    .map_err(|_| MixedError::Identity)?;
    let mut rows = [EntryRecord::EMPTY; 128];
    let mut framing = vec![0; HASH_SCRATCH_BYTES];
    let mut out = Vec::with_capacity(hmc.len());
    for ordinal in 0..hmc.len() as u32 {
        out.push(hmc.select(bound, ordinal, &index, &mut rows, &mut framing)?);
    }
    Ok(out)
}
/// Cold outboard for one selected entry. Caller sorts once, then lends.
pub fn mixed_outboard(
    source: &[u8],
    selected: SelectedPlainEntry<'_>,
) -> Result<(Vec<OutboardNode>, StrongDigest), MixedError> {
    let start = usize::try_from(selected.offset()).map_err(|_| MixedError::Limit)?;
    let length = usize::try_from(selected.logical_length()).map_err(|_| MixedError::Limit)?;
    let end = start.checked_add(length).ok_or(MixedError::Limit)?;
    if end > source.len() {
        return Err(MixedError::Identity);
    }
    let entry = &source[start..end];
    let spec = selected.tree_spec().map_err(|_| MixedError::Identity)?;
    let mut builder = TreeBuilder::new(spec);
    let mut nodes = Vec::new();
    for leaf in entry.chunks(LEAF_BYTES as usize) {
        builder
            .append(leaf, |node| {
                nodes.push(node);
                Ok(())
            })
            .map_err(|_| MixedError::Identity)?;
    }
    let root = builder
        .finish(|node| {
            nodes.push(node);
            Ok(())
        })
        .map_err(|_| MixedError::Identity)?;
    nodes.sort_unstable_by_key(|node| (node.start_leaf, node.leaf_count));
    Ok((nodes, root))
}
fn catalog<'p, 'n>(
    selected: SelectedPlainEntry<'p>,
    nodes: &'n [OutboardNode],
    root: StrongDigest,
) -> Result<OutboardCatalog<'p, 'n>, MixedError> {
    let spec = selected.tree_spec().map_err(|_| MixedError::Identity)?;
    OutboardCatalog::new(
        selected,
        spec,
        root,
        nodes,
        OutboardLimits {
            max_nodes: nodes.len(),
            max_bytes: nodes.len() * core::mem::size_of::<OutboardNode>(),
        },
    )
    .map_err(|_| MixedError::Identity)
}
/// Bind one selected publication entry to its plain provider.
pub fn bind_mixed_entry<'src, 'sel, 'nodes>(
    source: &'src [u8],
    selected: SelectedPlainEntry<'sel>,
    nodes: &'nodes [OutboardNode],
    root: StrongDigest,
) -> Result<BoundEntry<'src, 'sel, 'nodes>, MixedError> {
    Ok(BoundEntry {
        entry_id: selected.entry_id(),
        provider: PlainProvider::bind(
            PublicationClass::PlainCandidate,
            source,
            catalog(selected, nodes, root)?,
            POLICY_GENERATION,
        )
        .map_err(map_mux)?,
    })
}
/// Bind every selected entry. `nodes`/`roots` must be 1:1 with `selected`.
pub fn bind_mixed_entries<'src, 'sel, 'nodes>(
    source: &'src [u8],
    selected: &[SelectedPlainEntry<'sel>],
    nodes: &'nodes [Vec<OutboardNode>],
    roots: &[StrongDigest],
) -> Result<Vec<BoundEntry<'src, 'sel, 'nodes>>, MixedError> {
    if selected.len() != nodes.len() || selected.len() != roots.len() {
        return Err(MixedError::Identity);
    }
    let mut out = Vec::with_capacity(selected.len());
    for i in 0..selected.len() {
        out.push(bind_mixed_entry(source, selected[i], &nodes[i], roots[i])?);
    }
    Ok(out)
}
/// Bind clients 1:1 with selected entries.
pub fn bind_mixed_clients<'p>(
    selected: &[SelectedPlainEntry<'p>],
) -> Result<Vec<PlainClient<'p>>, MixedError> {
    let mut out = Vec::with_capacity(selected.len());
    for sel in selected {
        out.push(PlainClient::bind(*sel, POLICY_GENERATION, 0).map_err(map_mux)?);
    }
    Ok(out)
}
/// Compile the fair schedule from a mixed plan. Incomplete/Refused stay on
/// `MuxTransfer` and refuse at `transfer_mixed` before any leaf.
pub fn compile_mixed_mux(
    plan: &MixedPlan,
    selections: &[PrincipalSelection<'_>],
    present: &[u32],
) -> Result<MuxTransfer, MixedError> {
    let inventory = plan.inventory()?;
    MuxTransfer::compile(&inventory, selections, present).map_err(map_mux)
}
fn with_complete_range<R>(
    selected: SelectedPlainEntry<'_>,
    payload: &[u8],
    run: impl FnOnce(AuthenticatedRange<'_>) -> Result<R, MixedError>,
) -> Result<R, MixedError> {
    let spec = selected.tree_spec().map_err(|_| MixedError::Identity)?;
    let mut siblings = [StrongDigest::ZERO; 32];
    let span = verify_leaf_span(
        &spec,
        selected.root(),
        0,
        payload,
        &[&[0]],
        &mut siblings,
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: LEAF_BYTES as usize,
        },
    )
    .map_err(|_| MixedError::Identity)?;
    let range = AuthenticatedRange::new(selected, &span, 0, payload.len())
        .map_err(|_| MixedError::Identity)?;
    run(range)
}
/// Present ids after family admit. Opaque is never present. Qac/Emf/P64 require
/// their stream view. TenD/Q42/M64 stay present after select (their views are
/// split-range, not complete-entry).
pub fn mixed_present_ids(
    hmc: &MixedHmc,
    bound: &BoundClosure<'_>,
) -> Result<Vec<u32>, MixedError> {
    use crate::bundle::stream::plan::EntryFamily;
    let selected = select_mixed(hmc, bound)?;
    let mut present = Vec::new();
    for (i, sel) in selected.iter().enumerate() {
        let id = hmc.entry_id(i);
        if sel.entry_id() != id {
            return Err(MixedError::Identity);
        }
        let payload = hmc.payload(i);
        match hmc.family(i) {
            EntryFamily::Opaque => {}
            EntryFamily::Qac => {
                if with_complete_range(*sel, payload, |range| {
                    let view = QacStreamView::admit(range, QacStorageOptions::default())
                        .map_err(|_| MixedError::Unsupported)?;
                    Ok(qac_present_id(id, &view).is_some())
                })? {
                    present.push(id);
                }
            }
            EntryFamily::Emf => {
                if with_complete_range(*sel, payload, |range| {
                    let view = EmfStreamView::admit(range, EmfLimits::default())
                        .map_err(|_| MixedError::Unsupported)?;
                    Ok(emf_present_id(id, &view).is_some())
                })? {
                    present.push(id);
                }
            }
            EntryFamily::P64 => {
                if with_complete_range(*sel, payload, |range| {
                    let view = P64StreamView::admit(range).map_err(|_| MixedError::Unsupported)?;
                    Ok(p64_present_id(id, &view).is_some())
                })? {
                    present.push(id);
                }
            }
            EntryFamily::TenD | EntryFamily::Q42 | EntryFamily::M64 => {
                present.push(id);
            }
        }
    }
    Ok(present)
}
/// Drive `next` until empty over the compiled mixed schedule.
pub fn transfer_mixed<PS: ServiceIo, CS: ServiceIo>(
    mux: &mut MuxTransfer,
    entries: &[BoundEntry<'_, '_, '_>],
    clients: &mut [PlainClient<'_>],
    provider_session: &mut PS,
    client_session: &mut CS,
    grant: &mut LocalWholeSourceGrant<'_, '_>,
    pins: &PinAuthority,
    clock: impl Fn() -> u64,
    payload_out: &mut [u8],
) -> Result<MuxReport, MixedError> {
    transfer_scheduled(
        mux,
        entries,
        clients,
        provider_session,
        client_session,
        grant,
        pins,
        clock,
        payload_out,
    )
    .map_err(map_mux)
}
/// Convenience: plan + present + compile from one published HMC.
pub fn compile_from_mixed(
    hmc: &MixedHmc,
    bound: &BoundClosure<'_>,
    selections: &[PrincipalSelection<'_>],
) -> Result<(MixedPlan, Vec<u32>, MuxTransfer), MixedError> {
    let plan = mixed_plan(hmc)?;
    let present = mixed_present_ids(hmc, bound)?;
    let mux = compile_mixed_mux(&plan, selections, &present)?;
    Ok((plan, present, mux))
}
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "transfer_tests.rs"]
mod transfer_tests;
