//! Hashable collapse lineage over live original-support rows.
//!
//! Identity is SHA-256 of the canonical LE snapshot. CRC is not a signature
//! and is not used. Dead original slots are omitted (absence is Null, never
//! a 0.0 weight). Caller buffers only; FEMC is not a surface map.
use super::*;
use super::super::ddg_operators::SparseEntry;
use super::super::production_contracts::{ResourceEncoding, ResourceKind, ResourceRef};

const HEADER: usize = 160;
const ROW: usize = 56;
const ENTRY: usize = 16;
const SCHEMA: &[u8] = b"qualia.collapse-lineage/QCLN0001;le;header160;magic8=QCLN0001;u32-live,nnz;u64-committed_collapses;reserved8=0;sha256-run,source,target,policy;row56=u32-slot,support-len;u64-stable-id;domain40=u64-material,subpart,provenance,chart,protected;then-entries;entry16=u32-col,reserved0,f64-positive-weight;live-slots-only;absence-is-omission;strict-slot-order;no-trailing-bytes;identity=sha256-canonical-bytes;crc-is-not-a-signature";
const JSON_SCHEMA: &[u8] = b"qualia.collapse-lineage/QCLN0001";

pub fn lineage_snapshot_schema() -> ContentId {
    ContentId::sha256(SCHEMA)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineageIdentities {
    pub run: ContentId,
    pub source: ContentId,
    pub target: ContentId,
    pub policy: ContentId,
}
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct LineageLiveRow {
    pub slot: u32,
    pub stable_id: u64,
    pub domain: ProtectedDomain,
    pub support_start: u32,
    pub support_len: u32,
}
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct LineageSupport {
    pub col: u32,
    pub weight: f64,
}
#[derive(Debug, Clone, Copy)]
pub struct LineageSnapshot<'a> {
    pub identities: LineageIdentities,
    pub committed_collapses: u64,
    pub live: &'a [LineageLiveRow],
    pub support: &'a [LineageSupport],
}
impl LineageSnapshot<'_> {
    pub fn row_support(&self, i: usize) -> Option<&[LineageSupport]> {
        let r = self.live.get(i)?;
        let s = r.support_start as usize;
        Some(&self.support[s..s + r.support_len as usize])
    }
}

impl CollapseTransfer<'_> {
    pub fn lineage_identities(&self, target: ContentId) -> LineageIdentities {
        LineageIdentities { run: self.run(), source: self.source(), target, policy: self.policy() }
    }
}

/// Visit live original slots in index order. Empty support is omitted, not 0.0.
pub fn visit_live_lineage(
    transfer: &CollapseTransfer<'_>,
    mut visit: impl FnMut(u32, u64, ProtectedDomain, &[SparseEntry]) -> Result<(), TransferError>,
) -> Result<(usize, usize), TransferError> {
    let mut live = 0usize;
    let mut nnz = 0usize;
    let mut previous: Option<u32> = None;
    for slot in 0..transfer.original_count() {
        let support = transfer.original_support(slot).ok_or(TransferError::InvalidInput("lineage_slot"))?;
        if support.is_empty() {
            continue;
        }
        let domain = transfer.protected_domain(slot).ok_or(TransferError::InvalidInput("lineage_domain"))?;
        let stable = transfer.stable_id(slot).ok_or(TransferError::InvalidInput("lineage_stable_id"))?;
        let mut prev_col = None;
        for e in support {
            if e.row != slot as u32 {
                return Err(TransferError::InvalidInput("lineage_row"));
            }
            let col = e.col as usize;
            if col >= transfer.original_count() || !e.value.is_finite() || e.value <= 0.0 {
                return Err(TransferError::InvalidInput("lineage_weight"));
            }
            if prev_col.is_some_and(|p| p >= e.col) {
                return Err(TransferError::InvalidInput("lineage_order"));
            }
            if transfer.protected_domain(col) != Some(domain) {
                return Err(TransferError::DomainCrossing { row: slot, col });
            }
            prev_col = Some(e.col);
        }
        if previous.is_some_and(|p| p >= slot as u32) {
            return Err(TransferError::InvalidInput("lineage_slot_order"));
        }
        visit(slot as u32, stable, domain, support)?;
        live = live.checked_add(1).ok_or_else(capacity_overflow)?;
        nnz = nnz.checked_add(support.len()).ok_or_else(capacity_overflow)?;
        previous = Some(slot as u32);
    }
    if live == 0 {
        return Err(TransferError::InvalidInput("lineage_live"));
    }
    Ok((live, nnz))
}

pub fn lineage_snapshot_len(transfer: &CollapseTransfer<'_>) -> Result<usize, TransferError> {
    let (live, nnz) = visit_live_lineage(transfer, |_, _, _, _| Ok(()))?;
    if [live, nnz].iter().any(|n| *n > u32::MAX as usize) {
        return Err(TransferError::Capacity { required: live.max(nnz), available: u32::MAX as usize });
    }
    live.checked_mul(ROW)
        .and_then(|n| nnz.checked_mul(ENTRY).and_then(|e| n.checked_add(e)))
        .and_then(|n| n.checked_add(HEADER))
        .ok_or_else(capacity_overflow)
}

fn u32_at(bytes: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(bytes[p..p + 4].try_into().expect("admitted range"))
}
fn u64_at(bytes: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(bytes[p..p + 8].try_into().expect("admitted range"))
}
fn id_at(bytes: &[u8], p: usize) -> ContentId {
    ContentId { namespace: super::super::production_contracts::HashNamespace::Sha256V1, digest: bytes[p..p + 32].try_into().expect("admitted header") }
}
fn put_u32(out: &mut [u8], p: usize, v: u32) {
    out[p..p + 4].copy_from_slice(&v.to_le_bytes());
}
fn put_u64(out: &mut [u8], p: usize, v: u64) {
    out[p..p + 8].copy_from_slice(&v.to_le_bytes());
}

/// Canonical LE snapshot. `target` is the caller-bound simplified identity.
pub fn encode_lineage_snapshot(
    transfer: &CollapseTransfer<'_>,
    target: ContentId,
    output: &mut [u8],
    budget: &mut ProductionBudget<'_>,
) -> Result<ResourceRef, TransferError> {
    let n = lineage_snapshot_len(transfer)?;
    capacity(n, output.len())?;
    charge(budget, &[n, 3])?;
    let (live, nnz) = visit_live_lineage(transfer, |_, _, _, _| Ok(()))?;
    budget.checkpoint(0)?;
    output[..n].fill(0);
    output[..8].copy_from_slice(b"QCLN0001");
    put_u32(output, 8, live as u32);
    put_u32(output, 12, nnz as u32);
    put_u64(output, 16, transfer.committed_collapses());
    let ids = transfer.lineage_identities(target);
    for (i, id) in [ids.run, ids.source, ids.target, ids.policy].iter().enumerate() {
        output[32 + i * 32..64 + i * 32].copy_from_slice(&id.digest);
    }
    let mut row_p = HEADER;
    let mut entry_p = HEADER + live * ROW;
    visit_live_lineage(transfer, |slot, stable, domain, support| {
        put_u32(output, row_p, slot);
        put_u32(output, row_p + 4, support.len() as u32);
        put_u64(output, row_p + 8, stable);
        for (k, v) in [domain.material, domain.subpart, domain.provenance, domain.chart, domain.protected].into_iter().enumerate() {
            put_u64(output, row_p + 16 + k * 8, v);
        }
        row_p += ROW;
        for e in support {
            put_u32(output, entry_p, e.col);
            output[entry_p + 8..entry_p + 16].copy_from_slice(&e.value.to_le_bytes());
            entry_p += ENTRY;
        }
        Ok(())
    })?;
    Ok(ResourceRef {
        id: ContentId::sha256(&output[..n]),
        kind: ResourceKind::Evidence,
        encoding: ResourceEncoding::ExternalTyped { schema: lineage_snapshot_schema() },
        byte_len: n as u64,
        element_count: Some(live as u64),
    })
}

/// Entire output is disposable scratch on error. Publication is the returned view.
pub fn decode_lineage_snapshot<'a>(
    resource: &ResourceRef,
    bytes: &[u8],
    expected: LineageIdentities,
    live: &'a mut [LineageLiveRow],
    support: &'a mut [LineageSupport],
    budget: &mut ProductionBudget<'_>,
) -> Result<LineageSnapshot<'a>, TransferError> {
    charge(budget, &[bytes.len(), 3])?;
    if resource.kind != ResourceKind::Evidence
        || resource.encoding != (ResourceEncoding::ExternalTyped { schema: lineage_snapshot_schema() })
    {
        return Err(TransferError::Unsupported("lineage_resource_encoding"));
    }
    resource.verify(bytes).map_err(|_| TransferError::IdentityMismatch)?;
    if bytes.len() < HEADER || &bytes[..8] != b"QCLN0001" || bytes[24..32] != [0; 8] {
        return Err(TransferError::InvalidInput("lineage_header"));
    }
    let (n_live, n_nnz, sequence) = (u32_at(bytes, 8) as usize, u32_at(bytes, 12) as usize, u64_at(bytes, 16));
    let need = n_live.checked_mul(ROW)
        .and_then(|n| n_nnz.checked_mul(ENTRY).and_then(|e| n.checked_add(e)))
        .and_then(|n| n.checked_add(HEADER))
        .ok_or_else(capacity_overflow)?;
    if need != bytes.len() || n_live == 0 || resource.element_count != Some(n_live as u64) {
        return Err(TransferError::InvalidInput("lineage_length"));
    }
    capacity(n_live, live.len())?;
    capacity(n_nnz, support.len())?;
    let ids = LineageIdentities {
        run: id_at(bytes, 32),
        source: id_at(bytes, 64),
        target: id_at(bytes, 96),
        policy: id_at(bytes, 128),
    };
    if ids != expected {
        return Err(TransferError::IdentityMismatch);
    }
    let mut row_p = HEADER;
    let mut entry_p = HEADER + n_live * ROW;
    let mut filled = 0usize;
    let mut prev_slot = None;
    for i in 0..n_live {
        let slot = u32_at(bytes, row_p);
        let len = u32_at(bytes, row_p + 4) as usize;
        if len == 0 || filled.checked_add(len).ok_or_else(capacity_overflow)? > n_nnz || prev_slot.is_some_and(|p| p >= slot) {
            return Err(TransferError::InvalidInput("lineage_row"));
        }
        let domain = ProtectedDomain {
            material: u64_at(bytes, row_p + 16),
            subpart: u64_at(bytes, row_p + 24),
            provenance: u64_at(bytes, row_p + 32),
            chart: u64_at(bytes, row_p + 40),
            protected: u64_at(bytes, row_p + 48),
        };
        live[i] = LineageLiveRow {
            slot,
            stable_id: u64_at(bytes, row_p + 8),
            domain,
            support_start: filled as u32,
            support_len: len as u32,
        };
        let mut prev_col = None;
        for _ in 0..len {
            if bytes[entry_p + 4..entry_p + 8] != [0; 4] {
                return Err(TransferError::InvalidInput("lineage_entry_reserved"));
            }
            let col = u32_at(bytes, entry_p);
            let weight = f64::from_bits(u64_at(bytes, entry_p + 8));
            if !weight.is_finite() || weight <= 0.0 || prev_col.is_some_and(|p| p >= col) {
                return Err(TransferError::InvalidInput("lineage_weight"));
            }
            support[filled] = LineageSupport { col, weight };
            filled += 1;
            entry_p += ENTRY;
            prev_col = Some(col);
        }
        row_p += ROW;
        prev_slot = Some(slot);
    }
    if filled != n_nnz {
        return Err(TransferError::InvalidInput("lineage_nnz"));
    }
    budget.checkpoint(0)?;
    Ok(LineageSnapshot { identities: ids, committed_collapses: sequence, live: &live[..n_live], support: &support[..n_nnz] })
}

struct Put<'a> {
    buf: &'a mut [u8],
    n: usize,
}
impl Put<'_> {
    fn bytes(&mut self, x: &[u8]) -> Result<(), TransferError> {
        let end = self.n.checked_add(x.len()).ok_or_else(capacity_overflow)?;
        capacity(end, self.buf.len())?;
        self.buf[self.n..end].copy_from_slice(x);
        self.n = end;
        Ok(())
    }
    fn u64d(&mut self, mut v: u64) -> Result<(), TransferError> {
        if v == 0 {
            return self.bytes(b"0");
        }
        let mut tmp = [0u8; 20];
        let mut i = tmp.len();
        while v > 0 {
            i -= 1;
            tmp[i] = b'0' + (v % 10) as u8;
            v /= 10;
        }
        self.bytes(&tmp[i..])
    }
    fn hex(&mut self, x: &[u8]) -> Result<(), TransferError> {
        const D: &[u8; 16] = b"0123456789abcdef";
        let end = self.n.checked_add(x.len().checked_mul(2).ok_or_else(capacity_overflow)?).ok_or_else(capacity_overflow)?;
        capacity(end, self.buf.len())?;
        for &v in x {
            self.buf[self.n] = D[(v >> 4) as usize];
            self.buf[self.n + 1] = D[(v & 0xf) as usize];
            self.n += 2;
        }
        Ok(())
    }
}

/// Compact UTF-8 JSON, lexical object keys, schema field order. Hash the bytes;
/// do not treat CRC as a signature. Weights are IEEE-754 LE hex so absence never
/// serializes as 0.0 and bit changes are exact.
pub fn encode_lineage_snapshot_json(
    transfer: &CollapseTransfer<'_>,
    target: ContentId,
    output: &mut [u8],
    budget: &mut ProductionBudget<'_>,
) -> Result<usize, TransferError> {
    let (live, nnz) = visit_live_lineage(transfer, |_, _, _, _| Ok(()))?;
    charge(budget, &[live.checked_add(nnz).ok_or_else(capacity_overflow)?, 3])?;
    budget.checkpoint(0)?;
    let ids = transfer.lineage_identities(target);
    let mut put = Put { buf: output, n: 0 };
    put.bytes(b"{\"committed_collapses\":")?;
    put.u64d(transfer.committed_collapses())?;
    put.bytes(b",\"live\":[")?;
    let mut first = true;
    visit_live_lineage(transfer, |slot, stable, domain, support| {
        if !first {
            put.bytes(b",")?;
        }
        first = false;
        put.bytes(b"{\"domain\":{\"chart\":")?;
        put.u64d(domain.chart)?;
        put.bytes(b",\"material\":")?;
        put.u64d(domain.material)?;
        put.bytes(b",\"protected\":")?;
        put.u64d(domain.protected)?;
        put.bytes(b",\"provenance\":")?;
        put.u64d(domain.provenance)?;
        put.bytes(b",\"subpart\":")?;
        put.u64d(domain.subpart)?;
        put.bytes(b"},\"slot\":")?;
        put.u64d(slot as u64)?;
        put.bytes(b",\"stable_id\":")?;
        put.u64d(stable)?;
        put.bytes(b",\"support\":[")?;
        for (i, e) in support.iter().enumerate() {
            if i != 0 {
                put.bytes(b",")?;
            }
            put.bytes(b"{\"col\":")?;
            put.u64d(e.col as u64)?;
            put.bytes(b",\"weight_le\":\"")?;
            put.hex(&e.value.to_le_bytes())?;
            put.bytes(b"\"}")?;
        }
        put.bytes(b"]}")
    })?;
    put.bytes(b"],\"policy\":\"")?;
    put.hex(&ids.policy.digest)?;
    put.bytes(b"\",\"run\":\"")?;
    put.hex(&ids.run.digest)?;
    put.bytes(b"\",\"schema\":\"")?;
    put.bytes(JSON_SCHEMA)?;
    put.bytes(b"\",\"source\":\"")?;
    put.hex(&ids.source.digest)?;
    put.bytes(b"\",\"target\":\"")?;
    put.hex(&ids.target.digest)?;
    put.bytes(b"\"}")?;
    Ok(put.n)
}
