//! HTTP mixed-plan driver. C5 `HttpProvider` plus C4 `Schedule::next`.
//! Headers are the adapter. Bodies stay exact representation bytes.
//! JSON is not an application object. HTTP status is not a grant.
//! Incomplete mandatory is not 200/206. TLS/Web PKI is not PQ publisher authority.
use super::{
    FallbackPolicy, HttpObject, HttpProvider, InclusiveRange, ProviderError, ServeOptions, Served,
    Validator,
};
use crate::audio::qac::{QacStorageOptions, QacStorageView};
use crate::bundle::stream::formats::{
    emf_present_id, p64_present_id, qac_present_id, AuthenticatedRange, EmfStreamView,
    P64StreamView, Q42AuthenticatedRangeSource, QacStreamView,
};
use crate::bundle::stream::plan::{
    evaluate, EntryFamily, Inventory, Obligation, PlanError, PrincipalSelection, Readiness,
    RecordUnit, Region, Schedule, ScheduledUnit, WorkClass, MAX_PLAN_ENTRIES, MAX_SCHEDULE_ITEMS,
};
use crate::bundle::stream::policy::{LocalEffectControl, LocalWholeSourceGrant};
use crate::bundle::stream::proof::{
    OutboardNode, SpanLimits, VerifiedLeafSpan, LEAF_BYTES, MAX_SIBLINGS,
};
use crate::bundle::stream::proof::verify_leaf_span;
use crate::bundle::stream::selection::SelectedPlainEntry;
use crate::crypto::network::authority::PinAuthority;
use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardLimits};
use crate::net::peer::runtime::ReservationLedger;
use crate::net::qdnf::types::StrongDigest;
use crate::q42::field_envelope::{Family, Limits, StructuralView};

/// One selected publication entry bound to its HTTP provider.
pub struct BoundHttpEntry<'src, 'sel, 'nodes, 'meta> {
    pub entry_id: u32,
    pub provider: HttpProvider<'src, 'sel, 'nodes, 'meta>,
}

/// One representation actually written into the caller `out` buffer.
#[derive(Clone, Copy)]
pub struct ServedObject {
    pub entry_id: u32,
    pub family: EntryFamily,
    pub status: u16,
    pub body_off: usize,
    pub body_len: usize,
    pub total: u64,
    pub etag: [u8; 98],
    pub etag_len: usize,
}

/// Counts from one `serve_scheduled` walk. No `Debug` — tests `match` errors.
pub struct HttpMuxReport {
    pub transferred: u16,
    pub control: u16,
    pub delayed_deferred: u16,
    pub cancelled_skipped: u16,
    pub shared_reused: u16,
    pub order: [Option<EntryFamily>; MAX_SCHEDULE_ITEMS],
    pub order_len: u16,
    pub objects: [Option<ServedObject>; MAX_SCHEDULE_ITEMS],
    pub objects_len: u16,
}

/// Compiled fair schedule plus the readiness gate for this HTTP walk.
pub struct HttpMux {
    schedule: Schedule,
    blocked: Option<ProviderError>,
    shared_reused: u16,
    cancelled_skipped: u16,
    deferred_pending: u16,
    deferred_ids: [u32; MAX_PLAN_ENTRIES],
    deferred_len: usize,
}

fn map_plan(e: PlanError) -> ProviderError {
    match e {
        PlanError::Capacity | PlanError::Limit => ProviderError::Capacity,
        PlanError::DuplicateId | PlanError::Identity | PlanError::Cycle | PlanError::Graph => {
            ProviderError::Identity
        }
        PlanError::Field | PlanError::Unsupported => ProviderError::Unsupported,
        PlanError::Refused => ProviderError::Policy,
    }
}

fn deferred_family(family: Option<EntryFamily>) -> bool {
    matches!(family, Some(EntryFamily::M64 | EntryFamily::P64))
}

/// Exact selected-entry bytes. Bounds refuse; this is not a grant.
pub fn entry_slice<'a>(
    source: &'a [u8],
    selected: SelectedPlainEntry<'_>,
) -> Result<&'a [u8], ProviderError> {
    let start = usize::try_from(selected.offset()).map_err(|_| ProviderError::Bounds)?;
    let len = usize::try_from(selected.logical_length()).map_err(|_| ProviderError::Bounds)?;
    let end = start.checked_add(len).ok_or(ProviderError::Bounds)?;
    if end > source.len() {
        return Err(ProviderError::Identity);
    }
    Ok(&source[start..end])
}

fn whole_span<'a>(
    selected: SelectedPlainEntry<'a>,
    bytes: &'a [u8],
    siblings: &'a mut [StrongDigest],
) -> Result<VerifiedLeafSpan<'a>, ProviderError> {
    if selected.logical_length() != bytes.len() as u64 {
        return Err(ProviderError::Identity);
    }
    let spec = selected.tree_spec().map_err(|_| ProviderError::Identity)?;
    verify_leaf_span(
        &spec,
        selected.root(),
        0,
        bytes,
        &[&[0]],
        siblings,
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: LEAF_BYTES as usize,
        },
    )
    .map_err(|_| ProviderError::Proof)
}

/// Family tags `Qac` / `Emf` only after whole-entry admit. A blob is not those families.
pub fn entry_admits_family(
    source: &[u8],
    selected: SelectedPlainEntry<'_>,
    family: EntryFamily,
) -> bool {
    let Ok(bytes) = entry_slice(source, selected) else {
        return false;
    };
    if selected.logical_length() != bytes.len() as u64 {
        return false;
    }
    match family {
        EntryFamily::Qac => {
            let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
            if let Ok(span) = whole_span(selected, bytes, &mut siblings) {
                if let Ok(range) = AuthenticatedRange::new(selected, &span, 0, bytes.len()) {
                    if let Ok(view) = QacStreamView::admit(range, QacStorageOptions::default()) {
                        return qac_present_id(selected.entry_id(), &view).is_some();
                    }
                }
            }
            QacStorageView::open(bytes, QacStorageOptions::default()).is_ok()
        }
        EntryFamily::Emf => {
            let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
            if let Ok(span) = whole_span(selected, bytes, &mut siblings) {
                if let Ok(range) = AuthenticatedRange::new(selected, &span, 0, bytes.len()) {
                    if let Ok(view) = EmfStreamView::admit(range, Limits::default()) {
                        return emf_present_id(selected.entry_id(), &view).is_some();
                    }
                }
            }
            StructuralView::open(bytes, Family::EmfV4Candidate, Limits::default()).is_ok()
        }
        EntryFamily::Q42 => {
            let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
            let Ok(span) = whole_span(selected, bytes, &mut siblings) else {
                return false;
            };
            let Ok(range) = AuthenticatedRange::new(selected, &span, 0, bytes.len()) else {
                return false;
            };
            Q42AuthenticatedRangeSource::new(selected, &[range], bytes.len() as u64, 1).is_ok()
        }
        EntryFamily::Opaque => false,
        EntryFamily::P64 => {
            let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
            if let Ok(span) = whole_span(selected, bytes, &mut siblings) {
                if let Ok(range) = AuthenticatedRange::new(selected, &span, 0, bytes.len()) {
                    if let Ok(view) = P64StreamView::admit(range) {
                        return p64_present_id(selected.entry_id(), &view).is_some();
                    }
                }
            }
            false
        }
        EntryFamily::TenD | EntryFamily::M64 => true,
    }
}

/// Bind C5 provider to one selected entry's outboard. Does not wrap `HttpRangeSource`.
pub fn bind_http_entry<'src, 'sel, 'nodes, 'meta>(
    source: &'src [u8],
    selected: SelectedPlainEntry<'sel>,
    nodes: &'nodes [OutboardNode],
    root: StrongDigest,
    descriptor: &'meta [u8],
    entry_page: &'meta [u8],
) -> Result<BoundHttpEntry<'src, 'sel, 'nodes, 'meta>, ProviderError> {
    let spec = selected.tree_spec().map_err(|_| ProviderError::Identity)?;
    let catalog = OutboardCatalog::new(
        selected,
        spec,
        root,
        nodes,
        OutboardLimits {
            max_nodes: nodes.len(),
            max_bytes: nodes.len() * core::mem::size_of::<OutboardNode>(),
        },
    )?;
    Ok(BoundHttpEntry {
        entry_id: selected.entry_id(),
        provider: HttpProvider::bind(source, catalog, descriptor, entry_page)?,
    })
}

impl HttpMux {
    pub fn compile(
        inventory: &Inventory<'_>,
        selections: &[PrincipalSelection<'_>],
        present: &[u32],
    ) -> Result<Self, ProviderError> {
        let (blocked, shared_reused, deferred_ids, deferred_len) =
            inspect(inventory, selections, present);
        let schedule = Schedule::compile(inventory, selections, present).map_err(map_plan)?;
        Ok(Self {
            schedule,
            blocked,
            shared_reused,
            cancelled_skipped: 0,
            deferred_pending: deferred_len as u16,
            deferred_ids,
            deferred_len,
        })
    }

    pub fn next_unit(&mut self) -> Option<ScheduledUnit> {
        self.schedule.next()
    }

    pub fn acknowledge(&mut self, unit: ScheduledUnit) -> Result<(), ProviderError> {
        self.schedule
            .acknowledge_complete(unit.principal(), unit.entry_id(), unit.unit().leaf())
            .map_err(map_plan)
    }

    pub fn acknowledge_complete(
        &mut self,
        principal: u64,
        entry_id: Option<u32>,
        leaf: u64,
    ) -> Result<(), ProviderError> {
        self.schedule
            .acknowledge_complete(principal, entry_id, leaf)
            .map_err(map_plan)
    }

    pub fn cancel(&mut self, entry_id: u32, region: Option<Region>) -> usize {
        let n = self.schedule.cancel(entry_id, region);
        if n > 0 {
            self.cancelled_skipped = self.cancelled_skipped.saturating_add(n as u16);
            if self.is_deferred_id(entry_id) {
                self.deferred_pending = self.deferred_pending.saturating_sub(1);
            }
        }
        n
    }

    pub fn occupancy_retained(&self, principal: u64, entry_id: Option<u32>, leaf: u64) -> bool {
        self.schedule.occupancy_retained(principal, entry_id, leaf)
    }

    pub fn push_control(&mut self, principal: u64, unit: RecordUnit) -> Result<(), ProviderError> {
        self.schedule.push_control(principal, unit).map_err(map_plan)
    }

    pub fn is_charged(&self, entry_id: u32) -> bool {
        self.schedule.is_charged(self.schedule.candidate(), entry_id)
    }

    pub const fn cancelled_skipped(&self) -> u16 {
        self.cancelled_skipped
    }

    pub const fn shared_reused(&self) -> u16 {
        self.shared_reused
    }

    pub const fn len(&self) -> usize {
        self.schedule.len()
    }

    fn is_deferred_id(&self, entry_id: u32) -> bool {
        self.deferred_ids[..self.deferred_len]
            .iter()
            .any(|id| *id == entry_id)
    }
}

/// Drive `next` until empty. Control acknowledges with no HTTP body.
/// `HttpProvider` always reserves: skip a second serve of the same entry identity.
/// One grant scopes the walk (`LocalWholeSourceGrant` is one-shot).
pub fn serve_scheduled(
    mux: &mut HttpMux,
    entries: &[BoundHttpEntry<'_, '_, '_, '_>],
    grant: &mut LocalWholeSourceGrant<'_, '_>,
    pins: &PinAuthority,
    clock: impl Fn() -> u64,
    ledger: &mut ReservationLedger,
    out: &mut [u8],
) -> Result<HttpMuxReport, ProviderError> {
    if let Some(e) = mux.blocked {
        return Err(e);
    }
    match grant.accept_local_scoped(pins, clock, |authority| {
        Ok(drive_scheduled(mux, entries, authority, ledger, out))
    }) {
        Ok(inner) => inner,
        Err(_) => Err(ProviderError::Policy),
    }
}

fn drive_scheduled(
    mux: &mut HttpMux,
    entries: &[BoundHttpEntry<'_, '_, '_, '_>],
    authority: &mut LocalEffectControl<'_, '_>,
    ledger: &mut ReservationLedger,
    out: &mut [u8],
) -> Result<HttpMuxReport, ProviderError> {
    let mut report = HttpMuxReport {
        transferred: 0,
        control: 0,
        delayed_deferred: 0,
        cancelled_skipped: mux.cancelled_skipped,
        shared_reused: mux.shared_reused,
        order: [None; MAX_SCHEDULE_ITEMS],
        order_len: 0,
        objects: [None; MAX_SCHEDULE_ITEMS],
        objects_len: 0,
    };
    let mut sent = [None; MAX_SCHEDULE_ITEMS];
    let mut sent_len = 0usize;
    let mut cursor = 0usize;
    while let Some(unit) = mux.next_unit() {
        push_order(&mut report, unit.family());
        if unit.entry_id().is_none() || unit.class() == WorkClass::Control {
            if mux.deferred_pending > 0 {
                report.delayed_deferred = report.delayed_deferred.saturating_add(1);
            }
            mux.acknowledge(unit)?;
            report.control = report.control.saturating_add(1);
            continue;
        }
        let id = match unit.entry_id() {
            Some(id) => id,
            None => return Err(ProviderError::Identity),
        };
        let family = match unit.family() {
            Some(f) => f,
            None => return Err(ProviderError::Identity),
        };
        let leaf = unit.unit().leaf();
        if sent[..sent_len].iter().any(|slot| *slot == Some((id, leaf))) {
            report.shared_reused = report.shared_reused.saturating_add(1);
            mux.acknowledge(unit)?;
            continue;
        }
        let idx = match entries.iter().position(|e| e.entry_id == id) {
            Some(i) => i,
            None => return Err(ProviderError::Identity),
        };
        let deferred = deferred_family(unit.family());
        if !deferred && mux.deferred_pending > 0 {
            report.delayed_deferred = report.delayed_deferred.saturating_add(1);
        }
        let provider = &entries[idx].provider;
        let total = provider.selected().logical_length();
        let range = InclusiveRange::from_start_length(0, total)?;
        let mut etag = [0u8; 98];
        let etag_len = provider.representation_etag(&mut etag)?;
        let served: Served = provider.serve(
            HttpObject::Representation,
            Some(range),
            Some(&etag[..etag_len]),
            Validator {
                etag: Some(&etag[..etag_len]),
                last_modified: None,
            },
            ServeOptions {
                fallback: FallbackPolicy::FORBIDDEN,
                verified_peer: true,
            },
            authority,
            ledger,
            &mut out[cursor..],
        )?;
        if served.status != 200 && served.status != 206 {
            return Err(ProviderError::Unsatisfiable);
        }
        let object = ServedObject {
            entry_id: id,
            family,
            status: served.status,
            body_off: cursor,
            body_len: served.body_len,
            total: served.total,
            etag: served.headers.etag,
            etag_len: served.headers.etag_len,
        };
        cursor = cursor
            .checked_add(served.body_len)
            .ok_or(ProviderError::Capacity)?;
        if report.objects_len as usize >= MAX_SCHEDULE_ITEMS {
            return Err(ProviderError::Capacity);
        }
        report.objects[report.objects_len as usize] = Some(object);
        report.objects_len = report.objects_len.saturating_add(1);
        if sent_len < MAX_SCHEDULE_ITEMS {
            sent[sent_len] = Some((id, leaf));
            sent_len += 1;
        }
        if deferred {
            mux.deferred_pending = mux.deferred_pending.saturating_sub(1);
        }
        mux.acknowledge(unit)?;
        report.transferred = report.transferred.saturating_add(1);
    }
    Ok(report)
}

fn push_order(report: &mut HttpMuxReport, family: Option<EntryFamily>) {
    let i = report.order_len as usize;
    if i < MAX_SCHEDULE_ITEMS {
        report.order[i] = family;
        report.order_len = report.order_len.saturating_add(1);
    }
}

fn inspect(
    inventory: &Inventory<'_>,
    selections: &[PrincipalSelection<'_>],
    present: &[u32],
) -> (
    Option<ProviderError>,
    u16,
    [u32; MAX_PLAN_ENTRIES],
    usize,
) {
    let mut blocked = None;
    let mut cover = [0u8; MAX_PLAN_ENTRIES];
    for sel in selections {
        let mut seen = [false; MAX_PLAN_ENTRIES];
        for intent in sel.request().intents() {
            match evaluate(
                inventory,
                intent.entry_id(),
                Some(intent.family()),
                present,
            ) {
                Readiness::Refused | Readiness::Incomplete { .. } => {
                    blocked = Some(ProviderError::Policy);
                }
                Readiness::Ready | Readiness::Opaque => {}
            }
            walk_ready(
                inventory,
                intent.entry_id(),
                present,
                &mut seen,
                &mut cover,
            );
        }
    }
    let mut shared = 0u16;
    for c in cover {
        if c > 1 {
            shared = shared.saturating_add(u16::from(c - 1));
        }
    }
    let mut deferred_ids = [0u32; MAX_PLAN_ENTRIES];
    let mut deferred_len = 0usize;
    for (i, entry) in inventory.entries().iter().enumerate() {
        if cover[i] == 0 || !deferred_family(Some(entry.family())) {
            continue;
        }
        if deferred_len < MAX_PLAN_ENTRIES {
            deferred_ids[deferred_len] = entry.entry_id();
            deferred_len += 1;
        }
    }
    (blocked, shared, deferred_ids, deferred_len)
}

fn walk_ready(
    inventory: &Inventory<'_>,
    start: u32,
    present: &[u32],
    seen: &mut [bool; MAX_PLAN_ENTRIES],
    cover: &mut [u8; MAX_PLAN_ENTRIES],
) {
    let mut stack = [0u32; MAX_PLAN_ENTRIES];
    let mut sp = 0usize;
    push_id(inventory, &mut stack, &mut sp, seen, start);
    while sp > 0 {
        sp -= 1;
        let id = stack[sp];
        let Some(entry) = inventory.entry(id) else {
            continue;
        };
        let readiness = evaluate(inventory, id, Some(entry.family()), present);
        if readiness == Readiness::Ready {
            if let Some(idx) = inventory.index_of(id) {
                cover[idx] = cover[idx].saturating_add(1);
            }
        }
        for edge in inventory.edges() {
            if edge.from_id() != id {
                continue;
            }
            let dep = edge.to_id();
            let include = match edge.obligation() {
                Obligation::Mandatory => true,
                Obligation::Optional => evaluate(inventory, dep, None, present) == Readiness::Ready,
            };
            if include {
                push_id(inventory, &mut stack, &mut sp, seen, dep);
            }
        }
    }
}

fn push_id(
    inventory: &Inventory<'_>,
    stack: &mut [u32; MAX_PLAN_ENTRIES],
    sp: &mut usize,
    seen: &mut [bool; MAX_PLAN_ENTRIES],
    id: u32,
) {
    let Some(idx) = inventory.index_of(id) else {
        return;
    };
    if seen[idx] {
        return;
    }
    if *sp >= MAX_PLAN_ENTRIES {
        return;
    }
    seen[idx] = true;
    stack[*sp] = id;
    *sp += 1;
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod mixed_http_tests {
    //! EvidenceClass::Component (`net/qdnf/harness/qualification.rs`).
    //! Never Process. Never PhysicalNetwork.
    use super::{
        bind_http_entry, entry_admits_family, entry_slice, serve_scheduled, whole_span,
        AuthenticatedRange, BoundHttpEntry, EntryFamily, FallbackPolicy, HttpMux, HttpObject,
        InclusiveRange, OutboardNode, ProviderError, ReservationLedger, ServeOptions, ServedObject,
        Validator, LEAF_BYTES, MAX_SIBLINGS,
    };
    use crate::bundle::stream::proof::TreeBuilder;
    use super::super::{
        get_range, tls_is_pq_publisher_authority, write_content_range, write_strong_etag,
        FetchBudget, Fetched, LigGate, RangeRequest, HEADER_CAP, LEGACY_FULL_OBJECT_FALLBACK_BYTES,
    };
    use crate::bundle::stream::{
        descriptor::{
            CandidateIdentity, ContextParameters, EntryRecord, PageRef, HASH_SCRATCH_BYTES,
            MAX_ENCODED, MAX_METADATA,
        },
        index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
        plan::{
            DependencyEdge, DependencyReason, Inventory, Obligation, PlanEntry, PlanEntrySpec,
            PrincipalSelection, RecordUnit, RepresentationClass, ResourceEstimate,
            SelectionIntent, SelectionRequest,
        },
        policy::{
            adopt_local_whole_source, authenticate_closure, encode_manifest_into, BoundClosure,
            LocalGrantRequest, ManifestFields, PublicationProof, CLOSURE_PURPOSE, MAX_MANIFEST_BYTES,
            PUBLICATION_PURPOSE,
        },
        publish::{EntryAssignment, PlainPublisher, PublicationClass, PublisherOptions},
        selection::{select_entry, SelectedPlainEntry},
    };
    use crate::bundle::{BundleReadLimits, BundleWriter};
    use crate::crypto::network::{
        authority::{LocalBootstrap, PinAuthority, PinConfig, PinHandle},
        cose,
        digest::sha384,
        types::{KeyEpoch, KeyPurpose},
    };
    use crate::mini_parser::hash_token;
    use crate::net::peer::runtime::ResourceBudget;
    use crate::net::qdnf::authority::AuthorityOwner;
    use crate::net::qdnf::types::{OperationId, StrongDigest};
    use crate::q42::q42_volume::{
        validate_exact_range_response, write_unified_volume, Q42ByteRange, Q42RangeSource,
        Q42RangeVolume,
    };
    use crate::semantic_instruments::resolve::{bounded_closure::*, DependencyKind};
    use crate::NQuin;
    use crate::bundle::stream::formats::Q42AuthenticatedRangeSource;
    use ed25519_dalek::SigningKey;
    use fips204::{
        ml_dsa_65,
        traits::{KeyGen, SerDes},
    };
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    const QAC_OPAQUE: &[u8] =
        include_bytes!("../../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
    const QAC_PCM: &[u8] = include_bytes!(
        "../../../q42/field_envelope/fixtures/candidate/qac_candidate-pcm-signed-zero.qac"
    );
    const EMF_OPAQUE: &[u8] =
        include_bytes!("../../../q42/field_envelope/fixtures/candidate/emf_candidate-opaque.emf");
    const EMF_ALIGN: &[u8] = include_bytes!(
        "../../../q42/field_envelope/fixtures/candidate/emf_candidate-aligned4096.emf"
    );
    const M64_BYTES: &[u8] = b"m64-delay-payload-not-a-field";

    const ID_QAC_OPAQUE: u32 = 10;
    const ID_QAC_PCM: u32 = 11;
    const ID_EMF_OPAQUE: u32 = 20;
    const ID_EMF_ALIGN: u32 = 21;
    const ID_M64: u32 = 30;
    const ID_Q42: u32 = 40;
    const ID_CALIB: u32 = 60;

    const PATH: &[u8] = b"/objects/mixed/representations/10";

    struct Scene {
        nodes: [ClosureNode<'static>; 3],
        edges: [ClosureEdge; 2],
        fields: ManifestFields,
        manifest: Vec<u8>,
        body: Vec<u8>,
        source: Vec<u8>,
        entry_page: Vec<u8>,
        identity: CandidateIdentity,
        ml_sk: ml_dsa_65::PrivateKey,
        ml_pk: [u8; 1952],
        ed_sk: SigningKey,
        ed_pk: [u8; 32],
    }

    impl Scene {
        fn source(&self) -> &[u8] {
            &self.source
        }
        fn body(&self) -> &[u8] {
            &self.body
        }
        fn entry_page(&self) -> &[u8] {
            &self.entry_page
        }
        fn identity(&self) -> CandidateIdentity {
            self.identity
        }
        fn publish(files: &[(&str, &str, &[u8], u32)]) -> Self {
            let nodes = [
                ClosureNode {
                    digest: sha384(b"root"),
                    kind: DependencyKind::Instrument,
                    original_bytes: b"root",
                },
                ClosureNode {
                    digest: sha384(b"policy"),
                    kind: DependencyKind::LawPackage,
                    original_bytes: b"policy",
                },
                ClosureNode {
                    digest: sha384(b"consumer"),
                    kind: DependencyKind::NativeModule,
                    original_bytes: b"consumer",
                },
            ];
            let edges = [
                ClosureEdge {
                    dependent: nodes[0].digest,
                    dependency: nodes[1].digest,
                },
                ClosureEdge {
                    dependent: nodes[0].digest,
                    dependency: nodes[2].digest,
                },
            ];
            let mut writer = BundleWriter::new();
            for (key, kind, bytes, _) in files {
                writer
                    .add_file(*key, *kind, bytes.to_vec(), None)
                    .unwrap();
            }
            let source = writer.build().unwrap();
            let fields = ManifestFields {
                source: sha384(&source),
                source_length: source.len() as u64,
                semantic_root: nodes[0].digest,
                policy: nodes[1].digest,
                consumer: nodes[2].digest,
                issuer: StrongDigest([31; 48]),
            };
            let candidate = verify_closure(nodes[0].digest, &nodes, &edges).unwrap();
            let mut manifest = vec![0; MAX_MANIFEST_BYTES];
            let n = encode_manifest_into(fields, &candidate, &mut manifest).unwrap();
            manifest.truncate(n);
            let parameters = ContextParameters {
                nonce: [1; 32],
                generation: 0,
                predecessor: None,
                semantic_contract: sha384(&manifest),
                policy: fields.policy,
                consumer_profile: fields.consumer,
            };
            let assignments: Vec<EntryAssignment> = files
                .iter()
                .map(|(_, _, _, id)| EntryAssignment {
                    id: *id,
                    inner_version: 1,
                })
                .collect();
            let mut index = vec![EntryScratch::EMPTY; assignments.len()];
            let mut ids = vec![0u32; assignments.len()];
            let mut context_refs = [PageRef::EMPTY; 32];
            let mut entry_refs = [PageRef::EMPTY; 32];
            let mut publisher = PlainPublisher::prepare(
                &source,
                BundleReadLimits::default(),
                &assignments,
                PublisherOptions {
                    parameters,
                    class: PublicationClass::PlainCandidate,
                    metadata_limit: MAX_METADATA,
                },
                &mut index,
                &mut ids,
                &mut context_refs,
                &mut entry_refs,
            )
            .unwrap();
            let mut output = vec![0; MAX_ENCODED];
            let mut scratch = vec![0; HASH_SCRATCH_BYTES];
            for page in 0..publisher.page_count() {
                publisher
                    .emit_context_page(page, &mut output, &mut scratch, |_, _| Ok(()))
                    .unwrap();
            }
            let n = publisher
                .encode_context_root(&mut output, &mut scratch)
                .unwrap();
            let context = output[..n].to_vec();
            let mut entry_page = Vec::new();
            for page in 0..publisher.page_count() {
                publisher
                    .emit_entry_page(
                        page,
                        &mut output,
                        &mut scratch,
                        |_, _| Ok(()),
                        |bytes, _| {
                            entry_page = bytes.to_vec();
                            Ok(())
                        },
                    )
                    .unwrap();
            }
            let identity = publisher
                .finish(&context, &mut output, &mut scratch)
                .unwrap();
            let n = publisher.body_encoded_length(&context).unwrap();
            let body = output[..n].to_vec();
            drop(publisher);
            let (ml_pk, ml_sk) = ml_dsa_65::KG::keygen_from_seed(&[71; 32]);
            let ed_sk = SigningKey::from_bytes(&[72; 32]);
            let ed_pk = ed_sk.verifying_key().to_bytes();
            Self {
                nodes,
                edges,
                fields,
                manifest,
                body,
                source,
                entry_page,
                identity,
                ml_sk,
                ml_pk: ml_pk.into_bytes(),
                ed_sk,
                ed_pk,
            }
        }
        fn keys(&self) -> cose::PairedPublicKeys<'_> {
            cose::PairedPublicKeys {
                ml_dsa_65: &self.ml_pk,
                ed25519: &self.ed_pk,
            }
        }
        fn installed(&self, purpose: &[u8]) -> (PinAuthority, LocalBootstrap, PinHandle, PinHandle) {
            let (mut pins, admin) = PinAuthority::new_local_bootstrap().unwrap();
            let handle = pins
                .install_pin(
                    &admin,
                    PinConfig {
                        principal: self.fields.issuer,
                        provider_epoch: KeyEpoch {
                            controller: 7,
                            epoch: 1,
                            purpose: KeyPurpose::ControllerSign,
                        },
                        keys: self.keys(),
                        purpose,
                        audience: b"reader",
                        not_before_unix: 10,
                        expires_unix: 100,
                    },
                    10,
                )
                .unwrap();
            let publisher = pins
                .install_pin(
                    &admin,
                    PinConfig {
                        principal: self.fields.issuer,
                        provider_epoch: KeyEpoch {
                            controller: 7,
                            epoch: 1,
                            purpose: KeyPurpose::ControllerSign,
                        },
                        keys: self.keys(),
                        purpose: PUBLICATION_PURPOSE,
                        audience: b"reader",
                        not_before_unix: 10,
                        expires_unix: 100,
                    },
                    10,
                )
                .unwrap();
            (pins, admin, handle, publisher)
        }
        fn signed_digest(&self, digest: StrongDigest, purpose: &[u8]) -> Vec<u8> {
            let binding = cose::Binding {
                descriptor_digest: &digest.0,
                key_ids: self.keys().key_ids(),
                controller_epoch: 1,
                purpose,
                audience: b"reader",
            };
            let keys = cose::SigningKeys {
                ml_dsa_65: &self.ml_sk,
                ed25519: &self.ed_sk,
            };
            let mut out = vec![0; cose::MAX_ENVELOPE_BYTES];
            let n = cose::sign_pair_raw(&binding, &keys, &mut cose::Workspace::new(), &mut out)
                .unwrap();
            out.truncate(n);
            out
        }
        fn authenticated<'a>(
            &'a self,
            pins: &PinAuthority,
            pin: PinHandle,
            now: u64,
        ) -> crate::bundle::stream::policy::AuthenticatedClosure<'a> {
            let candidate =
                verify_closure(self.fields.semantic_root, &self.nodes, &self.edges).unwrap();
            let signed = self.signed_digest(sha384(&self.manifest), CLOSURE_PURPOSE);
            authenticate_closure(
                candidate,
                &self.manifest,
                pins,
                pin,
                self.keys().key_ids(),
                1,
                b"reader",
                &signed,
                now,
                &mut cose::Workspace::new(),
            )
            .unwrap()
        }
        fn bound<'a>(
            &'a self,
            pins: &PinAuthority,
            pin: PinHandle,
            publisher: PinHandle,
        ) -> BoundClosure<'a> {
            self.authenticated(pins, pin, 10)
                .bind_descriptor(
                    self.identity,
                    &self.body,
                    &mut vec![0; HASH_SCRATCH_BYTES],
                    pins,
                    self.publication(
                        publisher,
                        &self.signed_digest(self.identity.descriptor_id(), PUBLICATION_PURPOSE),
                    ),
                    10,
                    &mut cose::Workspace::new(),
                )
                .unwrap()
        }
        fn publication<'a>(&self, pin: PinHandle, envelope: &'a [u8]) -> PublicationProof<'a> {
            PublicationProof {
                pin,
                key_ids: self.keys().key_ids(),
                epoch: 1,
                audience: b"reader",
                envelope,
            }
        }
        fn request(&self) -> LocalGrantRequest {
            LocalGrantRequest {
                principal: StrongDigest([41; 48]),
                recipient: StrongDigest([42; 48]),
                purpose: sha384(b"local-read"),
                operation: OperationId([1; 16]),
                not_before: 10,
                expires: 90,
                resource_ceiling: self.fields.source_length,
            }
        }
    }

    fn four_live() -> [(&'static str, &'static str, &'static [u8], u32); 4] {
        [
            ("qac-opaque", "qac", QAC_OPAQUE, ID_QAC_OPAQUE),
            ("qac-pcm", "qac", QAC_PCM, ID_QAC_PCM),
            ("emf-opaque", "emf", EMF_OPAQUE, ID_EMF_OPAQUE),
            ("emf-align", "emf", EMF_ALIGN, ID_EMF_ALIGN),
        ]
    }

    fn q42_volume_bytes() -> Vec<u8> {
        let subject = hash_token("urn:q42:authenticated-subject");
        let predicate = hash_token("urn:q42:authenticated-predicate");
        let object = hash_token("urn:q42:authenticated-object");
        let quin = NQuin {
            subject,
            predicate,
            object,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let mut lexicon = HashMap::new();
        lexicon.insert(subject, "urn:q42:authenticated-subject".to_string());
        lexicon.insert(predicate, "urn:q42:authenticated-predicate".to_string());
        lexicon.insert(object, "urn:q42:authenticated-object".to_string());
        let file = NamedTempFile::new().unwrap();
        write_unified_volume(file.path(), &lexicon, &[(object, object)], &[vec![quin]]).unwrap();
        let mut q42 = Vec::new();
        std::fs::File::open(file.path())
            .unwrap()
            .read_to_end(&mut q42)
            .unwrap();
        assert!(!q42.is_empty());
        assert!(q42.len() as u64 <= LEAF_BYTES);
        assert!(q42.len() as u64 <= LEGACY_FULL_OBJECT_FALLBACK_BYTES);
        q42
    }

    fn family_of(id: u32) -> EntryFamily {
        match id {
            ID_QAC_OPAQUE | ID_QAC_PCM => EntryFamily::Qac,
            ID_EMF_OPAQUE | ID_EMF_ALIGN => EntryFamily::Emf,
            ID_M64 => EntryFamily::M64,
            ID_Q42 => EntryFamily::Q42,
            ID_CALIB => EntryFamily::Q42,
            _ => EntryFamily::Opaque,
        }
    }

    fn plan_entry(
        candidate: CandidateIdentity,
        entry_id: u32,
        family: EntryFamily,
        role: RepresentationClass,
        obligation: Obligation,
        bytes: u64,
    ) -> PlanEntry {
        PlanEntry::new(PlanEntrySpec {
            candidate,
            entry_id,
            family,
            role,
            interval: None,
            timebase: None,
            estimate: ResourceEstimate::new(bytes, 1, 1),
            obligation,
        })
        .unwrap()
    }

    fn select_all<'p>(bound: &BoundClosure<'_>, scene: &'p Scene) -> Vec<SelectedPlainEntry<'p>> {
        let header = admit_header(
            &scene.source()[..64],
            scene.source().len() as u64,
            ExtentLimit::default(),
            IndexReadLimits::default(),
        )
        .unwrap();
        let count = scene.identity().entry_count() as usize;
        let mut index_scratch = vec![EntryScratch::EMPTY; count];
        let index = admit_index(
            &header,
            &scene.source()[header.index_offset() as usize..],
            IndexReadLimits::default(),
            &mut index_scratch,
        )
        .unwrap();
        let mut rows = [EntryRecord::EMPTY; 128];
        let mut framing = vec![0; HASH_SCRATCH_BYTES];
        let mut out = Vec::with_capacity(count);
        for ordinal in 0..count as u32 {
            out.push(
                select_entry(
                    bound,
                    scene.body(),
                    &scene.source()[..64],
                    &index,
                    scene.entry_page(),
                    ordinal,
                    &mut rows,
                    &mut framing,
                )
                .unwrap(),
            );
        }
        out
    }

    fn build_outboard(
        source: &[u8],
        selected: SelectedPlainEntry<'_>,
    ) -> (Vec<OutboardNode>, StrongDigest) {
        let entry = entry_slice(source, selected).unwrap();
        let spec = selected.tree_spec().unwrap();
        let mut builder = TreeBuilder::new(spec);
        let mut nodes = Vec::new();
        for leaf in entry.chunks(LEAF_BYTES as usize) {
            builder
                .append(leaf, |node| {
                    nodes.push(node);
                    Ok(())
                })
                .unwrap();
        }
        let root = builder
            .finish(|node| {
                nodes.push(node);
                Ok(())
            })
            .unwrap();
        assert_eq!(root, selected.root());
        nodes.sort_unstable_by_key(|node| (node.start_leaf, node.leaf_count));
        (nodes, root)
    }

    fn outboards(
        source: &[u8],
        selected: &[SelectedPlainEntry<'_>],
    ) -> (Vec<Vec<OutboardNode>>, Vec<StrongDigest>) {
        let mut nodes = Vec::new();
        let mut roots = Vec::new();
        for sel in selected {
            let (n, r) = build_outboard(source, *sel);
            nodes.push(n);
            roots.push(r);
        }
        (nodes, roots)
    }

    fn bind_entries<'src, 'sel, 'nodes, 'meta>(
        source: &'src [u8],
        selected: &[SelectedPlainEntry<'sel>],
        nodes: &'nodes [Vec<OutboardNode>],
        roots: &[StrongDigest],
        descriptor: &'meta [u8],
        entry_page: &'meta [u8],
    ) -> Vec<BoundHttpEntry<'src, 'sel, 'nodes, 'meta>> {
        selected
            .iter()
            .zip(nodes.iter())
            .zip(roots.iter())
            .map(|((sel, nodes), root)| {
                bind_http_entry(source, *sel, nodes, *root, descriptor, entry_page).unwrap()
            })
            .collect()
    }

    fn present_admitted(source: &[u8], selected: &[SelectedPlainEntry<'_>]) -> Vec<u32> {
        let mut present = Vec::new();
        for sel in selected {
            let family = family_of(sel.entry_id());
            if family == EntryFamily::Qac
                || family == EntryFamily::Emf
                || family == EntryFamily::Q42
            {
                if entry_admits_family(source, *sel, family) {
                    present.push(sel.entry_id());
                }
            } else if family != EntryFamily::Opaque {
                present.push(sel.entry_id());
            }
        }
        present
    }

    fn live_plan(
        candidate: CandidateIdentity,
        ids: &[u32],
        selected: &[SelectedPlainEntry<'_>],
    ) -> Vec<PlanEntry> {
        ids.iter()
            .map(|id| {
                let bytes = selected
                    .iter()
                    .find(|s| s.entry_id() == *id)
                    .map(|s| s.logical_length())
                    .unwrap_or(1);
                plan_entry(
                    candidate,
                    *id,
                    family_of(*id),
                    RepresentationClass::Base,
                    Obligation::Mandatory,
                    bytes,
                )
            })
            .collect()
    }

    fn selected_by_id<'p>(
        selected: &[SelectedPlainEntry<'p>],
        id: u32,
    ) -> SelectedPlainEntry<'p> {
        *selected.iter().find(|s| s.entry_id() == id).unwrap()
    }

    fn ledger() -> ReservationLedger {
        let cap = ResourceBudget {
            bytes: 8 << 20,
            work: 64,
            io: 64,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    fn spawn_http(
        status: u16,
        hdrs: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> (
        std::net::SocketAddr,
        Arc<Mutex<Vec<u8>>>,
        thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let cap = captured.clone();
        let th = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(2))).ok();
            s.set_write_timeout(Some(Duration::from_secs(2))).ok();
            let mut buf = [0u8; HEADER_CAP];
            let mut n = 0;
            while n < buf.len() {
                match s.read(&mut buf[n..]) {
                    Ok(0) => break,
                    Ok(k) => {
                        n += k;
                        if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            *cap.lock().unwrap() = buf[..n].to_vec();
            let mut resp = format!("HTTP/1.1 {status} X\r\n");
            for (k, v) in &hdrs {
                resp.push_str(k);
                resp.push_str(": ");
                resp.push_str(v);
                resp.push_str("\r\n");
            }
            resp.push_str("\r\n");
            let _ = s.write_all(resp.as_bytes());
            let _ = s.write_all(&body);
            let _ = s.flush();
        });
        (addr, captured, th)
    }

    fn connect(addr: std::net::SocketAddr) -> TcpStream {
        let s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        s.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
        s
    }

    fn object_body<'a>(out: &'a [u8], object: &ServedObject) -> &'a [u8] {
        &out[object.body_off..object.body_off + object.body_len]
    }

    #[test]
    fn schedule_order_matches_native_bytes_over_http() {
        assert!(!tls_is_pq_publisher_authority());
        let scene = Scene::publish(&four_live());
        let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
        let bound = scene.bound(&pins, pin, publisher);
        let local = select_all(&bound, &scene);
        assert_eq!(local.len(), 4);
        let (nodes, roots) = outboards(scene.source(), &local);
        let entries = bind_entries(
            scene.source(),
            &local,
            &nodes,
            &roots,
            scene.body(),
            scene.entry_page(),
        );
        let present = present_admitted(scene.source(), &local);
        assert_eq!(present.len(), 4);
        let ids = [ID_QAC_OPAQUE, ID_QAC_PCM, ID_EMF_OPAQUE, ID_EMF_ALIGN];
        let plan_entries = live_plan(scene.identity(), &ids, &local);
        let inv = Inventory::compile(scene.identity(), &plan_entries, &[]).unwrap();
        let intents = [
            SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_QAC_PCM, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_ALIGN, EntryFamily::Emf, None, None).unwrap(),
        ];
        let request = SelectionRequest::scene(&intents).unwrap();
        let sel = PrincipalSelection::new(1, request).unwrap();
        let mut mux = match HttpMux::compile(&inv, &[sel], &present) {
            Ok(m) => m,
            Err(_) => panic!("four live families must compile"),
        };
        mux.push_control(1, RecordUnit::at(0)).unwrap();
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut ledger = ledger();
        let mut out = vec![0u8; 64 * 1024];
        let report = match serve_scheduled(
            &mut mux,
            &entries,
            &mut grant,
            &pins,
            || 20,
            &mut ledger,
            &mut out,
        ) {
            Ok(r) => r,
            Err(_) => panic!("four live families must serve over HTTP"),
        };
        assert_eq!(report.control, 1);
        assert_eq!(report.transferred, 4);
        if report.order_len > 0 {
            assert_eq!(report.order[0], None);
        }
        let mut saw_qac = 0u16;
        let mut saw_emf = 0u16;
        for i in 0..report.order_len as usize {
            match report.order[i] {
                None => {}
                Some(EntryFamily::Qac) => saw_qac += 1,
                Some(EntryFamily::Emf) => saw_emf += 1,
                Some(_) => panic!("only QAC/EMF/Control in this scene"),
            }
        }
        assert_eq!(saw_qac, 2);
        assert_eq!(saw_emf, 2);
        let mut charged = 0u64;
        let mut first_object = None;
        for i in 0..report.objects_len as usize {
            let object = match report.objects[i] {
                Some(o) => o,
                None => panic!("served object slot"),
            };
            assert!(object.status == 206 || object.status == 200);
            let sel = selected_by_id(&local, object.entry_id);
            let native = entry_slice(scene.source(), sel).unwrap();
            assert_eq!(object_body(&out, &object), native);
            assert!(entry_admits_family(scene.source(), sel, object.family));
            charged = charged.saturating_add(object.body_len as u64);
            if first_object.is_none() {
                first_object = Some((object, native.to_vec()));
            }
        }
        assert_eq!(ledger.used().host.bytes, charged);

        let (object, native) = match first_object {
            Some(v) => v,
            None => panic!("at least one representation"),
        };
        let mut cr = [0u8; 80];
        let range = InclusiveRange::from_start_length(0, native.len() as u64).unwrap();
        let cn = write_content_range(range, native.len() as u64, &mut cr).unwrap();
        let etag_s = std::str::from_utf8(&object.etag[..object.etag_len])
            .unwrap()
            .to_string();
        let (addr, captured, th) = spawn_http(
            object.status,
            vec![
                (
                    "Content-Range".into(),
                    String::from_utf8_lossy(&cr[..cn]).into_owned(),
                ),
                ("Content-Length".into(), native.len().to_string()),
                ("Content-Encoding".into(), "identity".into()),
                ("ETag".into(), etag_s),
            ],
            native.clone(),
        );
        let host = format!("{addr}").into_bytes();
        let req = RangeRequest {
            host: &host,
            path: PATH,
            range,
            pinned_total: native.len() as u64,
            if_range: Some(&object.etag[..object.etag_len]),
            expected: Validator {
                etag: Some(&object.etag[..object.etag_len]),
                last_modified: None,
            },
            fallback: FallbackPolicy::FORBIDDEN,
            budget: FetchBudget {
                max_body: LEGACY_FULL_OBJECT_FALLBACK_BYTES,
                max_headers: HEADER_CAP,
            },
            lig: LigGate::consented_active(),
        };
        let mut stream = connect(addr);
        let mut client_out = vec![0u8; native.len()];
        let got = match get_range(&mut stream, &req, &mut client_out) {
            Ok(v) => v,
            Err(_) => panic!("hostile loopback must return the native slice"),
        };
        assert_eq!(
            got,
            Fetched::Partial {
                length: native.len()
            }
        );
        assert_eq!(&client_out[..native.len()], native.as_slice());
        let text = String::from_utf8_lossy(&captured.lock().unwrap());
        assert!(text.contains("Accept-Encoding: identity"));
        assert!(text.contains("Range: bytes="));
        assert!(text.contains("If-Range: "));
        assert!(!text.contains("Authorization:"));
        th.join().unwrap();
    }

    #[test]
    fn http_delays_m64_while_ready_qac_and_emf_continue() {
        let mut files = four_live().to_vec();
        files.push(("m64-delay", "blob", M64_BYTES, ID_M64));
        let scene = Scene::publish(&files);
        let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
        let bound = scene.bound(&pins, pin, publisher);
        let local = select_all(&bound, &scene);
        let (nodes, roots) = outboards(scene.source(), &local);
        let entries = bind_entries(
            scene.source(),
            &local,
            &nodes,
            &roots,
            scene.body(),
            scene.entry_page(),
        );
        let present = present_admitted(scene.source(), &local);
        assert!(present.contains(&ID_QAC_OPAQUE));
        assert!(present.contains(&ID_EMF_OPAQUE));
        assert!(present.contains(&ID_M64));
        let ids = [
            ID_QAC_OPAQUE,
            ID_QAC_PCM,
            ID_EMF_OPAQUE,
            ID_EMF_ALIGN,
            ID_M64,
        ];
        let plan_entries = live_plan(scene.identity(), &ids, &local);
        let inv = Inventory::compile(scene.identity(), &plan_entries, &[]).unwrap();
        let intents = [
            SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_QAC_PCM, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_ALIGN, EntryFamily::Emf, None, None).unwrap(),
            SelectionIntent::scene(ID_M64, EntryFamily::M64, None, None).unwrap(),
        ];
        let request = SelectionRequest::scene(&intents).unwrap();
        let sel = PrincipalSelection::new(1, request).unwrap();
        let mut mux = match HttpMux::compile(&inv, &[sel], &present) {
            Ok(m) => m,
            Err(_) => panic!("live QAC/EMF plus delayed M64 must compile"),
        };
        mux.push_control(1, RecordUnit::at(0)).unwrap();
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut ledger = ledger();
        let mut out = vec![0u8; 64 * 1024];
        let report = match serve_scheduled(
            &mut mux,
            &entries,
            &mut grant,
            &pins,
            || 20,
            &mut ledger,
            &mut out,
        ) {
            Ok(r) => r,
            Err(_) => panic!("live QAC/EMF plus delayed M64 must serve"),
        };
        assert!(report.delayed_deferred >= 1);
        assert_eq!(report.control, 1);
        assert_eq!(report.transferred, 5);
        let mut saw_m64 = false;
        for i in 0..report.order_len as usize {
            match report.order[i] {
                None | Some(EntryFamily::Qac) | Some(EntryFamily::Emf) => {
                    assert!(!saw_m64, "M64 must wait on live families");
                }
                Some(EntryFamily::M64) => {
                    assert!(i > 0, "Control/QAC/EMF run before M64");
                    saw_m64 = true;
                }
                Some(_) => panic!("unexpected family in delay scene"),
            }
        }
        assert!(saw_m64);
        let first_body = match report.objects[0] {
            Some(o) => o,
            None => panic!("first served representation"),
        };
        assert!(
            first_body.family == EntryFamily::Qac || first_body.family == EntryFamily::Emf,
            "first HTTP body is a live family"
        );
    }

    #[test]
    fn http_cancel_obsolete_qac_skips_body() {
        let scene = Scene::publish(&four_live());
        let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
        let bound = scene.bound(&pins, pin, publisher);
        let local = select_all(&bound, &scene);
        let (nodes, roots) = outboards(scene.source(), &local);
        let entries = bind_entries(
            scene.source(),
            &local,
            &nodes,
            &roots,
            scene.body(),
            scene.entry_page(),
        );
        let present = present_admitted(scene.source(), &local);
        let ids = [ID_QAC_OPAQUE, ID_QAC_PCM, ID_EMF_OPAQUE, ID_EMF_ALIGN];
        let plan_entries = live_plan(scene.identity(), &ids, &local);
        let inv = Inventory::compile(scene.identity(), &plan_entries, &[]).unwrap();
        let intents = [
            SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_QAC_PCM, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_ALIGN, EntryFamily::Emf, None, None).unwrap(),
        ];
        let request = SelectionRequest::scene(&intents).unwrap();
        let sel = PrincipalSelection::new(1, request).unwrap();
        let mut mux = match HttpMux::compile(&inv, &[sel], &present) {
            Ok(m) => m,
            Err(_) => panic!("cancel scene must compile"),
        };
        let cancelled = mux.cancel(ID_QAC_OPAQUE, None);
        assert!(cancelled >= 1);
        assert!(mux.occupancy_retained(1, Some(ID_QAC_OPAQUE), 0));
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut ledger = ledger();
        let mut out = vec![0u8; 64 * 1024];
        let report = match serve_scheduled(
            &mut mux,
            &entries,
            &mut grant,
            &pins,
            || 20,
            &mut ledger,
            &mut out,
        ) {
            Ok(r) => r,
            Err(_) => panic!("sibling EMF must continue after QAC cancel"),
        };
        assert!(report.cancelled_skipped >= 1);
        assert_eq!(report.transferred, 3);
        let mut saw_emf = 0u16;
        let mut saw_qac = 0u16;
        for i in 0..report.objects_len as usize {
            let object = match report.objects[i] {
                Some(o) => o,
                None => panic!("served object slot"),
            };
            assert_ne!(object.entry_id, ID_QAC_OPAQUE);
            assert!(object.status == 206 || object.status == 200);
            let sel = selected_by_id(&local, object.entry_id);
            let native = entry_slice(scene.source(), sel).unwrap();
            assert_eq!(object_body(&out, &object), native);
            match object.family {
                EntryFamily::Qac => saw_qac += 1,
                EntryFamily::Emf => saw_emf += 1,
                _ => panic!("unexpected family after cancel"),
            }
        }
        assert_eq!(saw_emf, 2);
        assert_eq!(saw_qac, 1);
        assert!(mux.occupancy_retained(1, Some(ID_QAC_OPAQUE), 0));
        mux.acknowledge_complete(1, Some(ID_QAC_OPAQUE), 0).unwrap();
        assert!(!mux.occupancy_retained(1, Some(ID_QAC_OPAQUE), 0));
    }

    #[test]
    fn http_incomplete_mandatory_is_not_200_or_206() {
        let scene = Scene::publish(&four_live());
        let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
        let bound = scene.bound(&pins, pin, publisher);
        let local = select_all(&bound, &scene);
        let (nodes, roots) = outboards(scene.source(), &local);
        let entries = bind_entries(
            scene.source(),
            &local,
            &nodes,
            &roots,
            scene.body(),
            scene.entry_page(),
        );
        let present = present_admitted(scene.source(), &local);
        let mut plan_entries = live_plan(scene.identity(), &present, &local);
        plan_entries.push(plan_entry(
            scene.identity(),
            ID_CALIB,
            EntryFamily::Q42,
            RepresentationClass::Metadata,
            Obligation::Mandatory,
            7,
        ));
        let qac0 = plan_entries
            .iter()
            .find(|e| e.entry_id() == ID_QAC_OPAQUE)
            .copied()
            .unwrap();
        let emf0 = plan_entries
            .iter()
            .find(|e| e.entry_id() == ID_EMF_OPAQUE)
            .copied()
            .unwrap();
        let calib = plan_entries
            .iter()
            .find(|e| e.entry_id() == ID_CALIB)
            .copied()
            .unwrap();
        let edges = [
            DependencyEdge::bind(
                &qac0,
                &calib,
                Obligation::Mandatory,
                DependencyReason::Calibration,
            )
            .unwrap(),
            DependencyEdge::bind(
                &emf0,
                &calib,
                Obligation::Mandatory,
                DependencyReason::Calibration,
            )
            .unwrap(),
        ];
        let inv = Inventory::compile(scene.identity(), &plan_entries, &edges).unwrap();
        let intents = [
            SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
        ];
        let request = SelectionRequest::scene(&intents).unwrap();
        let sel = PrincipalSelection::new(1, request).unwrap();
        let mut mux = match HttpMux::compile(&inv, &[sel], &present) {
            Ok(m) => m,
            Err(_) => panic!("incomplete scene still compiles; serve must refuse"),
        };
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut ledger = ledger();
        let mut out = [0xA5u8; 64];
        match serve_scheduled(
            &mut mux,
            &entries,
            &mut grant,
            &pins,
            || 20,
            &mut ledger,
            &mut out,
        ) {
            Err(ProviderError::Policy) | Err(ProviderError::Unsatisfiable) => {}
            Ok(_) => panic!("missing mandatory must not disclose 200/206"),
            Err(_) => panic!("missing mandatory maps to Policy or Unsatisfiable"),
        }
        assert!(out.iter().all(|b| *b == 0xA5));
        assert_eq!(ledger.used().host.bytes, 0);
    }

    #[test]
    fn http_two_qac_and_two_emf_are_distinct_objects() {
        let scene = Scene::publish(&four_live());
        let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
        let bound = scene.bound(&pins, pin, publisher);
        let local = select_all(&bound, &scene);
        let (nodes, roots) = outboards(scene.source(), &local);
        let entries = bind_entries(
            scene.source(),
            &local,
            &nodes,
            &roots,
            scene.body(),
            scene.entry_page(),
        );
        let present = present_admitted(scene.source(), &local);
        let ids = [ID_QAC_OPAQUE, ID_QAC_PCM, ID_EMF_OPAQUE, ID_EMF_ALIGN];
        let plan_entries = live_plan(scene.identity(), &ids, &local);
        let inv = Inventory::compile(scene.identity(), &plan_entries, &[]).unwrap();
        let intents = [
            SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_QAC_PCM, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_ALIGN, EntryFamily::Emf, None, None).unwrap(),
        ];
        let request = SelectionRequest::scene(&intents).unwrap();
        let sel = PrincipalSelection::new(1, request).unwrap();
        let mut mux = match HttpMux::compile(&inv, &[sel], &present) {
            Ok(m) => m,
            Err(_) => panic!("four distinct objects must compile"),
        };
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut ledger = ledger();
        let mut out = vec![0u8; 64 * 1024];
        let report = match serve_scheduled(
            &mut mux,
            &entries,
            &mut grant,
            &pins,
            || 20,
            &mut ledger,
            &mut out,
        ) {
            Ok(r) => r,
            Err(_) => panic!("four distinct objects must serve"),
        };
        assert_eq!(report.transferred, 4);
        assert_eq!(report.objects_len, 4);
        let mut ids_seen = [0u32; 4];
        let mut etags = [[0u8; 98]; 4];
        let mut roots_seen = [StrongDigest::ZERO; 4];
        for i in 0..4 {
            let object = match report.objects[i] {
                Some(o) => o,
                None => panic!("four objects"),
            };
            assert!(object.status == 206 || object.status == 200);
            ids_seen[i] = object.entry_id;
            etags[i] = object.etag;
            let sel = selected_by_id(&local, object.entry_id);
            roots_seen[i] = sel.root();
            let native = entry_slice(scene.source(), sel).unwrap();
            assert_eq!(object_body(&out, &object), native);
            let mut expect_etag = [0u8; 98];
            write_strong_etag(sel.root(), &mut expect_etag).unwrap();
            assert_eq!(&object.etag[..object.etag_len], &expect_etag[..98]);
        }
        for i in 0..4 {
            for j in 0..i {
                assert_ne!(ids_seen[i], ids_seen[j]);
                assert_ne!(etags[i], etags[j]);
                assert_ne!(roots_seen[i], roots_seen[j]);
            }
        }
        let mut saw_qac = 0u16;
        let mut saw_emf = 0u16;
        for i in 0..4 {
            match report.objects[i].map(|o| o.family) {
                Some(EntryFamily::Qac) => saw_qac += 1,
                Some(EntryFamily::Emf) => saw_emf += 1,
                _ => panic!("only QAC/EMF objects"),
            }
        }
        assert_eq!(saw_qac, 2);
        assert_eq!(saw_emf, 2);
    }

    #[test]
    fn http_q42_range_matches_authenticated_source() {
        assert!(!tls_is_pq_publisher_authority());
        let q42 = q42_volume_bytes();
        assert!(q42.len() > 256);
        let take = 128usize;
        let mut files = four_live().to_vec();
        files.push(("q42-query", "q42", q42.as_slice(), ID_Q42));
        let scene = Scene::publish(&files);
        let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
        let bound = scene.bound(&pins, pin, publisher);
        let local = select_all(&bound, &scene);
        let selected = selected_by_id(&local, ID_Q42);
        let payload = entry_slice(scene.source(), selected).unwrap();
        assert_eq!(payload, q42.as_slice());
        assert!(entry_admits_family(scene.source(), selected, EntryFamily::Q42));

        let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
        let span = match whole_span(selected, payload, &mut siblings) {
            Ok(s) => s,
            Err(_) => panic!("q42 whole-entry span"),
        };
        let whole = match AuthenticatedRange::new(selected, &span, 0, payload.len()) {
            Ok(r) => r,
            Err(_) => panic!("q42 whole-entry authenticated range"),
        };
        let ranges = [whole];
        let source = match Q42AuthenticatedRangeSource::new(
            selected,
            &ranges,
            q42.len() as u64,
            1,
        ) {
            Ok(s) => s,
            Err(_) => panic!("q42 authenticated range source"),
        };
        let volume = match Q42RangeVolume::open(source) {
            Ok(v) => v,
            Err(_) => panic!("whole-entry q42 must open"),
        };
        assert!(volume.block_count() >= 1);
        let source = volume.into_source();
        let mut native_window = vec![0u8; take];
        match source.read_exact_at(0, &mut native_window) {
            Ok(()) => {}
            Err(_) => panic!("authenticated q42 window"),
        }
        match source.read_range_into(
            Q42ByteRange {
                offset: 0,
                length: take,
            },
            &mut native_window,
        ) {
            Ok(()) => {}
            Err(_) => panic!("authenticated q42 read_range_into"),
        }
        assert_eq!(&native_window, &q42[..take]);

        let left = match AuthenticatedRange::new(selected, &span, 0, 8) {
            Ok(r) => r,
            Err(_) => panic!("q42 left span"),
        };
        let right_start = q42.len().saturating_sub(8);
        let right = match AuthenticatedRange::new(
            selected,
            &span,
            right_start as u64,
            q42.len() - right_start,
        ) {
            Ok(r) => r,
            Err(_) => panic!("q42 right span"),
        };
        let sparse_ranges = [left, right];
        let sparse = match Q42AuthenticatedRangeSource::new(
            selected,
            &sparse_ranges,
            q42.len() as u64,
            2,
        ) {
            Ok(s) => s,
            Err(_) => panic!("q42 sparse source"),
        };
        let mut untouched = [0xa5u8; 4];
        match sparse.read_exact_at(8, &mut untouched) {
            Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
            Ok(()) => panic!("gap must not be a complete q42 answer"),
        }
        assert_eq!(untouched, [0xa5u8; 4]);

        let partial = match AuthenticatedRange::new(selected, &span, 0, take) {
            Ok(r) => r,
            Err(_) => panic!("q42 partial range"),
        };
        let partial_ranges = [partial];
        let partial_source = match Q42AuthenticatedRangeSource::new(
            selected,
            &partial_ranges,
            q42.len() as u64,
            1,
        ) {
            Ok(s) => s,
            Err(_) => panic!("q42 partial source"),
        };
        match Q42RangeVolume::open(partial_source) {
            Err(_) => {}
            Ok(_) => panic!("partial header/lex window must not open as a complete volume"),
        }
        let partial_source = match Q42AuthenticatedRangeSource::new(
            selected,
            &partial_ranges,
            q42.len() as u64,
            1,
        ) {
            Ok(s) => s,
            Err(_) => panic!("q42 partial source retry"),
        };
        let mut beyond = [0xa5u8; 4];
        match partial_source.read_exact_at(take as u64, &mut beyond) {
            Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
            Ok(()) => panic!("uncovered q42 bytes must not silently complete"),
        }
        assert_eq!(beyond, [0xa5u8; 4]);

        let (nodes, root) = build_outboard(scene.source(), selected);
        let bound_entry = match bind_http_entry(
            scene.source(),
            selected,
            &nodes,
            root,
            scene.body(),
            scene.entry_page(),
        ) {
            Ok(e) => e,
            Err(_) => panic!("q42 http bind"),
        };
        let range = InclusiveRange::from_start_length(0, take as u64).unwrap();
        let mut owner = AuthorityOwner::new();
        let mut grant =
            adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut ledger = ledger();
        let mut window = vec![0u8; take];
        let mut full_out = vec![0u8; q42.len()];
        let mut served_status = 0u16;
        let mut served_len = 0usize;
        let mut served_etag = [0u8; 98];
        let mut served_etag_len = 0usize;
        let mut served_cr = [0u8; 80];
        let mut served_cr_len = 0usize;
        let mut full_status = 0u16;
        let mut full_len = 0usize;
        match grant.accept_local_scoped(&pins, || 20, |authority| {
            Ok((|| -> Result<(), ProviderError> {
                let served = bound_entry.provider.serve(
                    HttpObject::Representation,
                    Some(range),
                    None,
                    Validator::NONE,
                    ServeOptions {
                        fallback: FallbackPolicy::FORBIDDEN,
                        verified_peer: true,
                    },
                    authority,
                    &mut ledger,
                    &mut window,
                )?;
                if served.status != 206 {
                    return Err(ProviderError::Unsatisfiable);
                }
                served_status = served.status;
                served_len = served.body_len;
                served_etag = served.headers.etag;
                served_etag_len = served.headers.etag_len;
                served_cr = served.headers.content_range;
                served_cr_len = served.headers.content_range_len;
                match validate_exact_range_response(
                    Q42ByteRange {
                        offset: 0,
                        length: take,
                    },
                    q42.len() as u64,
                    range.start(),
                    served.body_len,
                ) {
                    Ok(()) => {}
                    Err(_) => return Err(ProviderError::Unsatisfiable),
                }
                let fallback = match FallbackPolicy::explicit(LEGACY_FULL_OBJECT_FALLBACK_BYTES) {
                    Ok(p) => p,
                    Err(e) => return Err(ProviderError::Status(e)),
                };
                let full = bound_entry.provider.serve(
                    HttpObject::Representation,
                    None,
                    None,
                    Validator::NONE,
                    ServeOptions {
                        fallback,
                        verified_peer: true,
                    },
                    authority,
                    &mut ledger,
                    &mut full_out,
                )?;
                if full.status != 200 {
                    return Err(ProviderError::Unsatisfiable);
                }
                full_status = full.status;
                full_len = full.body_len;
                Ok(())
            })())
        }) {
            Ok(Ok(())) => {}
            Ok(Err(_)) => panic!("q42 http range provider"),
            Err(_) => panic!("q42 http range grant"),
        }
        assert_eq!(served_status, 206);
        assert_eq!(&window[..served_len], native_window.as_slice());
        assert_eq!(full_status, 200);
        assert_eq!(&full_out[..full_len], q42.as_slice());

        let mut cr = [0u8; 80];
        let cn = write_content_range(range, q42.len() as u64, &mut cr).unwrap();
        assert_eq!(&served_cr[..served_cr_len], &cr[..cn]);
        let etag_s = std::str::from_utf8(&served_etag[..served_etag_len])
            .unwrap()
            .to_string();
        let (addr, captured, th) = spawn_http(
            served_status,
            vec![
                (
                    "Content-Range".into(),
                    String::from_utf8_lossy(&cr[..cn]).into_owned(),
                ),
                ("Content-Length".into(), take.to_string()),
                ("Content-Encoding".into(), "identity".into()),
                ("ETag".into(), etag_s),
            ],
            native_window.clone(),
        );
        let host = format!("{addr}").into_bytes();
        let req = RangeRequest {
            host: &host,
            path: b"/objects/mixed/representations/40",
            range,
            pinned_total: q42.len() as u64,
            if_range: Some(&served_etag[..served_etag_len]),
            expected: Validator {
                etag: Some(&served_etag[..served_etag_len]),
                last_modified: None,
            },
            fallback: FallbackPolicy::FORBIDDEN,
            budget: FetchBudget {
                max_body: LEGACY_FULL_OBJECT_FALLBACK_BYTES,
                max_headers: HEADER_CAP,
            },
            lig: LigGate::consented_active(),
        };
        let mut stream = connect(addr);
        let mut client_out = vec![0u8; take];
        let got = match get_range(&mut stream, &req, &mut client_out) {
            Ok(v) => v,
            Err(_) => panic!("hostile q42 loopback must return the authenticated window"),
        };
        assert_eq!(
            got,
            Fetched::Partial {
                length: take
            }
        );
        assert_eq!(&client_out[..take], native_window.as_slice());
        let text = String::from_utf8_lossy(&captured.lock().unwrap());
        assert!(text.contains("Accept-Encoding: identity"));
        assert!(text.contains("Range: bytes="));
        assert!(!text.contains("Authorization:"));
        th.join().unwrap();
    }
}
